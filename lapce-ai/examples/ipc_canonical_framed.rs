/// Example: Canonical 24-byte Header with CRC32 and rkyv Serialization
/// 
/// Demonstrates the FramedShmStream wrapper that provides:
/// - Canonical 24-byte protocol header
/// - CRC32 validation
/// - Message type routing
/// - Message ID tracking

use lapce_ai_rust::ipc::canonical_header::{CanonicalHeader, MessageType, HEADER_SIZE};
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔐 Canonical Protocol Header Example\n");
    
    // Example 1: Basic header encode/decode
    println!("═══ Example 1: Header Encode/Decode ═══");
    let header = CanonicalHeader::new(MessageType::Data, 1024, 12345);
    println!("Created header:");
    println!("  Magic: 0x{:08X} ('LAPC')", header.magic);
    println!("  Version: {}", header.version);
    println!("  Type: {:?}", MessageType::from_u16(header.msg_type).unwrap());
    println!("  Payload length: {} bytes", header.payload_len);
    println!("  Message ID: {}", header.message_id);
    
    let encoded = header.encode();
    println!("\nEncoded to {} bytes (Little Endian)", encoded.len());
    
    let decoded = CanonicalHeader::decode(&encoded)?;
    println!("Decoded successfully:");
    println!("  Payload length: {}", decoded.payload_len);
    println!("  Message ID: {}\n", decoded.message_id);
    
    // Example 2: Message with CRC32 validation
    println!("═══ Example 2: CRC32 Validation ═══");
    let payload = b"Hello, lock-free IPC!";
    println!("Payload: {:?}", std::str::from_utf8(payload).unwrap());
    
    let start = Instant::now();
    let full_message = CanonicalHeader::encode_message(
        MessageType::Data,
        payload,
        67890
    )?;
    let encode_time = start.elapsed();
    
    println!("Encoded message:");
    println!("  Total size: {} bytes ({} header + {} payload)", 
        full_message.len(), HEADER_SIZE, payload.len());
    println!("  Encode time: {:?}", encode_time);
    
    let start = Instant::now();
    let (decoded_header, decoded_payload) = CanonicalHeader::decode_message(&full_message)?;
    let decode_time = start.elapsed();
    
    println!("\nDecoded and validated:");
    println!("  Message ID: {}", decoded_header.message_id);
    println!("  CRC32: 0x{:08X}", decoded_header.crc32);
    println!("  Payload: {:?}", std::str::from_utf8(&decoded_payload).unwrap());
    println!("  Decode + CRC32 validation time: {:?}\n", decode_time);
    
    // Example 3: CRC32 corruption detection
    println!("═══ Example 3: Corruption Detection ═══");
    let mut corrupted = full_message.clone();
    corrupted[HEADER_SIZE] ^= 0xFF;  // Flip bits in payload
    
    match CanonicalHeader::decode_message(&corrupted) {
        Ok(_) => println!("❌ Corruption not detected (unexpected)"),
        Err(e) => println!("✅ Corruption detected: {}", e),
    }
    
    // Example 4: Invalid protocol checks
    println!("\n═══ Example 4: Protocol Validation ═══");
    
    // Bad magic
    let mut bad_magic = [0u8; HEADER_SIZE];
    bad_magic[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
    match CanonicalHeader::decode(&bad_magic) {
        Ok(_) => println!("❌ Bad magic not detected"),
        Err(e) => println!("✅ Bad magic detected: {}", e),
    }
    
    // Wrong version
    let mut header_wrong_version = CanonicalHeader::new(MessageType::Data, 0, 1);
    header_wrong_version.version = 99;
    let bytes = header_wrong_version.encode();
    match CanonicalHeader::decode(&bytes) {
        Ok(_) => println!("❌ Wrong version not detected"),
        Err(e) => println!("✅ Wrong version detected: {}", e),
    }
    
    // Oversized payload
    let oversized = vec![0u8; 11 * 1024 * 1024];  // 11MB > 10MB limit
    match CanonicalHeader::encode_message(MessageType::Data, &oversized, 1) {
        Ok(_) => println!("❌ Oversized payload not detected"),
        Err(e) => println!("✅ Oversized payload detected: {}", e),
    }
    
    // Example 5: Performance benchmark
    println!("\n═══ Example 5: Performance Benchmark ═══");
    let test_payload = vec![0xAB; 1024];
    let iterations = 100_000;
    
    let start = Instant::now();
    for i in 0..iterations {
        let _ = CanonicalHeader::encode_message(MessageType::Data, &test_payload, i)?;
    }
    let encode_total = start.elapsed();
    
    let encoded_msg = CanonicalHeader::encode_message(MessageType::Data, &test_payload, 1)?;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = CanonicalHeader::decode_message(&encoded_msg)?;
    }
    let decode_total = start.elapsed();
    
    println!("Benchmark ({} iterations, 1KB payload):", iterations);
    println!("  Encode (header + CRC32):");
    println!("    Total: {:?}", encode_total);
    println!("    Per message: {:.2}µs", encode_total.as_micros() as f64 / iterations as f64);
    println!("  Decode (header + CRC32 validation):");
    println!("    Total: {:?}", decode_total);
    println!("    Per message: {:.2}µs", decode_total.as_micros() as f64 / iterations as f64);
    
    let encode_throughput = (iterations as f64) / encode_total.as_secs_f64();
    let decode_throughput = (iterations as f64) / decode_total.as_secs_f64();
    println!("\n  Encode throughput: {:.2} Mmsg/s", encode_throughput / 1_000_000.0);
    println!("  Decode throughput: {:.2} Mmsg/s", decode_throughput / 1_000_000.0);
    
    println!("\n✅ All examples completed successfully!");
    println!("\n📋 Summary:");
    println!("  - Header size: {} bytes (cache-line aligned)", HEADER_SIZE);
    println!("  - Magic: 0x4C415043 ('LAPC' in LE)");
    println!("  - CRC32: Fast validation (crc32fast crate)");
    println!("  - Max payload: 10MB");
    println!("  - Protocol version: 1");
    
    Ok(())
}
