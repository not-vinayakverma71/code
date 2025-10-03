/// Binary Protocol Codec - High-performance message encoding/decoding
/// Implements zero-copy serialization with rkyv

use bytes::{Bytes, BytesMut, BufMut};
use rkyv::{Archive, Deserialize, Serialize};
use anyhow::{Result, bail};
use crate::ipc_messages::{MessageType, AIRequest, ClineMessage, IpcMessage};

const MAGIC: u32 = 0x4C41504C; // "LAPL" - Lapce Protocol
const VERSION: u8 = 1;

/// Message header (12 bytes)
#[repr(C)]
pub struct MessageHeader {
    magic: u32,        // 4 bytes: Magic number
    version: u8,       // 1 byte: Protocol version
    msg_type: u8,      // 1 byte: Message type
    flags: u16,        // 2 bytes: Flags (compression, encryption, etc.)
    payload_len: u32,  // 4 bytes: Payload length
}

/// Codec flags
pub struct CodecFlags;
impl CodecFlags {
    pub const COMPRESSED: u16 = 1 << 0;
    pub const ENCRYPTED: u16 = 1 << 1;
    pub const STREAMING: u16 = 1 << 2;
    pub const PRIORITY: u16 = 1 << 3;
}

/// Binary codec for IPC messages
pub struct BinaryCodec {
    compression_threshold: usize,
    enable_compression: bool,
}

impl BinaryCodec {
    pub fn new() -> Self {
        Self {
            compression_threshold: 1024, // Compress messages > 1KB
            enable_compression: false,   // Disabled by default for max speed
        }
    }
    
    pub fn with_compression(mut self, threshold: usize) -> Self {
        self.enable_compression = true;
        self.compression_threshold = threshold;
        self
    }
    
    /// Encode message to binary format
    pub fn encode<T>(&self, msg_type: MessageType, payload: &T) -> Result<Bytes>
    where
        T: Serialize<rkyv::ser::serializers::AllocSerializer<256>>,
    {
        // Serialize payload with rkyv
        let serialized = rkyv::to_bytes::<_, 256>(payload)
            .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;
        
        let mut flags = 0u16;
        let payload_bytes: Vec<u8>;
        
        // Compress if needed
        if self.enable_compression && serialized.len() > self.compression_threshold {
            use zstd::stream::encode_all;
            payload_bytes = encode_all(serialized.as_ref(), 3)
                .unwrap_or_else(|_| serialized.to_vec());
            if payload_bytes.len() < serialized.len() {
                flags |= CodecFlags::COMPRESSED;
            }
        } else {
            payload_bytes = serialized.to_vec();
        }
        
        // Build message
        let total_size = 12 + payload_bytes.len(); // header + payload
        let mut buf = BytesMut::with_capacity(total_size);
        
        // Write header
        buf.put_u32_le(MAGIC);
        buf.put_u8(VERSION);
        buf.put_u8(msg_type as u8);
        buf.put_u16_le(flags);
        buf.put_u32_le(payload_bytes.len() as u32);
        
        // Write payload
        buf.extend_from_slice(&payload_bytes);
        
        Ok(buf.freeze())
    }
    
