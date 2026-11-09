use crate::constants::{MAX_DATA, FEND, TFEND, FESC, TFESC};

use crate::types::{KissError, KissEvent, KissFrame};

pub fn encode(frame: &KissFrame) -> Result<Vec<u8>, KissError> {
    if frame.port > 15 {
        return Err(KissError::InvalidPort { port: frame.port });
    }
    if frame.command > 15 {
        return Err(KissError::InvalidCommand { command: frame.command });
    }
    if frame.data.len() > MAX_DATA {
        return Err(KissError::DataTooLong { len: frame.data.len() });
    }

    let mut encoded = Vec::with_capacity(2 + frame.data.len() * 2);

    encoded.push(FEND);

    let command_byte = (frame.port << 4) | (frame.command & 0x0F);
    push_escaped(command_byte, &mut encoded);

    for &b in &frame.data {
        push_escaped(b, &mut encoded);
    }

    encoded.push(FEND);

    Ok(encoded)
}
fn push_escaped(byte: u8, dest: &mut Vec<u8>) {
    match byte {
        FEND => {
            dest.push(FESC);
            dest.push(TFEND);
        }
        FESC => {
            dest.push(FESC);
            dest.push(TFESC);
        }
        other => {
            dest.push(other);
        }
    }
}