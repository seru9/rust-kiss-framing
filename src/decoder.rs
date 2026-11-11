use crate::constants::{FEND, FESC, MAX_DATA, TFEND, TFESC};
use crate::types::{KissError, KissEvent, KissFrame};

/// Maximum allowed bytes in the unescaped frame body (1 command byte + 512 data bytes).
const MAX_BODY: usize = 1 + MAX_DATA;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Hunting,
    Body,
    Escape,
    Discard,
}

/// Streaming KISS protocol decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KissDecoder {
    state: State,
    body: Vec<u8>,
    offset: usize,
}

impl KissDecoder {
    /// Creates a new decoder instance initialized at offset 0.
    pub fn new() -> Self {
        Self {
            state: State::Hunting,
            body: Vec::new(),
            offset: 0,
        }
    }

    /// Feeds an arbitrary slice of bytes into the decoder and returns completed events in order.
    pub fn push(&mut self, data: &[u8]) -> Vec<KissEvent> {
        let mut events: Vec<KissEvent> = Vec::new();
        for &b in data {
            self.offset += 1;
            match self.state {
                State::Hunting => self.step_hunting(b),
                State::Body => self.step_body(b, &mut events),
                State::Escape => self.step_escape(b, &mut events),
                State::Discard => self.step_discard(b),
            }
        }
        events
    }

    /// Handles bytes when searching for the initial delimiter (startup noise).
    fn step_hunting(&mut self, b: u8) {
        if b == FEND {
            self.state = State::Body;
            self.body.clear();
        }
    }

    /// Handles bytes during frame body assembly.
    fn step_body(&mut self, b: u8, events: &mut Vec<KissEvent>) {
        match b {
            FEND => {
                if !self.body.is_empty() {
                    let port_cmd = self.body[0];
                    let port = port_cmd >> 4;
                    let command = port_cmd & 0x0F;
                    let frame_data = self.body[1..].to_vec();
                    events.push(KissEvent::Frame(KissFrame {
                        port,
                        command,
                        data: frame_data,
                    }));
                    self.body.clear();
                }
            }
            FESC => {
                self.state = State::Escape;
            }
            other => {
                self.push_body_byte(other, events);
            }
        }
    }

    /// Handles bytes following a FESC escape marker.
    fn step_escape(&mut self, b: u8, events: &mut Vec<KissEvent>) {
        match b {
            TFEND => {
                self.push_body_byte(FEND, events);
                if self.state != State::Discard {
                    self.state = State::Body;
                }
            }
            TFESC => {
                self.push_body_byte(FESC, events);
                if self.state != State::Discard {
                    self.state = State::Body;
                }
            }
            FEND => {
                events.push(KissEvent::Error(KissError::InvalidEscape {
                    byte: FEND,
                    at: self.offset - 1,
                }));
                self.body.clear();
                self.state = State::Body;
            }
            other => {
                events.push(KissEvent::Error(KissError::InvalidEscape {
                    byte: other,
                    at: self.offset - 1,
                }));
                self.body.clear();
                self.state = State::Discard;
            }
        }
    }

    /// Handles bytes when discarding an invalid or oversized frame until the next delimiter.
    fn step_discard(&mut self, b: u8) {
        if b == FEND {
            self.state = State::Body;
            self.body.clear();
        }
    }

    /// Appends a byte to the body buffer, enforcing the MAX_BODY size limit.
    fn push_body_byte(&mut self, byte: u8, events: &mut Vec<KissEvent>) {
        if self.body.len() >= MAX_BODY {
            events.push(KissEvent::Error(KissError::FrameTooLong {
                len: MAX_DATA + 1,
            }));
            self.body.clear();
            self.state = State::Discard;
        } else {
            self.body.push(byte);
        }
    }

    /// Signals the end of the current byte stream.
    pub fn finish(&mut self) -> Vec<KissEvent> {
        let mut events: Vec<KissEvent> = Vec::new();
        match self.state {
            State::Body if !self.body.is_empty() => {
                events.push(KissEvent::Error(KissError::TruncatedFrame {
                    at: self.offset,
                }));
            }
            State::Escape => {
                events.push(KissEvent::Error(KissError::TruncatedFrame {
                    at: self.offset,
                }));
            }
            _ => {}
        }
        self.reset();
        events
    }

    /// Discards all buffered state and resets the stream offset to 0 without emitting any events.
    pub fn reset(&mut self) {
        self.state = State::Hunting;
        self.body.clear();
        self.offset = 0;
    }
}

impl Default for KissDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function that decodes an entire byte slice in one shot.
pub fn decode(data: &[u8]) -> Vec<KissEvent> {
    let mut decoder = KissDecoder::new();
    let mut events = decoder.push(data);
    events.extend(decoder.finish());
    events
}
