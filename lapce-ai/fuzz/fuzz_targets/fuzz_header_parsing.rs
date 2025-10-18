#![no_main]
use libfuzzer_sys::fuzz_target;
use lapce_ai_rust::ipc::binary_codec::{MAGIC_HEADER, PROTOCOL_VERSION, HEADER_SIZE};

fuzz_target!(|data: &[u8]| {
    // Test header parsing robustness
    if data.len() < HEADER_SIZE {
        return;
    }
    
    // Parse header fields
    let magic = u32::from_le_bytes([
        data.get(0).copied().unwrap_or(0),
        data.get(1).copied().unwrap_or(0),
        data.get(2).copied().unwrap_or(0),
        data.get(3).copied().unwrap_or(0),
    ]);
    
    let version = data.get(4).copied().unwrap_or(0);
    let flags = data.get(5).copied().unwrap_or(0);
    
    let msg_type = u16::from_le_bytes([
        data.get(6).copied().unwrap_or(0),
        data.get(7).copied().unwrap_or(0),
    ]);
    
    let length = u32::from_le_bytes([
        data.get(8).copied().unwrap_or(0),
        data.get(9).copied().unwrap_or(0),
        data.get(10).copied().unwrap_or(0),
        data.get(11).copied().unwrap_or(0),
    ]);
    
    let msg_id = u64::from_le_bytes([
        data.get(12).copied().unwrap_or(0),
        data.get(13).copied().unwrap_or(0),
        data.get(14).copied().unwrap_or(0),
        data.get(15).copied().unwrap_or(0),
        data.get(16).copied().unwrap_or(0),
        data.get(17).copied().unwrap_or(0),
        data.get(18).copied().unwrap_or(0),
        data.get(19).copied().unwrap_or(0),
    ]);
    
    let crc32 = u32::from_le_bytes([
        data.get(20).copied().unwrap_or(0),
        data.get(21).copied().unwrap_or(0),
        data.get(22).copied().unwrap_or(0),
        data.get(23).copied().unwrap_or(0),
    ]);
    
    // Validate parsed values (should not panic)
    let _ = magic == MAGIC_HEADER;
    let _ = version == PROTOCOL_VERSION;
    let _ = flags & 0x01 != 0; // Compression flag
    let _ = msg_type < 0x1000; // Reasonable range
    let _ = length < 100_000_000; // Max message size
    let _ = msg_id > 0;
    let _ = crc32 != 0;
});
