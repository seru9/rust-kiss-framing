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




