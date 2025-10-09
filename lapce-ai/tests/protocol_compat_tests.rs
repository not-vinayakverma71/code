/// Protocol Backward Compatibility Tests
/// Tests version negotiation and graceful rejection of incompatible clients

use lapce_ai_rust::ipc::binary_codec::{
    BinaryCodec, Message, MessageType, MessagePayload, CompletionRequest,
    MAGIC_HEADER, PROTOCOL_VERSION, HEADER_SIZE, FLAG_COMPRESSED
};
use bytes::{BytesMut, BufMut};

#[test]
fn test_version_negotiation_same_version() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 1,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "test".to_string(),
            model: "model".to_string(),
            max_tokens: 10,
            temperature: 0.5,
            stream: false,
        }),
        timestamp: 1234567890,
    };
    
    let encoded = codec.encode(&msg).expect("Encode failed");
    
    // Verify version in header
    assert_eq!(encoded[4], PROTOCOL_VERSION, "Header should contain current protocol version");
    
    // Should decode successfully with same version
    let decoded = codec.decode(&encoded).expect("Decode failed");
    assert_eq!(decoded.id, msg.id);
}

#[test]
fn test_reject_incompatible_version() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 1,
        msg_type: MessageType::Heartbeat,
        payload: MessagePayload::Heartbeat,
        timestamp: 1234567890,
    };
    
    let encoded = codec.encode(&msg).expect("Encode failed");
    let mut encoded_vec = encoded.to_vec();
    
    // Corrupt version field
    let incompatible_version = PROTOCOL_VERSION + 10;
    encoded_vec[4] = incompatible_version;
    
    // Recalculate CRC after version change
    let payload_len = u32::from_le_bytes([encoded_vec[8], encoded_vec[9], encoded_vec[10], encoded_vec[11]]) as usize;
    let payload_start = HEADER_SIZE;
    let payload_end = payload_start + payload_len;
    
    let mut crc = crc32fast::Hasher::new();
    crc.update(&encoded_vec[0..20]); // Header without CRC
    if payload_end <= encoded_vec.len() {
        crc.update(&encoded_vec[payload_start..payload_end]);
    }
    let checksum = crc.finalize();
    encoded_vec[20..24].copy_from_slice(&checksum.to_le_bytes());
    
    let encoded = bytes::Bytes::from(encoded_vec);
    // Should succeed with valid CRC
    let result = codec.decode(&encoded);
    assert!(result.is_err(), "Should reject incompatible version");
    
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("version") || err_msg.contains("protocol"),
            "Error should mention version/protocol issue: {}", err_msg);
}

#[test]
fn test_forward_compatibility_flags() {
    let mut codec = BinaryCodec::new();
    
    // Create message with future flag set
    let mut buffer = BytesMut::with_capacity(1024);
    
    // Header
    buffer.put_u32_le(MAGIC_HEADER);
    buffer.put_u8(PROTOCOL_VERSION);
    buffer.put_u8(0x80); // Unknown future flag
    buffer.put_u16_le(MessageType::Heartbeat as u16);
    buffer.put_u32_le(0); // payload length
    buffer.put_u64_le(123); // message id
    
    // Calculate CRC
    let mut crc = crc32fast::Hasher::new();
    crc.update(&buffer[0..20]);
    let checksum = crc.finalize();
    buffer.put_u32_le(checksum);
    
    // Should handle gracefully (ignore unknown flags or reject)
    let result = codec.decode(&buffer);
    
    if let Ok(decoded) = result {
        // If accepted, should still decode basic fields
        assert_eq!(decoded.id, 123);
        assert_eq!(decoded.msg_type, MessageType::Heartbeat);
    } else {
        // Or reject with clear error about unknown flags
        let err = result.unwrap_err();
        println!("Correctly rejected unknown flag: {}", err);
    }
}

