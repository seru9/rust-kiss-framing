use r3_kiss::{decode, KissDecoder, KissError, KissEvent, KissFrame, FEND, FESC, MAX_DATA, TFEND, TFESC};

#[test]
fn test_decode_oneshot_success() {
    // FEND, Port/Command (0x10), Data (0x01, 0x02), FEND
    let raw_data = vec![FEND, 0x10, 0x01, 0x02, FEND];

    let events = decode(&raw_data);
    assert_eq!(events.len(), 1);

    match &events[0] {
        KissEvent::Frame(frame) => {
            assert_eq!(frame.port, 1);
            assert_eq!(frame.command, 0);
            assert_eq!(frame.data, vec![0x01, 0x02]);
        }
        _ => panic!("Expected a Frame event"),
    }
}

#[test]
fn test_decoder_chunked_and_escape() {
    let mut decoder = KissDecoder::new();

    // Split the stream into small chunks (simulating serial port data)
    // FESC followed by an invalid byte (e.g. 0xFF) -> should generate InvalidEscape
    let chunk1 = vec![FEND, 0x00, FESC, 0xFF];
    let events1 = decoder.push(&chunk1);

    assert_eq!(events1.len(), 1);
    match &events1[0] {
        KissEvent::Error(KissError::InvalidEscape { byte, .. }) => {
            assert_eq!(*byte, 0xFF);
        }
        _ => panic!("Expected an InvalidEscape error"),
    }
}

#[test]
fn test_decode_valid_escape_sequences() {
    // Verify decoding of FESC + TFEND -> FEND and FESC + TFESC -> FESC
    let raw_data = vec![FEND, 0x23, FESC, TFEND, 0x55, FESC, TFESC, FEND];
    let events = decode(&raw_data);

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        KissEvent::Frame(KissFrame {
            port: 2,
            command: 3,
            data: vec![FEND, 0x55, FESC],
        })
    );
}

#[test]
fn test_decode_ignores_multiple_fends_and_empty_frames() {
    // Consecutive FEND bytes at the start, middle, and end should be ignored
    let raw_data = vec![FEND, FEND, FEND, 0x00, 0xAA, FEND, FEND, 0x11, 0xBB, FEND, FEND];
    let events = decode(&raw_data);

    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        KissEvent::Frame(KissFrame {
            port: 0,
            command: 0,
            data: vec![0xAA],
        })
    );
    assert_eq!(
        events[1],
        KissEvent::Frame(KissFrame {
            port: 1,
            command: 1,
            data: vec![0xBB],
        })
    );
}

#[test]
fn test_decode_command_only_empty_data() {
    // Frame containing only the port/command byte with no data payload
    let raw_data = vec![FEND, 0xF5, FEND];
    let events = decode(&raw_data);

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        KissEvent::Frame(KissFrame {
            port: 15,
            command: 5,
            data: vec![],
        })
    );
}

#[test]
fn test_decode_truncated_frame_on_finish() {
    // Missing closing FEND should generate a TruncatedFrame error when finish() is called
    let mut decoder = KissDecoder::default();
    let events_push = decoder.push(&[FEND, 0x00, 0x01, 0x02]);
    assert!(events_push.is_empty());

    let events_finish = decoder.finish();
    assert_eq!(events_finish.len(), 1);
    assert!(matches!(
        events_finish[0],
        KissEvent::Error(KissError::TruncatedFrame { .. })
    ));
}

#[test]
fn test_decode_frame_too_long() {
    // Exceeding the data payload size beyond MAX_DATA (512 bytes)
    let mut raw_data = vec![FEND, 0x00];
    raw_data.extend(vec![0x42; MAX_DATA + 1]);
    raw_data.push(FEND);

    let events = decode(&raw_data);
    assert!(!events.is_empty());
    assert!(events
        .iter()
        .any(|e| matches!(e, KissEvent::Error(KissError::FrameTooLong { .. }))));
}

#[test]
fn test_decoder_reset_clears_state() {
    let mut decoder = KissDecoder::new();

    // Feed an incomplete frame and reset the decoder
    let _ = decoder.push(&vec![FEND, 0x10, 0xAA, 0xBB]);
    decoder.reset();

    // After reset, finish() should not report a TruncatedFrame error
    assert!(decoder.finish().is_empty());

    // A subsequent valid frame should decode cleanly
    let events = decoder.push(&vec![FEND, 0x20, 0xCC, FEND]);
    assert_eq!(
        events,
        vec![KissEvent::Frame(KissFrame {
            port: 2,
            command: 0,
            data: vec![0xCC],
        })]
    );
}

#[test]
fn test_decoder_split_escape_across_chunks() {
    // FESC + TFEND escape sequence split across two push() calls
    let mut decoder = KissDecoder::new();

    assert!(decoder.push(&[FEND, 0x00, FESC]).is_empty());
    let events = decoder.push(&vec![TFEND, 0x01, FEND]);

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        KissEvent::Frame(KissFrame {
            port: 0,
            command: 0,
            data: vec![FEND, 0x01],
        })
    );
}