# rust-kiss-framing

Byte-level KISS framing and streaming reassembly in Rust, with **zero external
dependencies** and no `unsafe` code.

This crate is standalone: it does not parse AX.25, interpret the command value,
or depend on any radio/TCP/PyO3 code. The data field is opaque bytes.

## Wire format

```text
+------+---------+------------------+------+
| FEND | command | data             | FEND |
| 0xC0 | 1 byte  | 0..=512 bytes    | 0xC0 |
+------+---------+------------------+------+
```

- A frame always has a command byte. Empty data is `C0 00 C0`.
- There is no length field, CRC or preamble.
- Maximum body before escaping: 1 command byte + 512 data bytes = 513 bytes.
- Extra leading `FEND`s are idle markers, not empty frames.
- A single `FEND` can close one frame and open the next.

### Constants

| Name     | Value  | Meaning           |
|----------|--------|-------------------|
| `FEND`   | `0xC0` | frame delimiter   |
| `FESC`   | `0xDB` | escape prefix     |
| `TFEND`  | `0xDC` | escaped `FEND`    |
| `TFESC`  | `0xDD` | escaped `FESC`    |
| `MAX_DATA` | `512` | maximum data bytes |

### Escaping

Inside the body, including the command byte:

| Raw byte    | On the wire       |
|-------------|-------------------|
| `0xC0`      | `0xDB 0xDC`       |
| `0xDB`      | `0xDB 0xDD`       |
| anything else | unchanged       |

`0xDC` and `0xDD` are ordinary data unless they follow `FESC`. A `FESC` must be
followed by exactly one of `TFEND` or `TFESC`; anything else is an invalid
escape. The opening and closing `FEND` are never escaped.

### Command byte

```text
command_byte = (port << 4) | command
port         = command_byte >> 4
command      = command_byte & 0x0F
```

`port` and `command` are each `0..=15`. This crate carries the command value
through unchanged. The command byte follows the same escaping rules as data.

## Example

```rust
use r3_kiss::{KissFrame, decode, encode};

let frame = KissFrame {
    port: 0,
    command: 0,
    data: b"ping".to_vec(),
};

let wire = encode(&frame).unwrap();
assert_eq!(wire, [0xc0, 0x00, b'p', b'i', b'n', b'g', 0xc0]);

// Available once the decoder core is implemented:
let events = decode(&wire);
assert_eq!(events.len(), 1);
```

## Public API

```rust
pub const FEND: u8 = 0xC0;
pub const FESC: u8 = 0xDB;
pub const TFEND: u8 = 0xDC;
pub const TFESC: u8 = 0xDD;
pub const MAX_DATA: usize = 512;

pub struct KissFrame {
    pub port: u8,
    pub command: u8,
    pub data: Vec<u8>,
}

pub enum KissError {
    DataTooLong { len: usize },
    InvalidPort { port: u8 },
    InvalidCommand { command: u8 },
    InvalidEscape { byte: u8, at: usize },
    FrameTooLong { len: usize },
    TruncatedFrame { at: usize },
}

pub enum KissEvent {
    Frame(KissFrame),
    Error(KissError),
}

pub fn encode(frame: &KissFrame) -> Result<Vec<u8>, KissError>;
pub fn encode_into(out: &mut Vec<u8>, frame: &KissFrame) -> Result<(), KissError>;
pub fn decode(data: &[u8]) -> Vec<KissEvent>;

pub struct KissDecoder { /* ... */ }

impl KissDecoder {
    pub fn new() -> Self;
    pub fn push(&mut self, data: &[u8]) -> Vec<KissEvent>;
    pub fn finish(&mut self) -> Vec<KissEvent>;
    pub fn reset(&mut self);
}
```

## Commands

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The project targets Rust 1.97, edition 2024.

## Test layout

| File | Purpose |
|------|---------|
| `tests/encoder_tests.rs` | Frame encoding, byte-stuffing/escaping (`FEND`, `FESC`), and validation errors (`InvalidPort`, `InvalidCommand`, `DataTooLong`) |
| `tests/decoder_tests.rs` | One-shot frame decoding, chunked streaming reassembly, and escape error handling (`InvalidEscape`) |
| `tests/integrations_tests.rs` | Full roundtrip encode-decode verification and byte-by-byte streaming chunk invariance |