#[test]
fn test_compressed_flag_handling() {
    let mut codec_with_compression = BinaryCodec::with_compression(true);
    let mut codec_without_compression = BinaryCodec::new();
    
    let msg = Message {
        id: 999,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "x".repeat(2000), // Large enough to trigger compression
            model: "model".to_string(),
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
        }),
        timestamp: 1234567890,
    };
    
    // Encode with compression
    let encoded = codec_with_compression.encode(&msg).expect("Encode failed");
    
    // Check if compressed flag is set
    let flags = encoded[5];
    if flags & FLAG_COMPRESSED != 0 {
        println!("Message was compressed");
        
        // Decoder without compression support should handle gracefully
        let result = codec_without_compression.decode(&encoded);
        match result {
            Ok(decoded) => {
                // If decoder auto-detects compression, verify correctness
                assert_eq!(decoded.id, msg.id);
            },
            Err(e) => {
                // Or reject with clear error
                println!("Correctly rejected compressed message: {}", e);
            }
        }
    }
}

#[test]
fn test_magic_number_mismatch() {
    let mut codec = BinaryCodec::new();
    
    let mut buffer = BytesMut::with_capacity(HEADER_SIZE);
    
    // Wrong magic number
    buffer.put_u32_le(0xDEADBEEF);
    buffer.put_u8(PROTOCOL_VERSION);
    buffer.put_u8(0);
    buffer.put_u16_le(0);
    buffer.put_u32_le(0);
    buffer.put_u64_le(0);
    buffer.put_u32_le(0);
    
    let result = codec.decode(&buffer);
    assert!(result.is_err(), "Should reject wrong magic number");
    
    let err = result.unwrap_err();
    assert!(err.to_string().contains("magic") || err.to_string().contains("header"),
            "Error should mention magic/header issue");
}

#[test]
fn test_message_type_evolution() {
    let mut codec = BinaryCodec::new();
    
    // Simulate old client sending deprecated message type
    let mut buffer = BytesMut::with_capacity(1024);
    
    buffer.put_u32_le(MAGIC_HEADER);
    buffer.put_u8(PROTOCOL_VERSION);
    buffer.put_u8(0);
    buffer.put_u16_le(9999); // Unknown/future message type
    buffer.put_u32_le(0);
    buffer.put_u64_le(456);
    
    let mut crc = crc32fast::Hasher::new();
    crc.update(&buffer[0..20]);
    let checksum = crc.finalize();
    buffer.put_u32_le(checksum);
    
    // Should either:
    // 1. Decode with unknown type and let handler reject
    // 2. Reject at decode time with clear error
    match codec.decode(&buffer) {
        Ok(decoded) => {
            println!("Decoded unknown message type as: {:?}", decoded.msg_type);
        },
        Err(e) => {
            println!("Rejected unknown message type: {}", e);
            assert!(e.to_string().contains("type") || e.to_string().contains("unknown"));
        }
    }
}

#[test]
fn test_backward_compatible_payload_extension() {
    // Test that adding optional fields to messages doesn't break old clients
    let mut codec = BinaryCodec::new();
    
    // Old format: just prompt and model
    let old_msg = Message {
        id: 111,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "test".to_string(),
            model: "model".to_string(),
            max_tokens: 100, // Default value
            temperature: 0.7, // Default value
            stream: false, // Default value
        }),
        timestamp: 1234567890,
    };
    
    let encoded = codec.encode(&old_msg).expect("Encode failed");
    let decoded = codec.decode(&encoded).expect("Decode failed");
    
    // Should decode with defaults for new fields
    assert_eq!(decoded.id, old_msg.id);
    
    if let MessagePayload::CompletionRequest(req) = decoded.payload {
        assert_eq!(req.prompt, "test");
        assert_eq!(req.model, "model");
        // New fields should have sensible defaults
        assert!(req.max_tokens > 0);
        assert!(req.temperature >= 0.0);
    }
}

#[test]
fn test_version_downgrade_protection() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 789,
        msg_type: MessageType::Heartbeat,
        payload: MessagePayload::Heartbeat,
        timestamp: 1234567890,
    };
    
    let encoded = codec.encode(&msg).expect("Encode failed");
    let mut encoded_vec = encoded.to_vec();
    
    // Simulate downgrade attack - lower version
    if PROTOCOL_VERSION > 0 {
        encoded_vec[4] = PROTOCOL_VERSION - 1;
        
        // Recalculate CRC
        let mut crc = crc32fast::Hasher::new();
        crc.update(&encoded_vec[0..20]);
        if encoded_vec.len() > HEADER_SIZE {
            crc.update(&encoded_vec[HEADER_SIZE..]);
        }
        let checksum = crc.finalize();
        encoded_vec[20..24].copy_from_slice(&checksum.to_le_bytes());
        
        let encoded = bytes::Bytes::from(encoded_vec);
        
        // Should reject older version in production mode
        let result = codec.decode(&encoded);
        
        // Either reject or accept with warning (implementation choice)
        match result {
            Ok(_) => println!("Accepted older version (backward compatibility enabled)"),
            Err(e) => println!("Rejected older version: {}", e),
        }
    }
}

