# R3 — KISS framing and streaming reassembly (Rust)

**Materials:** this document, `fixtures/r3_vectors.json`

## Why this task

KISS is the small framing layer between a host and a TNC or radio. It looks simple: one delimiter byte, two escape sequences and a command byte. Then the bytes arrive from a UART in arbitrary chunks. An escape sequence can be split across two reads. A frame can arrive in three pieces with a pause in the middle. A noisy line can leave you inside a frame that never closes. The decoder has to keep enough state to survive all of that, report the bad frame, and resync at the next delimiter without losing the frames that follow.

Build a standalone crate called `r3-kiss`. Don't touch `mcs-core`, don't import anything from `mcs-engine` and don't try to integrate with `mcs_engine`. This is a byte-level framing task: the data is opaque bytes, there is no CRC, no bit-level pipeline, no TCP, no PyO3 and no radio hardware. Don't parse AX.25 and don't interpret the command value. Everything you need is in this document.

This is a self-contained task. Don't assume the repo already has a working KISS decoder. The definitions in this document are the source of truth. Generic KISS code from the internet may use different limits or error handling; don't copy it blindly.

## What to build

```text
r3-kiss/
├── Cargo.toml
├── README.md
├── NOTES.md
├── src/
│   ├── lib.rs
│   ├── frame.rs
│   ├── decoder.rs
│   └── error.rs
└── tests/
    ├── frame_vectors.rs
    ├── decoder_vectors.rs
    ├── decoder_errors.rs
    └── chunking.rs
```

A different module split is fine, as long as the public API is coherent and documented.

## Protocol

### Constants

| Name | Value | Meaning |
|------|-------|---------|
| `FEND` | `0xC0` | frame delimiter |
| `FESC` | `0xDB` | escape prefix |
| `TFEND` | `0xDC` | escaped `FEND` |
| `TFESC` | `0xDD` | escaped `FESC` |
| `MAX_DATA` | `512` | maximum data bytes per frame |

These are the only special bytes. `0xDC` and `0xDD` are ordinary data unless they follow a `FESC`.

### Wire format

```text
+------+---------+------------------+------+
| FEND | command | data             | FEND |
| 0xC0 | 1 byte  | 0..=512 bytes    | 0xC0 |
+------+---------+------------------+------+
```

- A frame always has a command byte. A frame with empty data is `C0 00 C0`.
- `data` may contain any byte value, including `0x00`, `0xC0`, `0xDB`, `0xDC` and `0xDD`, after escaping.
- There is no length field, no CRC and no preamble. Don't add one.
- Maximum frame body before escaping: 1 command byte + 512 data bytes = 513 bytes.
- A frame may be preceded by any number of extra `FEND` bytes. Consecutive `FEND`s are idle markers, not empty frames.

### Command byte

The command byte packs two four-bit fields:

```text
command_byte = (port << 4) | command
port         = command_byte >> 4
command      = command_byte & 0x0F
```

- `port` is `0..=15`.
- `command` is `0..=15`. This task does not interpret the command value; it carries it through unchanged.
- The command byte is subject to the same escaping rules as data. For example, command byte `0xC0` is sent as `DB DC`. A command byte of `0xDC` or `0xDD` is sent unchanged; only `0xC0` and `0xDB` need escaping.

### Escaping

Inside the frame body, including the command byte:

| Raw byte | On the wire |
|----------|-------------|
| `0xC0` (`FEND`) | `0xDB 0xDC` (`FESC TFEND`) |
| `0xDB` (`FESC`) | `0xDB 0xDD` (`FESC TFESC`) |
| anything else | unchanged |

- The opening and closing `FEND` are not escaped.
- `FESC` must be followed by exactly one of `TFEND` or `TFESC`. Any other following byte is an invalid escape.
- A raw `0xC0` inside the body is always a delimiter, never data. A raw `0xDB` always starts an escape. That is why data containing these values must be escaped.

### Examples

