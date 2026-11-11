use crate::{FEND, FESC, MAX_DATA, TFEND, TFESC, types::KissEvent, types::KissError};

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
        let mut events: Vec<KissEvent> = Vec::<KissEvent>::new();
        for &b in data{
            self.offset += 1;
            match self.state{
                State::Hunting =>{
                    match b {
                        FEND => {
                            self.state = State::Body;
                            self.body.clear();
                        }
                        _ => {
                        }
                    }
                },
                State::Body => {
                    match b{
                        FEND =>{ // ending the frame 
                            if self.body.is_empty() {
                                continue;
                            }
                            let port_cmd = self.body[0]; 
                            let port = port_cmd >> 4;
                            let command = port_cmd & 0x0F;
                            let frame_data = self.body[1..].to_vec();
                            if frame_data.len() > MAX_DATA {
                                events.push(KissEvent::Error(crate::types::KissError::FrameTooLong { len: (frame_data.len()) }))
                            }
                            else{
                                let frame = crate::types::KissFrame{
                                    port,
                                    command,
                                    data : frame_data,
                                };
                                events.push(KissEvent::Frame(frame));
                            }
                            self.body.clear();
                        },
                        FESC =>{
                            self.state = State::Escape;
                        },
                        other =>{
                            if self.body.len() > (MAX_DATA + 1){
                                events.push(KissEvent::Error(crate::types::KissError::FrameTooLong { len: (self.body.len()) }));
                                self.body.clear();
                                continue;
                            }
                            self.body.push(other);
                        }
                    }
                },
                State::Discard => {
                    match b {
                        FEND => {
                            self.state = State::Body;
                            self.body.clear();
                        },
                        _ => { }
                    }
                },
                State::Escape => {
                    match b {
                        TFEND =>{
                            if self.body.len() > MAX_DATA{
                                events.push(KissEvent::Error(crate::types::KissError::FrameTooLong { len: (self.body.len()) }));
                                self.body.clear();
                                continue;
                            }
                            self.body.push(FEND);
                            self.state = State::Body;
                        },
                        TFESC => {
                            if self.body.len() > MAX_DATA{
                                events.push(KissEvent::Error(crate::types::KissError::FrameTooLong { len: (self.body.len()) }));
                                self.body.clear();
                                continue;
                            }

                            self.body.push(FESC);
                            self.state = State::Body;
                        },
                        FEND => {
                            events.push(KissEvent::Error(crate::types::KissError::InvalidEscape {
                                byte: FEND,
                                at: self.offset - 1, // invalid byte is not being considered
                            }));
                            self.state = State::Body;
                            self.body.clear();
                        }
                        other =>{ // invalid
                            events.push(KissEvent::Error(crate::types::KissError::InvalidEscape {
                                byte: other,
                                at: self.offset - 1, // invalid byte is not being considered
                            }));
                            self.body.clear();
                            self.state = State::Discard;
                        }
                    }
                }
            }
        }
        events
    }

    pub fn finish(&mut self) -> Vec<KissEvent> {
        let mut events: Vec<KissEvent> = Vec::<KissEvent>::new();
        self.reset();
        match self.state{
            State::Body => {
                if !self.body.is_empty() {
                    events.push(KissEvent::Error(KissError::TruncatedFrame { at: self.offset }));
                }
            },
            State::Escape => {
                events.push(KissEvent::Error(KissError::TruncatedFrame { at: self.offset }));
            },
            _ => {

            }
        }
        events
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

