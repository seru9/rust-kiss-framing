use crate::constants::{MAX_DATA, FEND, TFEND, FESC, TFESC};

use crate::types::{KissError, KissEvent, KissFrame};

pub fn encode(frame: &KissFrame) -> Result<Vec<u8>, KissError> {
    if frame.port > 0x0F {
        return Err(KissError::InvalidPort { port: frame.port });
    }
    if frame.command > 0x0F {
        return Err(KissError::InvalidCommand {
            command: frame.command,
        });
    }
    if frame.data.len() > MAX_DATA {
        return Err(KissError::DataTooLong {
            len: frame.data.len(),
        });
    }
    let mut encoded: Vec<u8> =  Vec::new();
    encoded.push(FEND); // FEND = u8 TYPE
    let port_cmd = (frame.port <<4) | (frame.command & 0x0F);
    encoded.push(port_cmd);
    for &item in &frame.data{
        match item{
            FEND => {
                encoded.push(FESC);
                encoded.push(TFEND);
            }
            FESC => {
                encoded.push(FESC);
                encoded.push(TFESC);
            }
            other => {
                encoded.push(other);
        }
    }}
    encoded.push(FEND);

    Ok(encoded)
}