| Frame | Wire (hex) |
|-------|------------|
| port 0, command 0, empty data | `c000c0` |
| port 0, command 0, `"ping"` | `c00070696e67c0` |
| port 0, command 0, data `c0 db` | `c000dbdcdbddc0` |
| port 0, command 0, data `dc dd` | `c000dcddc0` |
| port 12, command 0, empty data | `c0dbdcc0` |
| port 13, command 11, empty data | `c0dbddc0` |
| port 13, command 12, empty data | `c0dcc0` |
| port 3, command 1, data `01 02` | `c0310102c0` |

## Public API

```rust
pub const FEND: u8 = 0xC0;
pub const FESC: u8 = 0xDB;
pub const TFEND: u8 = 0xDC;
pub const TFESC: u8 = 0xDD;
pub const MAX_DATA: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KissFrame {
    pub port: u8,
    pub command: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KissError {
    DataTooLong { len: usize },
    InvalidPort { port: u8 },
    InvalidCommand { command: u8 },
    InvalidEscape { byte: u8, at: usize },
    FrameTooLong { len: usize },
    TruncatedFrame { at: usize },
}

impl std::fmt::Display for KissError { /* ... */ }
impl std::error::Error for KissError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KissEvent {
    Frame(KissFrame),
    Error(KissError),
}

pub fn encode(frame: &KissFrame) -> Result<Vec<u8>, KissError>;

pub struct KissDecoder { /* ... */ }

impl KissDecoder {
    pub fn new() -> Self;
    pub fn push(&mut self, data: &[u8]) -> Vec<KissEvent>;
    pub fn finish(&mut self) -> Vec<KissEvent>;
    pub fn reset(&mut self);
}

impl Default for KissDecoder { /* ... */ }

pub fn decode(data: &[u8]) -> Vec<KissEvent>;
```

You may add helpers, but the items above must exist with these names and signatures.

### `encode`

- Validate `port`, then `command`, then `data.len()`. A port or command greater than 15 is an error. Data longer than 512 bytes is `DataTooLong { len }`.
- Build exactly one frame: opening `FEND`, escaped command byte, escaped data, closing `FEND`.
- Empty data is valid.
- The result contains no extra bytes before or after the frame.
- `encode` must not panic.

### `KissDecoder`

- `new()` and `Default` create an empty decoder.
- `push(data)` consumes any chunk of bytes and returns the events completed by that chunk, in order. An empty chunk returns an empty vector.
- `finish()` reports the end of the current stream, then resets the decoder (including the byte offset) so it can be reused for a new stream. Calling `finish()` on a clean decoder returns an empty vector.
- `reset()` drops all state and the offset without emitting anything.
- The decoder must not panic for any input.

### `decode`

`decode(data)` is the one-shot convenience function: create a decoder, `push` the whole slice, `finish`, and return all events in order. It is equivalent to feeding the same bytes through a decoder in one chunk.

## Decoder behaviour

The decoder works on bytes, not on complete frames. It must keep its state across `push` calls, including the state of an escape sequence that is split across two chunks.

### Delimiters and startup noise

1. Before the first `FEND`, every byte is startup noise. Ignore it. This is not an error.
2. A `FEND` ends the frame currently being assembled. If the body has at least one byte (the command byte), emit `KissEvent::Frame`. If the body is empty, emit nothing; consecutive `FEND`s are idle.
3. After a `FEND`, the decoder starts a new frame; the first decoded body byte is its command byte. A single `FEND` can close one frame and open the next one; frames do not need two delimiters between them.
4. Once the first `FEND` has been seen, a single byte between two `FEND`s is a valid frame with that command byte and empty data. The decoder cannot tell a stray byte from a short frame; only the delimiters define frames.
5. If the stream ends with an open frame that has at least one body byte or a pending escape, `finish` emits `TruncatedFrame`.
6. A frame that was already reported as `InvalidEscape` or `FrameTooLong` is not reported again as truncated at `finish`.

### Escapes

1. `FESC` sets "expecting a transposed byte". The next byte must be `TFEND` or `TFESC`.
2. `TFEND` appends `0xC0`; `TFESC` appends `0xDB`.
3. Any other byte is `InvalidEscape { byte, at }`:
   - `byte` is the byte that followed `FESC`.
   - `at` is its zero-based absolute offset in the current stream.
   - Drop the malformed frame. If the offending byte is `FEND`, that delimiter also ends the malformed frame; otherwise discard bytes until the next `FEND`.
   - Do not emit a frame for the malformed frame.
