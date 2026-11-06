use crate::types::KissEvent;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Hunting,
    Body,
    Escape,
    Discard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KissDecoder {
    state: State,
    body: Vec<u8>,
    offset: usize,
}

impl KissDecoder {
    pub fn new() -> Self {
        Self {
            state: State::Hunting,
            body: Vec::new(),
            offset: 0,
        }
    }

    pub fn push(&mut self, data: &[u8]) -> Vec<KissEvent> {
        let _ = (self, data);
        Vec::new()
    }

    pub fn finish(&mut self) -> Vec<KissEvent> {
        self.reset();
        Vec::new()
    }

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
pub fn decode(data: &[u8]) -> Vec<KissEvent> {
    let mut decoder = KissDecoder::new();
    let mut events = decoder.push(data);
    events.extend(decoder.finish());
    events
}
