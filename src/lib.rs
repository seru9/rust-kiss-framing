mod constants;
mod types;
mod encoder;
mod decoder;

// Expose constants
pub use constants::{FEND, FESC, TFEND, TFESC, MAX_DATA};

// Expose types and errors
pub use types::{KissFrame, KissError, KissEvent};

// Expose functions and decoder struct
pub use encoder::{encode};
pub use decoder::KissDecoder;