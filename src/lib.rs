#![forbid(unsafe_code)]

//! KISS framing and streaming reassembly library.

mod constants;
mod decoder;
mod encoder;
mod types;

// Expose constants
pub use constants::{FEND, FESC, MAX_DATA, TFEND, TFESC};

// Expose types and errors
pub use types::{KissError, KissEvent, KissFrame};

// Expose functions and decoder struct
pub use decoder::{KissDecoder, decode};
pub use encoder::encode;
