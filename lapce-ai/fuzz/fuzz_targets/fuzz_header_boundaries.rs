#![no_main]
use libfuzzer_sys::fuzz_target;
use lapce_ai_rust::ipc::binary_codec::{BinaryCodec, MAGIC_HEADER, PROTOCOL_VERSION, HEADER_SIZE};
use lapce_ai_rust::ipc::zero_copy_codec::ZeroCopyCodec;
use bytes::BytesMut;
use tokio_util::codec::{Encoder, Decoder};

fuzz_target!(|data: &[u8]| {
    // Focus on header boundary conditions that could cause panics
    
    // Test 1: Header size boundaries (23, 24, 25 bytes)
    for boundary in [HEADER_SIZE - 1, HEADER_SIZE, HEADER_SIZE + 1] {
        if data.len() >= boundary {
            let boundary_data = &data[..boundary];
            
            // Test BinaryCodec
            let mut binary_codec = BinaryCodec::new();
            let _ = binary_codec.decode(boundary_data);
            
            // Test ZeroCopyCodec  
            let mut zero_codec = ZeroCopyCodec::new();
            let mut buffer = BytesMut::from(boundary_data);
            let _ = zero_codec.decode(&mut buffer);
        }
    }
    
    // Test 2: Malformed headers with valid structure but invalid data
    if data.len() >= HEADER_SIZE {
        let mut header = [0u8; HEADER_SIZE];
        
        // Copy fuzzed data into header structure
        for (i, &byte) in data.iter().take(HEADER_SIZE).enumerate() {
            header[i] = byte;
        }
        
        // Ensure valid magic but fuzz everything else
        header[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
        header[4] = PROTOCOL_VERSION;
        
        // Test with various payload sizes claimed in header
        for claimed_size in [0, 1, 100, 1000, 10000] {
            header[8..12].copy_from_slice(&(claimed_size as u32).to_le_bytes());
            
            // Create test data with claimed size if possible
            let total_size = HEADER_SIZE + claimed_size;
            if data.len() >= total_size {
                let test_data = &data[..total_size];
                
                let mut binary_codec = BinaryCodec::new();
                let _ = binary_codec.decode(test_data);
                
                let mut zero_codec = ZeroCopyCodec::new();
                let mut buffer = BytesMut::from(test_data);
                let _ = zero_codec.decode(&mut buffer);
            }
        }
    }
    
    // Test 3: Edge cases around max message size
    if data.len() >= HEADER_SIZE + 4 {
        let mut header = [0u8; HEADER_SIZE];
        header[0..4].copy_from_slice(&MAGIC_HEADER.to_le_bytes());
        header[4] = PROTOCOL_VERSION;
        
        // Test sizes around the boundary
        let boundary_sizes = [
            10 * 1024 * 1024 - 1,  // Just under limit
            10 * 1024 * 1024,      // At limit
            10 * 1024 * 1024 + 1,  // Just over limit
            u32::MAX,              // Maximum possible
        ];
        
        for size in boundary_sizes {
            header[8..12].copy_from_slice(&size.to_le_bytes());
            
            let mut binary_codec = BinaryCodec::new();
            let _ = binary_codec.decode(&header);
            
            let mut zero_codec = ZeroCopyCodec::new();
            let mut buffer = BytesMut::from(&header[..]);
            let _ = zero_codec.decode(&mut buffer);
        }
    }
    
    // Test 4: Incremental buffer building (streaming scenario)
    let mut zero_codec = ZeroCopyCodec::new();
    let mut streaming_buffer = BytesMut::new();
    
    // Add data byte by byte to simulate streaming
    for (i, &byte) in data.iter().enumerate() {
        streaming_buffer.extend_from_slice(&[byte]);
        let _ = zero_codec.decode(&mut streaming_buffer);
        
        // Limit iterations to prevent excessive fuzzing time
        if i > 1000 {
            break;
        }
    }
});
