/// Binary Protocol Implementation with rkyv
/// Zero-copy serialization for high-performance IPC
/// Implements the protocol from docs/02-BINARY-PROTOCOL-DESIGN.md

use std::sync::Arc;
use anyhow::{Result, bail};
use bytes::{Bytes, BytesMut, BufMut};
use rkyv::{Archive, Deserialize, Serialize, AlignedVec};
use tokio_util::codec::{Encoder, Decoder};

// Protocol constants matching Codex TypeScript implementation
pub const MAGIC_BYTES: &[u8] = b"LAPC";
pub const MAGIC_HEADER: u32 = 0x4C415043;  // "LAPC" in hex
pub const PROTOCOL_VERSION: u8 = 1;
pub const MIN_HEADER_SIZE: usize = 16;  // 4 (magic) + 1 (version) + 1 (flags) + 2 (type) + 4 (length) + 4 (checksum)
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;  // 10MB max

// Flags
pub const FLAG_COMPRESSED: u8 = 0x01;
pub const FLAG_ENCRYPTED: u8 = 0x02;
pub const FLAG_STREAMING: u8 = 0x04;
pub const FLAG_PRIORITY: u8 = 0x08;

const COMPRESSION_THRESHOLD: usize = 1024;  // Compress messages > 1KB

/// Message types matching Codex protocol
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[archive(check_bytes)]
#[repr(u16)]
pub enum MessageType {
    // Core messages
    CompletionRequest = 0x0001,
    CompletionResponse = 0x0002,
    StreamChunk = 0x0003,
    Error = 0x0004,
    Heartbeat = 0x0005,
    
    // Control messages
    Handshake = 0x0010,
    HandshakeAck = 0x0011,
    Disconnect = 0x0012,
    
    // AI-specific messages matching Codex
    AskRequest = 0x0100,
    AskResponse = 0x0101,
    EditRequest = 0x0102,
    EditResponse = 0x0103,
    ChatMessage = 0x0104,
    ToolCall = 0x0105,
    ToolResult = 0x0106,
}

impl TryFrom<u16> for MessageType {
    type Error = anyhow::Error;
    
    fn try_from(value: u16) -> Result<Self> {
        match value {
            0x0001 => Ok(MessageType::CompletionRequest),
            0x0002 => Ok(MessageType::CompletionResponse),
            0x0003 => Ok(MessageType::StreamChunk),
            0x0004 => Ok(MessageType::Error),
            0x0005 => Ok(MessageType::Heartbeat),
            0x0010 => Ok(MessageType::Handshake),
            0x0011 => Ok(MessageType::HandshakeAck),
            0x0012 => Ok(MessageType::Disconnect),
            0x0100 => Ok(MessageType::AskRequest),
            0x0101 => Ok(MessageType::AskResponse),
            0x0102 => Ok(MessageType::EditRequest),
            0x0103 => Ok(MessageType::EditResponse),
            0x0104 => Ok(MessageType::ChatMessage),
            0x0105 => Ok(MessageType::ToolCall),
            0x0106 => Ok(MessageType::ToolResult),
            _ => bail!("Unknown message type: {:#x}", value),
        }
    }
}

/// Binary message envelope structure
#[repr(C)]
pub struct MessageEnvelope {
    pub magic: u32,
    pub version: u8,
    pub flags: u8,
    pub msg_type: u16,
    pub length: u32,
    pub checksum: u32,
}

impl MessageEnvelope {
    pub fn new(msg_type: MessageType, length: u32, flags: u8) -> Self {
        Self {
            magic: MAGIC_HEADER,
            version: PROTOCOL_VERSION,
            flags,
            msg_type: msg_type as u16,
            length,
            checksum: 0,  // Will be calculated after payload is added
        }
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.magic != MAGIC_HEADER {
            bail!("Invalid magic header: {:#x}", self.magic);
        }
        if self.version != PROTOCOL_VERSION {
            bail!("Unsupported protocol version: {}", self.version);
        }
        if self.length as usize > MAX_MESSAGE_SIZE {
            bail!("Message too large: {} bytes", self.length);
        }
        Ok(())
    }
}

/// Core message structure
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct Message {
    pub id: u64,
    pub msg_type: MessageType,
    pub payload: MessagePayload,
    pub timestamp: u64,
}

