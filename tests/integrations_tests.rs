use r3_kiss::{decode, encode, KissDecoder, KissError, KissEvent, KissFrame, FEND, FESC, MAX_DATA, TFEND, TFESC};

#[test]
fn test_roundtrip_success() {
    // 1. Create the original frame
    let original_frame = KissFrame {
        port: 5,
        command: 2,
        data: vec![0xDE, 0xAD, 0xBE, 0xEF, 0xC0, 0xDB], // Contains special bytes to test byte stuffing
    };

    // 2. Encode the frame
    let encoded = encode(&original_frame).expect("Encoding should succeed");

    // 3. Decode the frame at once using the streaming decoder
    let mut decoder = KissDecoder::new();
    let mut events = decoder.push(&encoded);
    events.append(&mut decoder.finish());
    assert_eq!(events.len(), 1);

    // 4. Verify that the decoded frame matches the original
    match &events[0] {
        KissEvent::Frame(decoded_frame) => {
            assert_eq!(decoded_frame.port, original_frame.port);
            assert_eq!(decoded_frame.command, original_frame.command);
            assert_eq!(decoded_frame.data, original_frame.data);
        }
        _ => panic!("Expected a valid KISS frame"),
    }
}

#[test]
fn test_roundtrip_stream_chunks() {
    let frame = KissFrame {
        port: 3,
        command: 1,
        data: vec![10, 20, 30, 40],
    };

    let encoded = encode(&frame).expect("Encoding should succeed");
    let mut decoder = KissDecoder::new();
    let mut all_events = Vec::new();

    // Feed the decoder one byte at a time in a loop
    for &byte in &encoded {
        let mut res = decoder.push(&[byte]);
        all_events.append(&mut res);
    }
    all_events.append(&mut decoder.finish());

    assert_eq!(all_events.len(), 1);
    match &all_events[0] {
        KissEvent::Frame(decoded_frame) => {
            assert_eq!(decoded_frame.port, 3);
            assert_eq!(decoded_frame.command, 1);
            assert_eq!(decoded_frame.data, vec![10, 20, 30, 40]);
        }
        _ => panic!("Expected a frame after streaming byte by byte"),
    }
}

#[test]
fn test_roundtrip_multiple_frames_single_stream() {
    // Encode multiple frames into a single stream and decode them all at once
    let frames = vec![
        KissFrame {
            port: 0,
            command: 0,
            data: vec![1, 2, 3],
        },
        KissFrame {
            port: 4,
            command: 1,
            data: vec![FEND, FESC, TFEND, TFESC],
        },
        KissFrame {
            port: 15,
            command: 15,
            data: vec![],
        },
    ];

    let mut stream = Vec::new();
    for f in &frames {
        stream.extend(encode(f).unwrap());
    }

    let events = decode(&stream);
    assert_eq!(events.len(), 3);
    for (event, expected_frame) in events.into_iter().zip(frames.into_iter()) {
        assert_eq!(event, KissEvent::Frame(expected_frame));
    }
}

#[test]
fn test_roundtrip_all_byte_values() {
    // Test the full range of byte values (0x00..=0xFF) in the data payload
    let all_bytes: Vec<u8> = (0u8..=255u8).collect();
    let frame = KissFrame {
        port: 7,
        command: 8,
        data: all_bytes,
    };

    let encoded = encode(&frame).unwrap();
    let events = decode(&encoded);

    assert_eq!(events, vec![KissEvent::Frame(frame)]);
}

#[test]
fn test_roundtrip_max_data_special_bytes() {
    // Full MAX_DATA payload consisting exclusively of bytes that require byte stuffing
    let data: Vec<u8> = (0..MAX_DATA)
        .map(|i| if i % 2 == 0 { FEND } else { FESC })
        .collect();
    let frame = KissFrame {
        port: 2,
        command: 10,
        data,
    };

    let encoded = encode(&frame).unwrap();
    let events = decode(&encoded);

    assert_eq!(events, vec![KissEvent::Frame(frame)]);
}

#[test]
fn test_stream_recovery_after_corrupted_frame() {
    // Stream containing a valid frame, a corrupted frame (invalid escape), and another valid frame
    let valid_1 = KissFrame {
        port: 1,
        command: 0,
        data: vec![0xAA],
    };
    let valid_2 = KissFrame {
        port: 2,
        command: 0,
        data: vec![0xBB],
    };

    let mut stream = encode(&valid_1).unwrap();
    // Append a corrupted frame
    stream.extend_from_slice(&[FEND, 0x00, FESC, 0x00, FEND]);
    // Append the second valid frame
    stream.extend(encode(&valid_2).unwrap());

    let events = decode(&stream);
    assert!(events.contains(&KissEvent::Frame(valid_1)));
    assert!(events
        .iter()
        .any(|e| matches!(e, KissEvent::Error(KissError::InvalidEscape { .. }))));
    assert!(events.contains(&KissEvent::Frame(valid_2)));
}

#[test]
fn test_roundtrip_shared_fend_delimiter() {
    // In the KISS protocol, the closing FEND of one frame can serve as the opening FEND of the next
    let frame_1 = KissFrame {
        port: 1,
        command: 1,
        data: vec![0x10, 0x20],
    };
    let frame_2 = KissFrame {
        port: 2,
        command: 2,
        data: vec![0x30, 0x40],
    };

    let mut stream = encode(&frame_1).unwrap();
    let encoded_2 = encode(&frame_2).unwrap();
    // Skip the first byte (FEND) of the second frame so it shares the delimiter with the first frame
    stream.extend_from_slice(&encoded_2[1..]);

    let events = decode(&stream);
    assert_eq!(
        events,
        vec![KissEvent::Frame(frame_1), KissEvent::Frame(frame_2)]
    );
}

#[test]
fn test_error_display_and_std_error_trait() {
    // Verify Display and std::error::Error implementations for KissError
    let errors = vec![
        KissError::DataTooLong { len: 600 },
        KissError::InvalidPort { port: 20 },
        KissError::InvalidCommand { command: 99 },
        KissError::InvalidEscape { byte: 0x00, at: 3 },
        KissError::FrameTooLong { len: 513 },
        KissError::TruncatedFrame { at: 10 },
    ];

    for err in errors {
        let msg = format!("{err}");
        assert!(!msg.is_empty());
        let std_err: &dyn std::error::Error = &err;
        assert!(std_err.source().is_none());
    }
}