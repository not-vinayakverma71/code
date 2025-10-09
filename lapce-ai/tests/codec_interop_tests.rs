/// Codec Interoperability Tests
/// Verifies that BinaryCodec and ZeroCopyCodec work correctly together
/// and handle all edge cases per the canonical spec

use lapce_ai_rust::ipc::binary_codec::{
    BinaryCodec, Message, MessageType, MessagePayload, 
    CompletionRequest, ErrorMessage,
    MAGIC_HEADER, PROTOCOL_VERSION, HEADER_SIZE, FLAG_COMPRESSED
};
use lapce_ai_rust::ipc::zero_copy_codec::ZeroCopyCodec;
use bytes::{BytesMut, Bytes};
use tokio_util::codec::{Encoder, Decoder};

fn create_test_message(id: u64) -> Message {
    Message {
        id,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "Test prompt for interop testing".to_string(),
            model: "test-model".to_string(),
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
        }),
        timestamp: 1234567890,
    }
}

#[test]
fn test_round_trip_binary_codec() {
    let mut codec = BinaryCodec::new();
    let msg = create_test_message(42);
    
    // Encode
    let encoded = codec.encode(&msg).expect("Encoding should succeed");
    assert!(encoded.len() >= HEADER_SIZE, "Message should have full header");
    
    // Decode
    let decoded = codec.decode(&encoded).expect("Decoding should succeed");
    assert_eq!(decoded.id, msg.id);
    assert_eq!(decoded.msg_type, msg.msg_type);
    assert_eq!(decoded.timestamp, msg.timestamp);
}

#[tokio::test]
async fn test_cross_codec_compatibility() {
    let msg = create_test_message(123);
    
    // Encode with BinaryCodec
    let mut binary_codec = BinaryCodec::new();
    let binary_encoded = binary_codec.encode(&msg).expect("Binary encode should succeed");
    
    // Decode with ZeroCopyCodec
    let mut zero_copy_codec = ZeroCopyCodec::new();
    let mut buffer = BytesMut::from(&binary_encoded[..]);
    let zero_decoded = zero_copy_codec.decode(&mut buffer)
        .expect("Zero-copy decode should succeed")
        .expect("Should have complete message");
    
    assert_eq!(zero_decoded.id, msg.id);
    assert_eq!(zero_decoded.msg_type, msg.msg_type);
    assert_eq!(zero_decoded.timestamp, msg.timestamp);
    
    // Encode with ZeroCopyCodec
    let mut zero_buffer = BytesMut::new();
    zero_copy_codec.encode(msg.clone(), &mut zero_buffer)
        .expect("Zero-copy encode should succeed");
    
    // Decode with BinaryCodec
    let binary_decoded = binary_codec.decode(&zero_buffer)
        .expect("Binary decode should succeed");
    
    assert_eq!(binary_decoded.id, msg.id);
    assert_eq!(binary_decoded.msg_type, msg.msg_type);
    assert_eq!(binary_decoded.timestamp, msg.timestamp);
}

#[test]
fn test_invalid_magic_rejection() {
    let mut codec = BinaryCodec::new();
    
    // Create invalid message with wrong magic
    let mut bad_data = vec![0u8; HEADER_SIZE + 10];
    bad_data[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
    bad_data[4] = PROTOCOL_VERSION;
    
    let result = codec.decode(&bad_data);
    assert!(result.is_err(), "Should reject invalid magic");
    
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("magic"), "Error should mention magic");
}

#[test]
fn test_invalid_version_rejection() {
    let mut codec = BinaryCodec::new();
    
    // Create message with wrong version
    let mut bad_data = vec![0u8; HEADER_SIZE + 10];
    bad_data[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    bad_data[4] = 99; // Invalid version
    
    let result = codec.decode(&bad_data);
    assert!(result.is_err(), "Should reject invalid version");
    
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("version"), "Error should mention version");
}

#[test]
fn test_crc32_validation() {
    let mut codec = BinaryCodec::new();
    let msg = create_test_message(789);
    
    // Encode properly
    let mut encoded = codec.encode(&msg).expect("Encoding should succeed");
    
    // Corrupt the payload (after header)
    if encoded.len() > HEADER_SIZE + 5 {
        encoded[HEADER_SIZE + 5] ^= 0xFF; // Flip bits in payload
    }
    
    // Decode should fail due to CRC mismatch
    let result = codec.decode(&encoded);
    assert!(result.is_err(), "Should reject corrupted message");
    
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("CRC") || err_msg.contains("checksum"), 
            "Error should mention CRC/checksum");
}

#[test]
fn test_message_too_large() {
    let mut codec = BinaryCodec::new();
    
    // Create header claiming huge payload
    let mut bad_data = vec![0u8; HEADER_SIZE];
    bad_data[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    bad_data[4] = PROTOCOL_VERSION;
    bad_data[5] = 0; // flags
    bad_data[6..8].copy_from_slice(&(MessageType::CompletionRequest as u16).to_le_bytes());
    bad_data[8..12].copy_from_slice(&(100_000_000u32).to_le_bytes()); // 100MB - too large
    
    let result = codec.decode(&bad_data);
    assert!(result.is_err(), "Should reject oversized message");
    
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("large") || err_msg.contains("size"), 
            "Error should mention size");
}

