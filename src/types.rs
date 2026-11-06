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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KissEvent {
    Frame(KissFrame),
    Error(KissError),
}