    /// Decode message from binary format
    pub fn decode<T>(&self, data: &[u8]) -> Result<(MessageType, T)>
    where
        T: Archive,
        T::Archived: Deserialize<T, rkyv::Infallible>,
    {
        if data.len() < 12 {
            bail!("Message too short: {} bytes", data.len());
        }
        
        // Parse header
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != MAGIC {
            bail!("Invalid magic number: 0x{:08x}", magic);
        }
        
        let version = data[4];
        if version != VERSION {
            bail!("Unsupported protocol version: {}", version);
        }
        
        let msg_type = data[5];
        let flags = u16::from_le_bytes([data[6], data[7]]);
        let payload_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        
        if data.len() < 12 + payload_len {
            bail!("Incomplete message: expected {} bytes, got {}", 
                  12 + payload_len, data.len());
        }
        
        let payload_data = &data[12..12 + payload_len];
        
        // Decompress if needed
        let decompressed: Vec<u8>;
        let final_payload = if flags & CodecFlags::COMPRESSED != 0 {
            use zstd::stream::decode_all;
            decompressed = decode_all(payload_data)
                .map_err(|e| anyhow::anyhow!("Decompression failed: {}", e))?;
            &decompressed
        } else {
            payload_data
        };
        
        // Deserialize with rkyv
        let archived = unsafe { 
            rkyv::archived_root::<T>(final_payload) 
        };
        let deserialized: T = archived.deserialize(&mut rkyv::Infallible)
            .map_err(|e| anyhow::anyhow!("Deserialization failed: {:?}", e))?;
        
        let message_type = match msg_type {
            0 => MessageType::Echo,
            1 => MessageType::Complete,
            2 => MessageType::Stream,
            3 => MessageType::Cancel,
            4 => MessageType::Heartbeat,
            5 => MessageType::Shutdown,
            99 => MessageType::Custom,
            _ => bail!("Unknown message type: {}", msg_type),
        };
        
        Ok((message_type, deserialized))
    }
    
    /// Encode raw bytes with header only (for pre-serialized data)
    pub fn encode_raw(&self, msg_type: MessageType, payload: &[u8]) -> Bytes {
        let mut buf = BytesMut::with_capacity(12 + payload.len());
        
        // Write header
        buf.put_u32_le(MAGIC);
        buf.put_u8(VERSION);
        buf.put_u8(msg_type as u8);
        buf.put_u16_le(0); // no flags for raw
        buf.put_u32_le(payload.len() as u32);
        
        // Write payload
        buf.extend_from_slice(payload);
        
        buf.freeze()
    }
    
    /// Extract header without decoding payload
    pub fn peek_header(data: &[u8]) -> Result<(MessageType, usize, u16)> {
        if data.len() < 12 {
            bail!("Message too short for header");
        }
        
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != MAGIC {
            bail!("Invalid magic number");
        }
        
        let msg_type = match data[5] {
            0 => MessageType::Echo,
            1 => MessageType::Complete,
            2 => MessageType::Stream,
            3 => MessageType::Cancel,
            4 => MessageType::Heartbeat,
            5 => MessageType::Shutdown,
            99 => MessageType::Custom,
            t => bail!("Unknown message type: {}", t),
        };
        
        let flags = u16::from_le_bytes([data[6], data[7]]);
        let payload_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        
        Ok((msg_type, payload_len, flags))
    }
}

impl Default for BinaryCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc_messages::{Message, MessageRole};
    
    #[test]
    #[ignore] // TODO: Fix after rkyv traits are properly implemented
    fn test_encode_decode() {
        let codec = BinaryCodec::new();
        
        // Create test message
        let request = AIRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                tool_calls: None,
            }],
            model: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(100),
            tools: None,
            system_prompt: None,
            stream: Some(false),
        };
        
        // Encode
        let encoded = codec.encode(MessageType::Complete, &request).unwrap();
        
        // Decode
        let (msg_type, decoded): (MessageType, AIRequest) = codec.decode(&encoded).unwrap();
        
        assert_eq!(msg_type, MessageType::Complete);
        assert_eq!(decoded.model, request.model);
        assert_eq!(decoded.messages.len(), request.messages.len());
    }
    
    #[test]
    fn test_peek_header() {
        let codec = BinaryCodec::new();
        let payload = b"test payload";
        let encoded = codec.encode_raw(MessageType::Echo, payload);
        
        let (msg_type, len, flags) = BinaryCodec::peek_header(&encoded).unwrap();
        assert_eq!(msg_type, MessageType::Echo);
        assert_eq!(len, payload.len());
        assert_eq!(flags, 0);
    }
}
