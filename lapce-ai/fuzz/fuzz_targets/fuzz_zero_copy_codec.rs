#![no_main]
use libfuzzer_sys::fuzz_target;
use lapce_ai_rust::ipc::zero_copy_codec::ZeroCopyCodec;
use lapce_ai_rust::ipc::binary_codec::{HEADER_SIZE, Message, MessageType, MessagePayload};
use bytes::BytesMut;
use tokio_util::codec::{Encoder, Decoder};

fuzz_target!(|data: &[u8]| {
    let mut codec = ZeroCopyCodec::new();
    let mut buffer = BytesMut::from(data);
    
    // Try to decode arbitrary data - should not panic
    let _ = codec.decode(&mut buffer);
    
    // Test header boundary conditions
    if data.len() >= HEADER_SIZE {
        let mut header_buffer = BytesMut::from(&data[..HEADER_SIZE]);
        let _ = codec.decode(&mut header_buffer);
    }
    
    // Test oversized buffers
    if data.len() > HEADER_SIZE + 1000 {
        let mut large_buffer = BytesMut::from(&data[..HEADER_SIZE + 1000]);
        let _ = codec.decode(&mut large_buffer);
    }
    
    // Test encoding with fuzzed data if we can create a valid message
    if data.len() >= 8 {
        let id = u64::from_le_bytes([
            data.get(0).copied().unwrap_or(0),
            data.get(1).copied().unwrap_or(0),
            data.get(2).copied().unwrap_or(0),
            data.get(3).copied().unwrap_or(0),
            data.get(4).copied().unwrap_or(0),
            data.get(5).copied().unwrap_or(0),
            data.get(6).copied().unwrap_or(0),
            data.get(7).copied().unwrap_or(0),
        ]);
        
        // Create a test message with fuzzed ID
        let msg = Message {
            id,
            msg_type: MessageType::Heartbeat,
            payload: MessagePayload::Heartbeat,
            timestamp: 1234567890,
        };
        
        let mut encode_buffer = BytesMut::new();
        let _ = codec.encode(msg, &mut encode_buffer);
    }
    
    // Test partial buffer conditions
    for i in 1..std::cmp::min(data.len(), HEADER_SIZE + 100) {
        let mut partial_buffer = BytesMut::from(&data[..i]);
        let _ = codec.decode(&mut partial_buffer);
    }
});
