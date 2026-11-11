use crate::constants::MAX_DATA;

/// A KISS frame consisting of a port ID, a command ID, and arbitrary data payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KissFrame {
    pub port: u8,
    pub command: u8,
    pub data: Vec<u8>,
}

/// Errors that can occur during KISS encoding or decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KissError {
    DataTooLong { len: usize },
    InvalidPort { port: u8 },
    InvalidCommand { command: u8 },
    InvalidEscape { byte: u8, at: usize },
    FrameTooLong { len: usize },
    TruncatedFrame { at: usize },
}

impl std::fmt::Display for KissError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DataTooLong { len } => {
                write!(f, "data too long: {len} bytes (maximum is {MAX_DATA})")
            }
            Self::InvalidPort { port } => write!(f, "invalid port: {port} (must be 0..=15)"),
            Self::InvalidCommand { command } => {
                write!(f, "invalid command: {command} (must be 0..=15)")
            }
            Self::InvalidEscape { byte, at } => {
                write!(f, "invalid escape byte 0x{byte:02X} at offset {at}")
            }
            Self::FrameTooLong { len } => {
                write!(
                    f,
                    "frame data too long: {len} bytes (maximum is {MAX_DATA})"
                )
            }
            Self::TruncatedFrame { at } => write!(f, "truncated frame at offset {at}"),
        }
    }
}

impl std::error::Error for KissError {}

/// Events emitted by the KISS streaming decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KissEvent {
    Frame(KissFrame),
    Error(KissError),
}