/// Message payloads matching Codex TypeScript types
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub enum MessagePayload {
    CompletionRequest(CompletionRequest),
    CompletionResponse(CompletionResponse),
    StreamChunk(StreamChunk),
    Error(ErrorMessage),
    Heartbeat,
    
    // Codex-specific payloads
    AskRequest(AskRequest),
    AskResponse(AskResponse),
    EditRequest(EditRequest),
    EditResponse(EditResponse),
    ChatMessage(ChatMessage),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct CompletionRequest {
    pub prompt: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub stream: bool,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
    pub tokens_used: u32,
    pub finish_reason: String,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct StreamChunk {
    pub stream_id: u64,
    pub sequence: u32,
    pub content: ChunkContent,
    pub is_final: bool,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub enum ChunkContent {
    Text(String),
    Token(Token),
    FunctionCall(FunctionCall),
    Error(String),
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct Token {
    pub id: u32,
    pub text: String,
    pub logprob: f32,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct ErrorMessage {
    pub code: i32,
    pub message: String,
    pub details: String,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct AskRequest {
    pub query: String,
    pub context: Vec<String>,
    pub model: String,
    pub max_tokens: u32,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct AskResponse {
    pub answer: String,
    pub confidence: f32,
    pub sources: Vec<String>,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct EditRequest {
    pub file_path: String,
    pub original_code: String,
    pub instructions: String,
    pub language: String,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct EditResponse {
    pub modified_code: String,
    pub diff: String,
    pub explanation: String,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct ChatMessage {
    pub role: String,  // "user", "assistant", "system"
    pub content: String,
    pub timestamp: u64,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: String,  // JSON string
    pub id: String,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub result: String,
    pub error: Option<String>,
}

/// Binary codec for encoding/decoding messages
#[derive(Clone)]
pub struct BinaryCodec {
    // Pre-allocated buffers
    encode_buffer: BytesMut,
    aligned_buffer: AlignedVec,
    
    // Settings
    enable_compression: bool,
    compression_threshold: usize,  // Min size to trigger compression
    max_message_size: usize,
    
    // Optional zstd compressor/decompressor
    #[cfg(feature = "compression")]
    compressor: Option<Arc<zstd::bulk::Compressor<'static>>>,
    #[cfg(feature = "compression")]
    decompressor: Option<Arc<zstd::bulk::Decompressor<'static>>>,
}

impl BinaryCodec {
    pub fn new() -> Self {
        Self::with_compression(false)
    }
    
    pub fn with_compression(enable: bool) -> Self {
        #[cfg(feature = "compression")]
        let (compressor, decompressor) = if enable {
            (
                Some(Arc::new(zstd::bulk::Compressor::new(3).unwrap())),
                Some(Arc::new(zstd::bulk::Decompressor::new().unwrap())),
            )
        } else {
            (None, None)
        };
        
        Self {
            encode_buffer: BytesMut::with_capacity(1024 * 1024),  // 1MB initial
            aligned_buffer: AlignedVec::new(),
            enable_compression: enable,
            compression_threshold: 1024,  // Compress messages > 1KB
            max_message_size: MAX_MESSAGE_SIZE,
            #[cfg(feature = "compression")]
            compressor,
            #[cfg(feature = "compression")]
            decompressor,
        }
    }
    
    /// Encode a message with zero-copy serialization
    pub fn encode(&mut self, msg: &Message) -> Result<Bytes> {
        // Clear buffers
        self.encode_buffer.clear();
        self.aligned_buffer.clear();
        
        // Serialize payload with rkyv (zero-copy)
        let archived = rkyv::to_bytes::<_, 256>(&msg.payload)
            .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;
        
        let payload_bytes = archived.as_slice();
        let payload_len = payload_bytes.len();
        
        // Check size limit
        if payload_len > self.max_message_size {
            bail!("Message too large: {} bytes", payload_len);
        }
        
        // Determine if compression needed
        let should_compress = self.enable_compression && payload_len > COMPRESSION_THRESHOLD;
        let flags = if should_compress { FLAG_COMPRESSED } else { 0 };
        
        // Create envelope
        let envelope = MessageEnvelope::new(msg.msg_type, payload_len as u32, flags);
        
        // Write envelope (16 bytes)
        self.encode_buffer.put_u32(envelope.magic);
        self.encode_buffer.put_u8(envelope.version);
        self.encode_buffer.put_u8(envelope.flags);
        self.encode_buffer.put_u16(envelope.msg_type);
        self.encode_buffer.put_u32(envelope.length);
        
        // Calculate simple checksum (sum of bytes for now, CRC32 can be added later)
        let checksum = payload_bytes.iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32));
        self.encode_buffer.put_u32(checksum);
        
        // Write payload
        self.encode_buffer.extend_from_slice(payload_bytes);
        
        Ok(self.encode_buffer.clone().freeze())
    }
    
    /// Decode a message with zero-copy deserialization
    pub fn decode(&mut self, data: &[u8]) -> Result<Message> {
        if data.len() < MIN_HEADER_SIZE {
            bail!("Message too small: {} bytes", data.len());
        }
        
        // Read envelope
        let mut cursor = 0;
        let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        cursor += 4;
        
        let version = data[cursor];
        cursor += 1;
        
        let flags = data[cursor];
        cursor += 1;
        
        let msg_type_raw = u16::from_be_bytes([data[cursor], data[cursor + 1]]);
        cursor += 2;
        
        let length = u32::from_be_bytes([data[cursor], data[cursor + 1], data[cursor + 2], data[cursor + 3]]);
        cursor += 4;
        
        let checksum = u32::from_be_bytes([data[cursor], data[cursor + 1], data[cursor + 2], data[cursor + 3]]);
        cursor += 4;
        
        // Validate envelope
        let envelope = MessageEnvelope {
            magic,
            version,
            flags,
            msg_type: msg_type_raw,
            length,
            checksum,
        };
        envelope.validate()?;
        
        // Verify we have full payload
        let payload_end = cursor + length as usize;
        if data.len() < payload_end {
            bail!("Incomplete message: expected {} bytes, got {}", payload_end, data.len());
        }
        
        // Get payload slice
        let payload_bytes = &data[cursor..payload_end];
        
        // Verify checksum
        let calculated_checksum = payload_bytes.iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32));
        if calculated_checksum != checksum {
            bail!("Checksum mismatch: expected {:#x}, got {:#x}", checksum, calculated_checksum);
        }
        
        // Deserialize payload (zero-copy with rkyv)
        let msg_type = MessageType::try_from(msg_type_raw)?;
        let archived = rkyv::check_archived_root::<MessagePayload>(payload_bytes)
            .map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))?;
        let payload: MessagePayload = archived.deserialize(&mut rkyv::Infallible)
            .map_err(|e| anyhow::anyhow!("Deserialization failed: {:?}", e))?;
        
        Ok(Message {
            id: 0, // Will be set by caller
            msg_type,
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
}

// Implement tokio_util codec traits
impl Decoder for BinaryCodec {
    type Item = Message;
    type Error = anyhow::Error;
    
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < MIN_HEADER_SIZE {
            return Ok(None); // Need more data
        }
        
        // Peek at length field
        let length = u32::from_be_bytes([src[8], src[9], src[10], src[11]]) as usize;
        let total_len = MIN_HEADER_SIZE + length;
        
        if src.len() < total_len {
            return Ok(None); // Need more data
        }
        
        // We have a complete message
        let data = src.split_to(total_len);
        self.decode(&data).map(Some)
    }
}

impl Encoder<Message> for BinaryCodec {
    type Error = anyhow::Error;
    
    fn encode(&mut self, msg: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let encoded = self.encode(&msg)?;
        dst.extend_from_slice(&encoded);
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encode_decode() {
        let mut codec = BinaryCodec::new();
        
        let msg = Message {
            id: 12345,
            msg_type: MessageType::CompletionRequest,
            payload: MessagePayload::CompletionRequest(CompletionRequest {
                prompt: "Hello, world!".to_string(),
                model: "gpt-4".to_string(),
                max_tokens: 100,
                temperature: 0.7,
                stream: false,
            }),
            timestamp: 1234567890,
        };
        
        let encoded = codec.encode(&msg).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        
        assert_eq!(decoded.id, msg.id);
        assert_eq!(decoded.msg_type, msg.msg_type);
        assert_eq!(decoded.timestamp, msg.timestamp);
    }
    
    #[test]
    fn test_header_validation() {
        let mut codec = BinaryCodec::new();
        
        // Too small
        let result = codec.decode(&[0u8; 5]);
        assert!(result.is_err());
        
        // Wrong magic
        let mut bad_data = vec![0u8; MIN_HEADER_SIZE + 10];
        bad_data[0..4].copy_from_slice(&0xDEADBEEFu32.to_be_bytes());
        let result = codec.decode(&bad_data);
        assert!(result.is_err());
    }
}
