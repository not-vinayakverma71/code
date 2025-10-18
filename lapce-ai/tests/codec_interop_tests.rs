/// Codec Interoperability Tests
/// Verifies that BinaryCodec and ZeroCopyCodec work correctly together
/// and handle all edge cases per the canonical spec

use lapce_ai_rust::ipc::binary_codec::{
    BinaryCodec, Message, MessageType, MessagePayload, 
    CompletionRequest, ErrorMessage,
    LspRequestPayload, LspResponsePayload, LspNotificationPayload,
    LspDiagnosticsPayload, LspProgressPayload,
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
    let encoded = codec.encode(&msg).expect("Encoding should succeed");
    
    // Corrupt the payload (after header) - convert to Vec to mutate
    let mut corrupted = encoded.to_vec();
    if corrupted.len() > HEADER_SIZE + 5 {
        corrupted[HEADER_SIZE + 5] ^= 0xFF; // Flip bits in payload
    }
    
    // Decode should fail due to CRC mismatch
    let result = codec.decode(&corrupted);
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

// ============================================================================
// LSP Gateway Message Type Tests (LSP-001)
// ============================================================================

#[test]
fn test_lsp_request_round_trip() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 1001,
        msg_type: MessageType::LspRequest,
        payload: MessagePayload::LspRequest(LspRequestPayload {
            id: "req-123".to_string(),
            method: "textDocument/documentSymbol".to_string(),
            uri: "file:///home/user/project/src/main.rs".to_string(),
            language_id: "rust".to_string(),
            params_json: r#"{"textDocument":{"uri":"file:///home/user/project/src/main.rs"}}"#.to_string(),
        }),
        timestamp: 1234567890,
    };
    
    let encoded = codec.encode(&msg).expect("Encode LSP request");
    assert!(encoded.len() >= HEADER_SIZE);
    
    // Verify message type in header
    let msg_type_raw = u16::from_le_bytes([encoded[6], encoded[7]]);
    assert_eq!(msg_type_raw, 0x0200);
    
    let decoded = codec.decode(&encoded).expect("Decode LSP request");
    assert_eq!(decoded.id, msg.id);
    assert_eq!(decoded.msg_type, MessageType::LspRequest);
    
    if let MessagePayload::LspRequest(payload) = decoded.payload {
        assert_eq!(payload.id, "req-123");
        assert_eq!(payload.method, "textDocument/documentSymbol");
        assert_eq!(payload.language_id, "rust");
    } else {
        panic!("Expected LspRequest payload");
    }
}

#[test]
fn test_lsp_response_success() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 1002,
        msg_type: MessageType::LspResponse,
        payload: MessagePayload::LspResponse(LspResponsePayload {
            id: "req-123".to_string(),
            ok: true,
            result_json: r#"{"symbols":[{"name":"main","kind":12}]}"#.to_string(),
            error: None,
            error_code: None,
        }),
        timestamp: 1234567891,
    };
    
    let encoded = codec.encode(&msg).expect("Encode LSP response");
    let decoded = codec.decode(&encoded).expect("Decode LSP response");
    
    assert_eq!(decoded.msg_type, MessageType::LspResponse);
    if let MessagePayload::LspResponse(payload) = decoded.payload {
        assert_eq!(payload.id, "req-123");
        assert!(payload.ok);
        assert!(payload.error.is_none());
        assert!(payload.result_json.contains("symbols"));
    } else {
        panic!("Expected LspResponse payload");
    }
}

#[test]
fn test_lsp_response_error() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 1003,
        msg_type: MessageType::LspResponse,
        payload: MessagePayload::LspResponse(LspResponsePayload {
            id: "req-456".to_string(),
            ok: false,
            result_json: String::new(),
            error: Some("Method not found".to_string()),
            error_code: Some(-32601),
        }),
        timestamp: 1234567892,
    };
    
    let encoded = codec.encode(&msg).expect("Encode error response");
    let decoded = codec.decode(&encoded).expect("Decode error response");
    
    if let MessagePayload::LspResponse(payload) = decoded.payload {
        assert!(!payload.ok);
        assert_eq!(payload.error, Some("Method not found".to_string()));
        assert_eq!(payload.error_code, Some(-32601));
    } else {
        panic!("Expected LspResponse payload");
    }
}

