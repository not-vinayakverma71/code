// Create proper binary corpus seeds for fuzzing
use std::fs;

const MAGIC_HEADER: u32 = 0x4C415043;  // "LAPC"
const HEADER_SIZE: usize = 24;

fn create_valid_header(payload_len: u32, msg_id: u64) -> Vec<u8> {
    let mut header = vec![0u8; HEADER_SIZE];
    header[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());  // Magic
    header[4] = 1;  // Version
    header[5] = 0;  // Flags
    header[6..8].copy_from_slice(&1u16.to_le_bytes());  // Type (Heartbeat)
    header[8..12].copy_from_slice(&payload_len.to_le_bytes());  // Length
    header[12..20].copy_from_slice(&msg_id.to_le_bytes());  // Message ID
    
    // Calculate CRC32
    let mut full_msg = Vec::with_capacity(HEADER_SIZE + payload_len as usize);
    full_msg.extend_from_slice(&header);
    full_msg.resize(HEADER_SIZE + payload_len as usize, 0);
    let crc = crc32fast::hash(&full_msg);
    header[20..24].copy_from_slice(&crc.to_le_bytes());
    
    full_msg[0..HEADER_SIZE].copy_from_slice(&header);
    full_msg
}

fn main() {
    fs::create_dir_all("fuzz/corpus").expect("Failed to create corpus directory");
    
    // Seed 1: Valid empty message (heartbeat)
    let seed1 = create_valid_header(0, 12345);
    fs::write("fuzz/corpus/valid_empty", &seed1).expect("Failed to write seed1");
    
    // Seed 2: Valid message with small payload
    let seed2 = create_valid_header(10, 67890);
    fs::write("fuzz/corpus/valid_small", &seed2).expect("Failed to write seed2");
    
    // Seed 3: Header only (truncated)
    let mut header_only = vec![0u8; HEADER_SIZE];
    header_only[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    header_only[4] = 1;
    header_only[8..12].copy_from_slice(&100u32.to_le_bytes()); // Claims 100 bytes but no payload
    fs::write("fuzz/corpus/truncated", &header_only).expect("Failed to write seed3");
    
    // Seed 4: Invalid magic
    let mut bad_magic = create_valid_header(0, 11111);
    bad_magic[0..4].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
    fs::write("fuzz/corpus/bad_magic", &bad_magic).expect("Failed to write seed4");
    
    // Seed 5: Invalid version
    let mut bad_version = create_valid_header(0, 22222);
    bad_version[4] = 99;
    fs::write("fuzz/corpus/bad_version", &bad_version).expect("Failed to write seed5");
    
    // Seed 6: Oversized message claim
    let mut oversized = vec![0u8; HEADER_SIZE];
    oversized[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
    oversized[4] = 1;
    oversized[8..12].copy_from_slice(&(100_000_000u32).to_le_bytes()); // Claims 100MB
    fs::write("fuzz/corpus/oversized", &oversized).expect("Failed to write seed6");
    
    // Seed 7: Boundary size at exactly 23 bytes
    let boundary23 = vec![0u8; 23];
    fs::write("fuzz/corpus/boundary_23", &boundary23).expect("Failed to write seed7");
    
    // Seed 8: Boundary size at exactly 25 bytes  
    let boundary25 = vec![0u8; 25];
    fs::write("fuzz/corpus/boundary_25", &boundary25).expect("Failed to write seed8");
    
    println!("Created 8 corpus seeds for fuzzing");
}