4. An escape applies to the command byte as well as to data.

### Size limit

- The body limit is 513 bytes: 1 command byte + 512 data bytes.
- When a byte would make the body 514 bytes long, emit `FrameTooLong { len: 513 }` once for that frame, drop the body and discard bytes until the next `FEND`.
- `len` is the data length at the point of failure, which is `513`; it does not include the command byte.
- Do not allocate without bound. A frame that is already too long must not keep growing in memory while the decoder waits for the closing `FEND`.

### Resynchronization

- After `InvalidEscape` (unless the offending byte was a `FEND`) or `FrameTooLong`, the decoder does not stop and does not reset the stream. It discards bytes until the next `FEND`, then continues with the next frame.
- A good frame after a bad one must still decode. This is the main reason the decoder needs a discard state instead of returning an error and losing the rest of the chunk.
- A frame that is truncated at the end of the stream is reported by `finish`; a later stream can be decoded after `finish` or `reset`.

### Event order

- `push` returns events in the order they are detected.
- A malformed or oversized frame produces exactly one error and no frame event.
- A single `push` may return several frames, or an error followed by a frame.

### Offsets and chunking

- Offsets are zero-based and count every byte consumed in the current stream, including noise, delimiters, escapes and discarded bytes.
- `push` must produce the same events regardless of how the stream is split into chunks. A chunk boundary may fall between `FESC` and `TFEND`, between a command byte and data, or between frames.
- The decoder owns its buffer; `push` borrows the input slice only for the duration of the call.
- `finish` and `reset` set the offset back to 0.

### `finish` details

- If the current frame has a command byte but no closing `FEND`, emit `TruncatedFrame { at }` where `at` is the number of bytes consumed in the current stream.
- If the current frame has only an opening `FEND` and no body bytes, it is not truncated.
- If the decoder is in discard mode after an error, `finish` does not add a second error.
- `finish` resets all state, so a later `push` starts a new stream at offset 0.

### Error fields

| Error | Field | Meaning |
|-------|-------|---------|
| `DataTooLong` | `len` | data length passed to `encode` |
| `InvalidPort` | `port` | port value greater than 15 |
| `InvalidCommand` | `command` | command value greater than 15 |
| `InvalidEscape` | `byte` | the byte that followed `FESC` |
| `InvalidEscape` | `at` | zero-based offset of that byte |
| `FrameTooLong` | `len` | data length at failure, `513` |
| `TruncatedFrame` | `at` | number of bytes consumed, one past the last byte |

`InvalidPort` and `InvalidCommand` come from `encode` (or from validating a hand-built `KissFrame`). The decoder cannot produce them, because every command byte splits into two four-bit values.

### No panic

- `push`, `finish` and `decode` must not panic for any byte sequence, including empty input, all 256 single-byte inputs, truncated escapes and frames far over the limit.
- `encode` must not panic for any `KissFrame`.
- No `unsafe`.

## Reference vectors

`fixtures/r3_vectors.json` contains:

- `constants` — the values above.
- `frames` — valid frames with `port`, `command`, `data_hex` and the expected `wire_hex`.
- `encode_errors` — invalid port and invalid command cases; each entry has the input fields and the expected `error` object.
- `streams` — byte streams with `input_hex`, optional `chunk_size`, and the exact ordered `events`. A frame event has `kind: "frame"`, `port`, `command` and `data_hex`. Error events have `kind` and the fields of the corresponding error.

The fixture is frozen. Copy it into your project if you like, but don't edit it. If a vector disagrees with this document, this document wins; tell us about the mismatch.

## Tests

At least 18. These are the ones I'd check:

### Encoding

1. `empty_data` → `c000c0`.
2. `ping` → `c00070696e67c0`.
3. `escaped_data` (`c0 db`) → `c000dbdcdbddc0`.
4. Escaped command byte: `0xC0` → `c0dbdcc0`, `0xDB` → `c0dbddc0`; command bytes `0xDC` and `0xDD` stay unchanged (`c0dcc0`, `c0ddc0`).
5. `port3_command1` → `c0310102c0`; `max_port_command` (port 15, command 15) → `c0ffc0`.
6. `DataTooLong` for 513 data bytes; 512 bytes is valid.
7. `InvalidPort` for 16, `InvalidCommand` for 16.
8. Roundtrip a deterministic set of frames covering data lengths 0, 1, 2, 255, 511 and 512, and all byte values `0x00..=0xFF`.
9. Encoded output has exactly two unescaped `FEND`s: one at the start and one at the end. Every `FESC` in the body is followed by `TFEND` or `TFESC`.

