# NOTES

Implementation notes, decisions and remaining work for `r3-kiss`.

## Why an explicit state machine?

`Full question: Why does the decoder need explicit state instead of scanning for FEND and parsing between delimiters?`

Scanning for `FEND` and parsing each completed slice between delimiters looks
simpler, but doesn't take into account possibilities of byte `FEND` it's not only delimiter, but
also data stored in payload. Such simple approach would mean that the chunk would split in the middle of
the frame (when `FEND` is in body). 
Also the size bound of the body forces are to at least add one more state, where we're not able to accept
more bytes waiting for `FEND` to start new frame. (because last one is discarded.) 


## Chunk boundaries
`Full question: What happens if a chunk boundary splits FESC from TFEND? How do you test that?`

When a chunk boundary splits `FESC` (`0xDB`) from `TFEND` (`0xDC`):
1. `FESC` is processed as the final byte of chunk N. The decoder transitions from `State::Body` to `State::Escape`.
2. The `push` call completes cleanly and returns any frames completed before this split. The decoder does not look ahead and does not need to retain or reallocate the consumed chunk slice.
3. When chunk N+1 arrives, its first byte is `TFEND`. Because the decoder preserved `State::Escape` across `push` calls, it correctly recognizes the escape sequence, pushes unescaped `0xC0` (`FEND`) to the body buffer, and returns to `State::Body`.
4. Frame reassembly continues seamlessly without corruption or dropped frames.

**How to test this:**
- `test_decoder_split_escape_across_chunks()`: Encode a frame containing an escaped byte (`0xC0` -> `0xDB 0xDC`). Locate the index of `0xDB` and split the wire bytes into two separate slices: `chunk1` ending with `0xDB`, and `chunk2` starting with `0xDC`. Feed `chunk1` into `decoder.push()`, assert that no error or frame is emitted, feed `chunk2`, and assert that the decoded frame matches the original.

## Why an invalid escape does not drop the stream
`Full question: Why is an invalid escape not a reason to drop the whole stream? How do you resync?`

Only the malformed frame is bad, but later frames can be intact. On an invalid
escape the decoder emits one error, drops the current body, and enters
`Discard` until the next `FEND`, which is the natural resynchronization point.
If the offending byte is itself `FEND`, that byte already ends the malformed
frame, so the decoder can start the next frame immediately instead of entering
`Discard`.

## Idle `FEND`, truncated frame, oversized frame
`Full question: What is the difference between an idle FEND, a truncated frame and a frame that is too long? Why does each need a different event?`

- An idle `FEND` is a delimiter with no body. It opens/closes nothing and emits
  no event. Consecutive `FEND`s are normal line idle.
- A truncated frame has at least one body byte (or a pending escape) but no
  closing `FEND`. It is reported only by `finish`, because until then more
  bytes could still complete it.
- An oversized frame exceeded the body limit before the closing `FEND`. It is
  an immediate `FrameTooLong`, and the rest of the frame is discarded.

They need different events because the recovery action and the information
available differ.

## Decisions and traps
`Full question: What would you change for a real serial port with timeouts, a maximum frame rate and a host that can disappear mid-frame?`

For a real serial port with timeouts, a maximum frame rate and a host that can disappear mid-frame

1. **Inter-byte and Frame Timeouts:**
   - *Problem:* A host or radio can disconnect or freeze mid-frame (leaving the decoder indefinitely stuck in `State::Body` or `State::Escape`).
   - *Change:* Implement a timer reset on each incoming byte. If no byte arrives within a configurable timeout window (e.g., 50–200 ms) while assembling an unclosed frame, synthesize a timeout error, flush the partial body, and transition to `State::Hunting` (or `State::Discard`).

2. **Protection against DoS**
   - *Problem:* A valid KISS frame can be as small as 3 bytes. A malfunctioning radio, a noisy line, or a corrupted microcontroller could theoretically blast up to ~3,800 frames per second. Allocating `Vec<u8>` and emitting KissEvent::Frame can easily overwhelm bandwindth or cause out-of-memory problems.
   - *Change:* Implement a maximum frate rate (f.e at most 50 or 100 frames/sec). So that any frame arriving beyong the threshold are dropped. However it't not great solution when it comes to satellite ground segment that cannot allow itself to lose so many frames.



