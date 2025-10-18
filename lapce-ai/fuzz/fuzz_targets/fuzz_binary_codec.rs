#![no_main]
use libfuzzer_sys::fuzz_target;
use lapce_ai_rust::ipc::binary_codec::{BinaryCodec, HEADER_SIZE};

fuzz_target!(|data: &[u8]| {
    let mut codec = BinaryCodec::new();
    
    // Try to decode arbitrary data
    // This should not panic, only return errors
    let _ = codec.decode(data);
    
    // Also test with compression enabled
    let mut codec_compressed = BinaryCodec::with_compression(true);
    let _ = codec_compressed.decode(data);
    
    // Test boundary conditions
    if data.len() >= HEADER_SIZE {
        let _ = codec.decode(&data[..HEADER_SIZE]);
    }
    
    if data.len() > HEADER_SIZE + 100 {
        let _ = codec.decode(&data[..HEADER_SIZE + 100]);
    }
});