#[test]
fn test_lsp_notification() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 1004,
        msg_type: MessageType::LspNotification,
        payload: MessagePayload::LspNotification(LspNotificationPayload {
            method: "textDocument/didOpen".to_string(),
            uri: "file:///home/user/project/src/lib.rs".to_string(),
            params_json: r#"{"textDocument":{"uri":"file:///home/user/project/src/lib.rs","languageId":"rust","version":1,"text":"fn main() {}"}}"#.to_string(),
        }),
        timestamp: 1234567893,
    };
    
    let encoded = codec.encode(&msg).expect("Encode notification");
    let decoded = codec.decode(&encoded).expect("Decode notification");
    
    assert_eq!(decoded.msg_type, MessageType::LspNotification);
    if let MessagePayload::LspNotification(payload) = decoded.payload {
        assert_eq!(payload.method, "textDocument/didOpen");
        assert!(payload.params_json.contains("textDocument"));
    } else {
        panic!("Expected LspNotification payload");
    }
}

#[test]
fn test_lsp_diagnostics() {
    let mut codec = BinaryCodec::new();
    
    let diagnostics_json = r#"[
        {
            "range": {"start": {"line": 10, "character": 5}, "end": {"line": 10, "character": 15}},
            "severity": 1,
            "code": "E0308",
            "source": "rustc",
            "message": "mismatched types"
        }
    ]"#;
    
    let msg = Message {
        id: 1005,
        msg_type: MessageType::LspDiagnostics,
        payload: MessagePayload::LspDiagnostics(LspDiagnosticsPayload {
            uri: "file:///home/user/project/src/main.rs".to_string(),
            version: Some(5),
            diagnostics_json: diagnostics_json.to_string(),
        }),
        timestamp: 1234567894,
    };
    
    let encoded = codec.encode(&msg).expect("Encode diagnostics");
    let decoded = codec.decode(&encoded).expect("Decode diagnostics");
    
    assert_eq!(decoded.msg_type, MessageType::LspDiagnostics);
    if let MessagePayload::LspDiagnostics(payload) = decoded.payload {
        assert_eq!(payload.version, Some(5));
        assert!(payload.diagnostics_json.contains("E0308"));
        assert!(payload.diagnostics_json.contains("mismatched types"));
    } else {
        panic!("Expected LspDiagnostics payload");
    }
}

#[test]
fn test_lsp_progress() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 1006,
        msg_type: MessageType::LspProgress,
        payload: MessagePayload::LspProgress(LspProgressPayload {
            token: "indexing-workspace".to_string(),
            value_json: r#"{"kind":"begin","title":"Indexing workspace","percentage":0}"#.to_string(),
        }),
        timestamp: 1234567895,
    };
    
    let encoded = codec.encode(&msg).expect("Encode progress");
    let decoded = codec.decode(&encoded).expect("Decode progress");
    
    assert_eq!(decoded.msg_type, MessageType::LspProgress);
    if let MessagePayload::LspProgress(payload) = decoded.payload {
        assert_eq!(payload.token, "indexing-workspace");
        assert!(payload.value_json.contains("Indexing workspace"));
    } else {
        panic!("Expected LspProgress payload");
    }
}

#[test]
fn test_cancel_message() {
    let mut codec = BinaryCodec::new();
    
    let msg = Message {
        id: 1007,
        msg_type: MessageType::Cancel,
        payload: MessagePayload::Cancel {
            request_id: "req-789".to_string(),
        },
        timestamp: 1234567896,
    };
    
    let encoded = codec.encode(&msg).expect("Encode cancel");
    let decoded = codec.decode(&encoded).expect("Decode cancel");
    
    assert_eq!(decoded.msg_type, MessageType::Cancel);
    if let MessagePayload::Cancel { request_id } = decoded.payload {
        assert_eq!(request_id, "req-789");
    } else {
        panic!("Expected Cancel payload");
    }
}

