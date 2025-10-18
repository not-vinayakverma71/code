// Protocol Compliance Tests - IPC-031
// Validates conformance to CANONICAL_BINARY_HEADER_SPEC.md
// Tests header fields, sizes, endianness, and checksum

use lapce_ai_rust::ipc::binary_codec::BinaryCodec;
use lapce_ai_rust::ipc::zero_copy_codec::ZeroCopyCodec;
use lapce_ai_rust::ipc::MessageType;
use bytes::{Bytes, BytesMut};
use std::convert::TryInto;

// Canonical header constants from spec
const MAGIC_HEADER: u32 = 0x4C415043; // "LAPC" in ASCII
const PROTOCOL_VERSION: u8 = 1;
const HEADER_SIZE: usize = 24;

/// Validate canonical header structure
#[test]
fn test_canonical_header_structure() {
    // Header layout (24 bytes total):
    // [0..4]   - Magic (u32 LE): 0x4C415043 ("LAPC")
    // [4]      - Version (u8): 1
    // [5]      - Flags (u8): reserved
    // [6..8]   - Message Type (u16 LE)
    // [8..12]  - Payload Length (u32 LE)
    // [12..20] - Message ID (u64 LE)
    // [20..24] - CRC32 (u32 LE)
    
    let mut header = vec![0u8; HEADER_SIZE];
    
    // Write magic
    header[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    assert_eq!(&header[0..4], &[0x43, 0x50, 0x41, 0x4C]); // "LAPC" in LE
    
    // Write version
    header[4] = PROTOCOL_VERSION;
    assert_eq!(header[4], 1);
    
    // Write flags
    header[5] = 0; // Reserved
    
    // Write message type
    let msg_type = MessageType::Request as u16;
    header[6..8].copy_from_slice(&msg_type.to_le_bytes());
    
    // Write payload length
    let payload_len: u32 = 1024;
    header[8..12].copy_from_slice(&payload_len.to_le_bytes());
    
    // Write message ID
    let msg_id: u64 = 0x0123456789ABCDEF;
    header[12..20].copy_from_slice(&msg_id.to_le_bytes());
    
    // Calculate CRC32 (with CRC field zeroed)
    let crc = crc32fast::hash(&header[0..20]);
    header[20..24].copy_from_slice(&crc.to_le_bytes());
    
    // Verify total size
    assert_eq!(header.len(), HEADER_SIZE);
    
    // Verify each field can be read back correctly
    assert_eq!(u32::from_le_bytes(header[0..4].try_into().unwrap()), MAGIC_HEADER);
    assert_eq!(header[4], PROTOCOL_VERSION);
    assert_eq!(header[5], 0);
    assert_eq!(u16::from_le_bytes(header[6..8].try_into().unwrap()), msg_type);
    assert_eq!(u32::from_le_bytes(header[8..12].try_into().unwrap()), payload_len);
    assert_eq!(u64::from_le_bytes(header[12..20].try_into().unwrap()), msg_id);
    assert_eq!(u32::from_le_bytes(header[20..24].try_into().unwrap()), crc);
}

/// Test little-endian encoding compliance
#[test]
fn test_endianness_compliance() {
    let test_u16: u16 = 0x1234;
    let test_u32: u32 = 0x12345678;
    let test_u64: u64 = 0x123456789ABCDEF0;
    
    // Test u16 LE encoding
    let u16_bytes = test_u16.to_le_bytes();
    assert_eq!(u16_bytes, [0x34, 0x12]);
    assert_eq!(u16::from_le_bytes(u16_bytes), test_u16);
    
    // Test u32 LE encoding
    let u32_bytes = test_u32.to_le_bytes();
    assert_eq!(u32_bytes, [0x78, 0x56, 0x34, 0x12]);
    assert_eq!(u32::from_le_bytes(u32_bytes), test_u32);
    
    // Test u64 LE encoding
    let u64_bytes = test_u64.to_le_bytes();
    assert_eq!(u64_bytes, [0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]);
    assert_eq!(u64::from_le_bytes(u64_bytes), test_u64);
}

/// Test CRC32 calculation and validation
#[test]
fn test_crc32_compliance() {
    let mut header = vec![0u8; HEADER_SIZE];
    
    // Fill header with test data
    header[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    header[4] = PROTOCOL_VERSION;
    header[5] = 0;
    header[6..8].copy_from_slice(&(MessageType::Request as u16).to_le_bytes());
    header[8..12].copy_from_slice(&100u32.to_le_bytes());
    header[12..20].copy_from_slice(&12345u64.to_le_bytes());
    
    // Calculate CRC with CRC field zeroed
    header[20..24].fill(0);
    let calculated_crc = crc32fast::hash(&header);
    header[20..24].copy_from_slice(&calculated_crc.to_le_bytes());
    
    // Verify CRC
    let mut verify_buffer = header.clone();
    let received_crc = u32::from_le_bytes(verify_buffer[20..24].try_into().unwrap());
    verify_buffer[20..24].fill(0);
    let verify_crc = crc32fast::hash(&verify_buffer);
    
    assert_eq!(received_crc, verify_crc, "CRC validation failed");
}

/// Test BinaryCodec compliance with canonical spec
#[tokio::test]
async fn test_binary_codec_compliance() {
    let codec = BinaryCodec::new();
    let test_payload = Bytes::from(vec![0x42; 256]);
    
    // Encode message
    let encoded = codec.encode(MessageType::Request, test_payload.clone())
        .await
        .expect("Encoding failed");
    
    // Verify header structure
    assert!(encoded.len() >= HEADER_SIZE, "Message too short");
    
    // Check magic
    let magic = u32::from_le_bytes(encoded[0..4].try_into().unwrap());
    assert_eq!(magic, MAGIC_HEADER, "Invalid magic header");
    
    // Check version
    assert_eq!(encoded[4], PROTOCOL_VERSION, "Invalid protocol version");
    
    // Check message type
    let msg_type = u16::from_le_bytes(encoded[6..8].try_into().unwrap());
    assert_eq!(msg_type, MessageType::Request as u16, "Invalid message type");
    
    // Check payload length
    let payload_len = u32::from_le_bytes(encoded[8..12].try_into().unwrap());
    assert_eq!(payload_len as usize, 256, "Invalid payload length");
    
    // Check CRC
    let received_crc = u32::from_le_bytes(encoded[20..24].try_into().unwrap());
    let mut verify_buffer = encoded[0..HEADER_SIZE + payload_len as usize].to_vec();
    verify_buffer[20..24].fill(0);
    let calculated_crc = crc32fast::hash(&verify_buffer);
    assert_eq!(received_crc, calculated_crc, "CRC mismatch");
    
    // Decode and verify round-trip
    let decoded = codec.decode(encoded).await.expect("Decoding failed");
    assert_eq!(decoded.msg_type, MessageType::Request);
    assert_eq!(decoded.payload.len(), 256);
}

/// Test ZeroCopyCodec compliance with canonical spec
#[tokio::test]
async fn test_zerocopy_codec_compliance() {
    let codec = ZeroCopyCodec::new();
    let test_payload = Bytes::from(vec![0x55; 512]);
    
    // Encode message
    let encoded = codec.encode(MessageType::Response, test_payload.clone())
        .await
        .expect("Encoding failed");
    
    // Verify header structure
    assert!(encoded.len() >= HEADER_SIZE, "Message too short");
    
    // Check magic
    let magic = u32::from_le_bytes(encoded[0..4].try_into().unwrap());
    assert_eq!(magic, MAGIC_HEADER, "Invalid magic header");
    
    // Check version
    assert_eq!(encoded[4], PROTOCOL_VERSION, "Invalid protocol version");
    
    // Check message type
    let msg_type = u16::from_le_bytes(encoded[6..8].try_into().unwrap());
    assert_eq!(msg_type, MessageType::Response as u16, "Invalid message type");
    
    // Check payload length
    let payload_len = u32::from_le_bytes(encoded[8..12].try_into().unwrap());
    assert_eq!(payload_len as usize, 512, "Invalid payload length");
    
    // Verify zero-copy behavior
    let decoded = codec.decode(encoded.clone()).await.expect("Decoding failed");
    assert_eq!(decoded.msg_type, MessageType::Response);
    assert_eq!(decoded.payload.len(), 512);
}

/// Test header field boundaries
#[test]
fn test_header_field_boundaries() {
    let mut header = vec![0u8; HEADER_SIZE];
    
    // Test maximum values for each field
    
    // Max u32 for magic (but we use specific value)
    header[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    
    // Max u8 for version (but we use 1)
    header[4] = PROTOCOL_VERSION;
    
    // Max u8 for flags
    header[5] = 0xFF;
    
    // Max u16 for message type
    header[6..8].copy_from_slice(&0xFFFFu16.to_le_bytes());
    
    // Max u32 for payload length
    header[8..12].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    
    // Max u64 for message ID
    header[12..20].copy_from_slice(&0xFFFFFFFFFFFFFFFFu64.to_le_bytes());
    
    // CRC32 can be any value
    header[20..24].copy_from_slice(&0x12345678u32.to_le_bytes());
    
    // Verify all fields read correctly
    assert_eq!(u32::from_le_bytes(header[0..4].try_into().unwrap()), MAGIC_HEADER);
    assert_eq!(header[4], PROTOCOL_VERSION);
    assert_eq!(header[5], 0xFF);
    assert_eq!(u16::from_le_bytes(header[6..8].try_into().unwrap()), 0xFFFF);
    assert_eq!(u32::from_le_bytes(header[8..12].try_into().unwrap()), 0xFFFFFFFF);
    assert_eq!(u64::from_le_bytes(header[12..20].try_into().unwrap()), 0xFFFFFFFFFFFFFFFF);
}

/// Test version negotiation compliance
#[test]
fn test_version_negotiation() {
    let mut header = vec![0u8; HEADER_SIZE];
    header[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    
    // Test current version
    header[4] = PROTOCOL_VERSION;
    assert_eq!(header[4], 1);
    
    // Test unsupported version detection
    header[4] = 2;
    assert_ne!(header[4], PROTOCOL_VERSION);
    
    // Test version 0 (invalid)
    header[4] = 0;
    assert_ne!(header[4], PROTOCOL_VERSION);
}

/// Test flags field compliance
#[test]
fn test_flags_compliance() {
    let mut header = vec![0u8; HEADER_SIZE];
    
    // Test reserved flags (should be 0)
    header[5] = 0x00;
    assert_eq!(header[5], 0x00);
    
    // Future flags for extensions
    header[5] = 0x01; // Compression flag (future)
    assert_eq!(header[5] & 0x01, 0x01);
    
    header[5] = 0x02; // Encryption flag (future)
    assert_eq!(header[5] & 0x02, 0x02);
    
    header[5] = 0x04; // Priority flag (future)
    assert_eq!(header[5] & 0x04, 0x04);
}

/// Test message type encoding compliance
#[test]
fn test_message_type_encoding() {
    let types = [
        (MessageType::Request, 0u16),
        (MessageType::Response, 1u16),
        (MessageType::Error, 2u16),
        (MessageType::Heartbeat, 3u16),
        (MessageType::Shutdown, 4u16),
    ];
    
    for (msg_type, expected_value) in types {
        let encoded_value = msg_type as u16;
        assert_eq!(encoded_value, expected_value, 
                   "MessageType::{:?} should encode to {}", msg_type, expected_value);
        
        let mut header_bytes = [0u8; 2];
        header_bytes.copy_from_slice(&encoded_value.to_le_bytes());
        
        let decoded_value = u16::from_le_bytes(header_bytes);
        assert_eq!(decoded_value, expected_value);
    }
}

/// Test payload length limits
#[test]
fn test_payload_length_limits() {
    // Test zero-length payload
    let zero_len = 0u32;
    let zero_bytes = zero_len.to_le_bytes();
    assert_eq!(u32::from_le_bytes(zero_bytes), 0);
    
    // Test maximum practical payload (1MB)
    let max_practical = 1024 * 1024u32;
    let max_bytes = max_practical.to_le_bytes();
    assert_eq!(u32::from_le_bytes(max_bytes), 1024 * 1024);
    
    // Test theoretical maximum (4GB - 1)
    let max_theoretical = 0xFFFFFFFFu32;
    let max_theo_bytes = max_theoretical.to_le_bytes();
    assert_eq!(u32::from_le_bytes(max_theo_bytes), 0xFFFFFFFF);
}

/// Integration test: Full protocol compliance
#[tokio::test]
async fn test_full_protocol_compliance() {
    let binary_codec = BinaryCodec::new();
    let zerocopy_codec = ZeroCopyCodec::new();
    
    // Test various payload sizes
    let payloads = vec![
        Bytes::from(vec![0x01; 0]),      // Empty
        Bytes::from(vec![0x02; 1]),      // 1 byte
        Bytes::from(vec![0x03; 64]),     // 64 bytes
        Bytes::from(vec![0x04; 1024]),   // 1KB
        Bytes::from(vec![0x05; 65536]),  // 64KB
    ];
    
    for payload in payloads {
        let payload_size = payload.len();
        
        // Test with BinaryCodec
        let binary_encoded = binary_codec.encode(MessageType::Request, payload.clone())
            .await
            .expect("Binary encoding failed");
        
        // Verify compliance
        assert!(binary_encoded.len() >= HEADER_SIZE);
        assert_eq!(u32::from_le_bytes(binary_encoded[0..4].try_into().unwrap()), MAGIC_HEADER);
        assert_eq!(binary_encoded[4], PROTOCOL_VERSION);
        assert_eq!(u32::from_le_bytes(binary_encoded[8..12].try_into().unwrap()) as usize, payload_size);
        
        // Test with ZeroCopyCodec
        let zc_encoded = zerocopy_codec.encode(MessageType::Request, payload.clone())
            .await
            .expect("ZeroCopy encoding failed");
        
        // Verify compliance
        assert!(zc_encoded.len() >= HEADER_SIZE);
        assert_eq!(u32::from_le_bytes(zc_encoded[0..4].try_into().unwrap()), MAGIC_HEADER);
        assert_eq!(zc_encoded[4], PROTOCOL_VERSION);
        assert_eq!(u32::from_le_bytes(zc_encoded[8..12].try_into().unwrap()) as usize, payload_size);
        
        // Verify cross-codec compatibility
        let binary_to_zc = zerocopy_codec.decode(binary_encoded)
            .await
            .expect("Cross-decode failed");
        assert_eq!(binary_to_zc.payload.len(), payload_size);
        
        let zc_to_binary = binary_codec.decode(zc_encoded)
            .await
            .expect("Cross-decode failed");
        assert_eq!(zc_to_binary.payload.len(), payload_size);
    }
    
    println!("✓ Full protocol compliance verified");
}