#[test]
fn test_incomplete_message_handling() {
    let mut codec = BinaryCodec::new();
    
    // Just a header, no payload
    let mut incomplete = vec![0u8; HEADER_SIZE];
    incomplete[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    incomplete[4] = PROTOCOL_VERSION;
    incomplete[8..12].copy_from_slice(&100u32.to_le_bytes()); // Claims 100 byte payload
    
    let result = codec.decode(&incomplete);
    assert!(result.is_err(), "Should reject incomplete message");
}

#[test]
fn test_compression_flag_handling() {
    let mut codec = BinaryCodec::with_compression(true);
    
    // Create a large message to trigger compression
    let mut large_prompt = String::new();
    for _ in 0..500 {
        large_prompt.push_str("This is a test prompt that will be compressed. ");
    }
    
    let msg = Message {
        id: 999,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: large_prompt,
            model: "test-model".to_string(),
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
        }),
        timestamp: 1234567890,
    };
    
    let encoded = codec.encode(&msg).expect("Encoding should succeed");
    
    // Check compression flag is set
    assert_eq!(encoded[5] & FLAG_COMPRESSED, FLAG_COMPRESSED, 
               "Compression flag should be set for large message");
    
    // Decode should handle compression
    let decoded = codec.decode(&encoded).expect("Decoding should succeed");
    assert_eq!(decoded.id, msg.id);
}

#[tokio::test]
async fn test_streaming_codec_interop() {
    let mut zero_codec = ZeroCopyCodec::new();
    let mut buffer = BytesMut::new();
    
    // Encode multiple messages
    let msg1 = create_test_message(1);
    let msg2 = create_test_message(2);
    let msg3 = create_test_message(3);
    
    zero_codec.encode(msg1.clone(), &mut buffer).expect("Encode 1");
    zero_codec.encode(msg2.clone(), &mut buffer).expect("Encode 2");
    zero_codec.encode(msg3.clone(), &mut buffer).expect("Encode 3");
    
    // Decode all messages
    let decoded1 = zero_codec.decode(&mut buffer)
        .expect("Decode 1 should succeed")
        .expect("Should have message 1");
    assert_eq!(decoded1.id, 1);
    
    let decoded2 = zero_codec.decode(&mut buffer)
        .expect("Decode 2 should succeed")
        .expect("Should have message 2");
    assert_eq!(decoded2.id, 2);
    
    let decoded3 = zero_codec.decode(&mut buffer)
        .expect("Decode 3 should succeed")
        .expect("Should have message 3");
    assert_eq!(decoded3.id, 3);
    
    // Buffer should be empty now
    let decoded4 = zero_codec.decode(&mut buffer)
        .expect("Decode 4 should succeed");
    assert!(decoded4.is_none(), "Should have no more messages");
}

#[test]
fn test_all_message_types() {
    let mut codec = BinaryCodec::new();
    
    let test_cases = vec![
        (MessageType::CompletionRequest, MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "test".to_string(),
            model: "model".to_string(),
            max_tokens: 10,
            temperature: 0.5,
            stream: false,
        })),
        (MessageType::Error, MessagePayload::Error(ErrorMessage {
            code: 404,
            message: "Not found".to_string(),
            details: "Details".to_string(),
        })),
        (MessageType::Heartbeat, MessagePayload::Heartbeat),
    ];
    
    for (msg_type, payload) in test_cases {
        let msg = Message {
            id: rand::random(),
            msg_type,
            payload,
            timestamp: 1234567890,
        };
        
        let encoded = codec.encode(&msg).expect("Encode should succeed");
        let decoded = codec.decode(&encoded).expect("Decode should succeed");
        
        assert_eq!(decoded.msg_type, msg.msg_type);
        assert_eq!(decoded.id, msg.id);
    }
}

#[test]
fn test_zero_length_payload() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 555,
        msg_type: MessageType::Heartbeat,
        payload: MessagePayload::Heartbeat,
        timestamp: 1234567890,
    };
    
    let encoded = codec.encode(&msg).expect("Encoding should succeed");
    let decoded = codec.decode(&encoded).expect("Decoding should succeed");
    
    assert_eq!(decoded.id, msg.id);
    assert_eq!(decoded.msg_type, MessageType::Heartbeat);
}

#[test]
fn test_max_message_boundary() {
    let mut codec = BinaryCodec::new();
    
    // Create message at exactly max size boundary
    let max_prompt_size = 10 * 1024 * 1024 - 1000; // Just under 10MB
    let large_prompt = "x".repeat(max_prompt_size);
    
    let msg = Message {
        id: 777,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: large_prompt,
            model: "test".to_string(),
            max_tokens: 1,
            temperature: 0.1,
            stream: false,
        }),
        timestamp: 1234567890,
    };
    
    // This should succeed if under limit
    let result = codec.encode(&msg);
    if result.is_err() {
        // If it fails, make sure it's due to size
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("large") || err_msg.contains("size"));
    }
}
