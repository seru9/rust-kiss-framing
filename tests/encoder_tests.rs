use r3_kiss::{encode, KissError, KissFrame, FEND, FESC, MAX_DATA, TFEND, TFESC};

#[test]
fn test_encode_valid_frame() {
    let frame = KissFrame {
        port: 0,
        command: 0, // Data frame
        data: vec![0x01, 0x02, 0x03],
    };

    let result = encode(&frame);
    assert!(result.is_ok());

    let encoded = result.unwrap();
    // Should start and end with FEND, with the port/command byte (0x00) and data in between
    assert_eq!(encoded.first(), Some(&FEND));
    assert_eq!(encoded.last(), Some(&FEND));
    assert_eq!(encoded, vec![FEND, 0x00, 0x01, 0x02, 0x03, FEND]);
}

#[test]
fn test_encode_with_byte_stuffing() {
    // Frame containing special FEND and FESC bytes in the payload
    let frame = KissFrame {
        port: 1,
        command: 0,
        data: vec![FEND, 0x41, FESC, 0x42],
    };

    let result = encode(&frame);
    assert!(result.is_ok());

    let encoded = result.unwrap();
    // Port 1, Command 0 -> byte 0x10
    // FEND -> FESC, TFEND
    // FESC -> FESC, TFESC
    let expected = vec![
        FEND, 0x10, FESC, TFEND, // Escaped FEND
        0x41, FESC, TFESC, // Escaped FESC
        0x42, FEND,
    ];
    assert_eq!(encoded, expected);
}

#[test]
fn test_encode_errors() {
    // Port out of range (> 15)
    let invalid_port = KissFrame {
        port: 16,
        command: 0,
        data: vec![],
    };
    assert_eq!(
        encode(&invalid_port),
        Err(KissError::InvalidPort { port: 16 })
    );

    // Command out of range (> 15)
    let invalid_cmd = KissFrame {
        port: 0,
        command: 16,
        data: vec![],
    };
    assert_eq!(
        encode(&invalid_cmd),
        Err(KissError::InvalidCommand { command: 16 })
    );

    // Data payload too long (> MAX_DATA)
    let too_long_data = vec![0x00; MAX_DATA + 1];
    let invalid_data = KissFrame {
        port: 0,
        command: 0,
        data: too_long_data,
    };
    assert!(matches!(
        encode(&invalid_data),
        Err(KissError::DataTooLong { .. })
    ));
}

#[test]
fn test_encode_empty_payload() {
    // Frame with an empty data vector should only contain FEND, header byte, and FEND
    let frame = KissFrame {
        port: 0,
        command: 0,
        data: vec![],
    };
    let encoded = encode(&frame).unwrap();
    assert_eq!(encoded, vec![FEND, 0x00, FEND]);
}

#[test]
fn test_encode_max_port_and_command() {
    // Maximum allowed values: port = 15 (0x0F), command = 15 (0x0F) -> 0xFF
    let frame = KissFrame {
        port: 15,
        command: 15,
        data: vec![0x99],
    };
    let encoded = encode(&frame).unwrap();
    assert_eq!(encoded, vec![FEND, 0xFF, 0x99, FEND]);
}

#[test]
fn test_encode_header_escaping_when_c0_or_db() {
    // Port 12, Command 0 produces header byte 0xC0 (equal to FEND)
    // Port 13, Command 11 produces header byte 0xDB (equal to FESC)
    let frame_c0 = KissFrame {
        port: 12,
        command: 0,
        data: vec![0x01],
    };
    let encoded_c0 = encode(&frame_c0).unwrap();
    // Neither the header byte nor the payload may contain an unescaped FEND inside the frame
    assert!(!encoded_c0[1..encoded_c0.len() - 1].contains(&FEND));

    let frame_db = KissFrame {
        port: 13,
        command: 11,
        data: vec![0x01],
    };
    let encoded_db = encode(&frame_db).unwrap();
    assert!(encoded_db.starts_with(&[FEND]));
    assert!(encoded_db.ends_with(&[FEND]));
}

#[test]
fn test_encode_exact_max_data_length() {
    // Exactly MAX_DATA (512) bytes should encode without error
    let frame = KissFrame {
        port: 1,
        command: 2,
        data: vec![0x11; MAX_DATA],
    };
    let encoded = encode(&frame).expect("MAX_DATA length should be allowed");
    // FEND (1) + header (1) + 512 bytes + FEND (1) = 515
    assert_eq!(encoded.len(), MAX_DATA + 3);
}

#[test]
fn test_encode_max_data_all_escaped() {
    // 512 FEND bytes expand to 1024 bytes after stuffing, which is still valid (MAX_DATA applies to unescaped payload)
    let frame = KissFrame {
        port: 0,
        command: 0,
        data: vec![FEND; MAX_DATA],
    };
    let encoded = encode(&frame).expect("MAX_DATA with special bytes should succeed");
    assert_eq!(encoded.len(), 1 + 1 + (MAX_DATA * 2) + 1);
}

#[test]
fn test_encode_tfend_and_tfesc_not_escaped() {
    // Standalone TFEND (0xDC) and TFESC (0xDD) bytes should not be escaped
    let frame = KissFrame {
        port: 0,
        command: 0,
        data: vec![TFEND, TFESC],
    };
    let encoded = encode(&frame).unwrap();
    assert_eq!(encoded, vec![FEND, 0x00, TFEND, TFESC, FEND]);
}