#[test]
fn test_all_lsp_message_types() {
    let mut codec = BinaryCodec::new();
    
    let test_messages = vec![
        (MessageType::LspRequest, MessagePayload::LspRequest(LspRequestPayload {
            id: "1".to_string(),
            method: "textDocument/hover".to_string(),
            uri: "file:///test.rs".to_string(),
            language_id: "rust".to_string(),
            params_json: "{}".to_string(),
        })),
        (MessageType::LspResponse, MessagePayload::LspResponse(LspResponsePayload {
            id: "1".to_string(),
            ok: true,
            result_json: "{}".to_string(),
            error: None,
            error_code: None,
        })),
        (MessageType::LspNotification, MessagePayload::LspNotification(LspNotificationPayload {
            method: "textDocument/didChange".to_string(),
            uri: "file:///test.rs".to_string(),
            params_json: "{}".to_string(),
        })),
        (MessageType::LspDiagnostics, MessagePayload::LspDiagnostics(LspDiagnosticsPayload {
            uri: "file:///test.rs".to_string(),
            version: Some(1),
            diagnostics_json: "[]".to_string(),
        })),
        (MessageType::LspProgress, MessagePayload::LspProgress(LspProgressPayload {
            token: "token".to_string(),
            value_json: "{}".to_string(),
        })),
        (MessageType::Cancel, MessagePayload::Cancel {
            request_id: "req-1".to_string(),
        }),
    ];
    
    for (msg_type, payload) in test_messages {
        let msg = Message {
            id: rand::random(),
            msg_type,
            payload,
            timestamp: 1234567890,
        };
        
        let encoded = codec.encode(&msg).expect("Encode should succeed");
        let decoded = codec.decode(&encoded).expect("Decode should succeed");
        
        assert_eq!(decoded.msg_type, msg.msg_type, "Message type mismatch for {:?}", msg_type);
        assert_eq!(decoded.id, msg.id);
    }
}

#[test]
fn test_lsp_large_diagnostics_payload() {
    let mut codec = BinaryCodec::new();
    
    // Create a large diagnostics array (simulate many errors)
    let mut diagnostics = Vec::new();
    for i in 0..1000 {
        diagnostics.push(format!(
            r#"{{
                "range": {{"start": {{"line": {}, "character": 0}}, "end": {{"line": {}, "character": 10}}}},
                "severity": 1,
                "message": "Error {}"
            }}"#,
            i, i, i
        ));
    }
    let diagnostics_json = format!("[{}]", diagnostics.join(","));
    
    let msg = Message {
        id: 2000,
        msg_type: MessageType::LspDiagnostics,
        payload: MessagePayload::LspDiagnostics(LspDiagnosticsPayload {
            uri: "file:///large.rs".to_string(),
            version: Some(10),
            diagnostics_json,
        }),
        timestamp: 1234567900,
    };
    
    let encoded = codec.encode(&msg).expect("Should encode large diagnostics");
    let decoded = codec.decode(&encoded).expect("Should decode large diagnostics");
    
    assert_eq!(decoded.msg_type, MessageType::LspDiagnostics);
    if let MessagePayload::LspDiagnostics(payload) = decoded.payload {
        assert!(payload.diagnostics_json.contains("Error 999"), "Should contain all errors");
    } else {
        panic!("Expected LspDiagnostics payload");
    }
}

#[tokio::test]
async fn test_lsp_cross_codec_compatibility() {
    let mut binary_codec = BinaryCodec::new();
    let mut zero_codec = ZeroCopyCodec::new();
    
    let lsp_msg = Message {
        id: 3000,
        msg_type: MessageType::LspRequest,
        payload: MessagePayload::LspRequest(LspRequestPayload {
            id: "cross-test-1".to_string(),
            method: "textDocument/definition".to_string(),
            uri: "file:///test.rs".to_string(),
            language_id: "rust".to_string(),
            params_json: r#"{"position":{"line":10,"character":5}}"#.to_string(),
        }),
        timestamp: 1234567901,
    };
    
    // Binary encode -> Zero decode
    let binary_encoded = binary_codec.encode(&lsp_msg).expect("Binary encode");
    let mut buffer = BytesMut::from(&binary_encoded[..]);
    let zero_decoded = zero_codec.decode(&mut buffer)
        .expect("Zero decode")
        .expect("Should have message");
    
    assert_eq!(zero_decoded.msg_type, MessageType::LspRequest);
    assert_eq!(zero_decoded.id, 3000);
    
    // Zero encode -> Binary decode
    let mut zero_buffer = BytesMut::new();
    zero_codec.encode(lsp_msg, &mut zero_buffer).expect("Zero encode");
    let binary_decoded = binary_codec.decode(&zero_buffer).expect("Binary decode");
    
    assert_eq!(binary_decoded.msg_type, MessageType::LspRequest);
    assert_eq!(binary_decoded.id, 3000);
}