### Decoding — happy path

10. Every `frames` entry in the fixture decodes to exactly one `Frame` with the expected fields.
11. Noise before the first `FEND` is ignored.
12. Two frames separated by two `FEND`s, and two frames sharing one `FEND`, both decode in order.
13. Idle `FEND`s produce no events.
14. Empty data, a command-only frame, and port 15/command 15 all decode.
15. A frame whose data is all 256 byte values roundtrips; a 512-byte payload roundtrips.
16. Chunking invariance: feed the same stream in chunks of 1, 2, 3, 7 and 64 bytes, including empty chunks, and compare the event sequence with the one-shot result.
17. `push(&[])` returns no events; `finish` on a fresh decoder returns no events.

### Decoding — errors and resync

18. `InvalidEscape` for `FESC 0x00`: correct `byte` and `at`; a good frame after the next `FEND` still decodes. Include a case with noise before the first `FEND` so `at` counts it.
19. Invalid escape at end of stream: exactly one `InvalidEscape`, no `TruncatedFrame`.
20. Invalid escape whose offending byte is `FEND`: one `InvalidEscape`, the malformed frame ends there, and the next frame decodes.
21. 513 data bytes → one `FrameTooLong { len: 513 }`, then a good frame after the next `FEND`.
22. 512 data bytes → one valid frame, no error.
23. Truncated frame without a closing `FEND` → `TruncatedFrame { at: len }`; include a case with noise before the first `FEND` so `at` counts the noise too.
24. Truncated frame ending in `FESC` → `TruncatedFrame { at: len }`.
25. A stream ending with an idle `FEND` is not truncated.
26. `finish` resets: after a truncated frame, a complete frame can be pushed without calling `reset`; its events and offsets start from zero. A second `finish` returns no events.
27. `reset` drops a partial frame silently.
28. No panic: all 256 one-byte inputs, all 65,536 two-byte inputs (or a deterministic sample), and a pseudo-random fuzz of at least 10,000 bytes.

Tests must not need the network or external crates.

## Optional extras

Not required, but if you have time:

- `encode_into(&mut Vec<u8>, frame)` to append to a caller-owned buffer without a per-frame allocation.
- `KissDecoder::push_byte(byte) -> Option<KissEvent>` as a convenience on top of `push`; show how it relates to the chunked API.
- `KissDecoder::stats()` with counts of frames, errors and discarded bytes.
- `KissFrame::command_byte()` as the inverse of the port/command split.
- A small property test that any byte sequence can be fed to the decoder without panicking.

## Non-functional stuff

- Rust 1.97, edition 2024.
- **Zero external dependencies** — `std` is enough.
- `#![forbid(unsafe_code)]`.
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` should be clean.
- Code and comments in English (repo convention); `README`/`NOTES` can be Polish or English.
- Don't copy code from the repo or the internet. The fixture is your reference point.

## What you deliver

```text
solutions/R3/
├── Cargo.toml
├── README.md          # API, wire format, vectors, commands
├── NOTES.md           # decisions, traps, what next
├── src/
└── tests/
```

`README.md` should contain:

- the wire format and the escape table,
- the command byte layout,
- an `encode`/`decode` example,
- the test/clippy/fmt commands.

`NOTES.md` (1–2 pages):

1. Why does the decoder need explicit state instead of scanning for `FEND` and parsing between delimiters?
2. What happens if a chunk boundary splits `FESC` from `TFEND`? How do you test that?
3. Why is an invalid escape not a reason to drop the whole stream? How do you resync?
4. What is the difference between an idle `FEND`, a truncated frame and a frame that is too long? Why does each need a different event?
5. What would you change for a real serial port with timeouts, a maximum frame rate and a host that can disappear mid-frame?