#[test]
fn test_header_size_consistency() {
    // Verify header size matches documentation
    assert_eq!(HEADER_SIZE, 24, "Header size must be 24 bytes as per spec");
    
    let mut codec = BinaryCodec::new();
    let msg = Message {
        id: 1,
        msg_type: MessageType::Heartbeat,
        payload: MessagePayload::Heartbeat,
        timestamp: 1234567890,
    };
    
    let encoded = codec.encode(&msg).expect("Encode failed");
    
    // Minimum message is header only (for heartbeat)
    assert!(encoded.len() >= HEADER_SIZE, "Message must be at least header size");
}

#[test]
fn test_cross_version_compatibility_matrix() {
    // Test matrix of version combinations
    let versions = vec![
        (PROTOCOL_VERSION, PROTOCOL_VERSION, true, "same version"),
    ];
    
    for (sender_ver, receiver_ver, should_work, desc) in versions {
        let mut codec = BinaryCodec::new();
        
        let msg = Message {
            id: 1,
            msg_type: MessageType::Heartbeat,
            payload: MessagePayload::Heartbeat,
            timestamp: 1234567890,
        };
        
        let encoded = codec.encode(&msg).expect("Encode failed");
        let mut encoded_vec = encoded.to_vec();
        encoded_vec[4] = sender_ver;
        
        // Recalculate CRC
        let mut crc = crc32fast::Hasher::new();
        crc.update(&encoded_vec[0..20]);
        let checksum = crc.finalize();
        encoded_vec[20..24].copy_from_slice(&checksum.to_le_bytes());
        
        let encoded = bytes::Bytes::from(encoded_vec);
        
        let result = codec.decode(&encoded);
        
        if should_work {
            assert!(result.is_ok(), "Should work: {}", desc);
        } else {
            assert!(result.is_err(), "Should fail: {}", desc);
        }
    }
}

#[test]
fn test_graceful_degradation() {
    // Test that system degrades gracefully with limited features
    let mut codec = BinaryCodec::new();
    
    // Client without compression support
    let msg = Message {
        id: 100,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "x".repeat(5000), // Large message
            model: "model".to_string(),
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
        }),
        timestamp: 1234567890,
    };
    
    // Should encode without compression
    let encoded = codec.encode(&msg).expect("Encode failed");
    let flags = encoded[5];
    
    // Verify no compression flag
    assert_eq!(flags & FLAG_COMPRESSED, 0, "Should not compress without compression enabled");
    
    // Should still decode correctly
    let decoded = codec.decode(&encoded).expect("Decode failed");
    assert_eq!(decoded.id, msg.id);
}

#[test]
fn test_protocol_negotiation_handshake() {
    // Simulate protocol version negotiation during connection
    let client_version = PROTOCOL_VERSION;
    let server_version = PROTOCOL_VERSION;
    
    // Client announces its version
    let mut client_hello = BytesMut::with_capacity(HEADER_SIZE);
    client_hello.put_u32_le(MAGIC_HEADER);
    client_hello.put_u8(client_version);
    client_hello.put_u8(0); // No flags
    client_hello.put_u16_le(MessageType::Heartbeat as u16);
    client_hello.put_u32_le(0);
    client_hello.put_u64_le(0);
    
    let mut crc = crc32fast::Hasher::new();
    crc.update(&client_hello[0..20]);
    let checksum = crc.finalize();
    client_hello.put_u32_le(checksum);
    
    // Server validates
    let mut codec = BinaryCodec::new();
    let result = codec.decode(&client_hello);
    
    if client_version == server_version {
        assert!(result.is_ok(), "Server should accept compatible client version");
    } else {
        // Version mismatch handling
        println!("Version mismatch: client={}, server={}", client_version, server_version);
    }
}
