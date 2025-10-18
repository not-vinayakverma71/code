use anyhow::{Result, bail};
use bytes::{Bytes, BytesMut, BufMut};
use rkyv::{Archive, Deserialize, Serialize, AlignedVec};
use tokio_util::codec::{Encoder, Decoder};

// Protocol constants matching Codex TypeScript implementation
pub const MAGIC_BYTES: &[u8] = b"LAPC";
pub const MAGIC_HEADER: u32 = 0x4C415043;  // "LAPC" in hex
pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_SIZE: usize = 24;  // 4 (magic) + 1 (version) + 1 (flags) + 2 (type) + 4 (length) + 8 (id) + 4 (crc32)
pub const MIN_HEADER_SIZE: usize = HEADER_SIZE;  // Compatibility alias
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;  // 10MB max

// Flags
pub const FLAG_COMPRESSED: u8 = 0x01;
pub const FLAG_ENCRYPTED: u8 = 0x02;
pub const FLAG_STREAMING: u8 = 0x04;
pub const FLAG_PRIORITY: u8 = 0x08;

const COMPRESSION_THRESHOLD: usize = 1024;  // Compress messages > 1KB

/// Message types matching Codex protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Archive, Serialize, Deserialize)]
#[derive(serde::Serialize, serde::Deserialize)]
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
    ExecuteTool = 0x0107,
    ToolProgress = 0x0108,
    Ack = 0x0109,
    
    // LSP Gateway messages (native tree-sitter based)
    LspRequest = 0x0200,
    LspResponse = 0x0201,
    LspNotification = 0x0202,
    LspDiagnostics = 0x0203,
    LspProgress = 0x0204,
    Cancel = 0x0205,
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
            0x0107 => Ok(MessageType::ExecuteTool),
            0x0108 => Ok(MessageType::ToolProgress),
            0x0109 => Ok(MessageType::Ack),
            0x0200 => Ok(MessageType::LspRequest),
            0x0201 => Ok(MessageType::LspResponse),
            0x0202 => Ok(MessageType::LspNotification),
            0x0203 => Ok(MessageType::LspDiagnostics),
            0x0204 => Ok(MessageType::LspProgress),
            0x0205 => Ok(MessageType::Cancel),
            _ => bail!("Unknown message type: {:#x}", value),
        }
    }
}

/// Binary message envelope structure (24 bytes)
#[repr(C, packed)]
pub struct MessageEnvelope {
    pub magic: u32,      // 0-3: Magic number "LAPC" (LE)
    pub version: u8,     // 4: Protocol version
    pub flags: u8,       // 5: Bit flags
    pub msg_type: u16,   // 6-7: Message type (LE)
    pub length: u32,     // 8-11: Payload length (LE)
    pub msg_id: u64,     // 12-19: Message ID (LE)
    pub crc32: u32,      // 20-23: CRC32 checksum (LE)
}

impl MessageEnvelope {
    pub fn new(msg_type: MessageType, length: u32, flags: u8, msg_id: u64) -> Self {
        Self {
            magic: MAGIC_HEADER,
            version: PROTOCOL_VERSION,
            flags,
            msg_type: msg_type as u16,
            length,
            msg_id,
            crc32: 0,  // Will be calculated after full message is built
        }
    }
    
    pub fn validate(&self) -> Result<()> {
        let magic = self.magic;
        let version = self.version;
        let length = self.length;
        
        if magic != MAGIC_HEADER {
            bail!("Invalid magic header: {:#x}", magic);
        }
        if version != PROTOCOL_VERSION {
            bail!("Unsupported protocol version: {}", version);
        }
        if length as usize > MAX_MESSAGE_SIZE {
            bail!("Message too large: {} bytes", length);
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

impl Message {
    pub fn msg_type(&self) -> MessageType {
        self.msg_type
    }
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
    ToolResult { correlation_id: String, success: bool, result: Option<String>, error: Option<String> },
    ExecuteTool { tool_name: String, params: String, workspace_path: String, user_id: String, correlation_id: String, require_approval: bool },
    ToolProgress { correlation_id: String, message: String, percentage: Option<u8> },
    Ack,
    
    // LSP Gateway payloads
    LspRequest(LspRequestPayload),
    LspResponse(LspResponsePayload),
    LspNotification(LspNotificationPayload),
    LspDiagnostics(LspDiagnosticsPayload),
    LspProgress(LspProgressPayload),
    Cancel { request_id: String },
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

// ============================================================================
// LSP Gateway Payload Structures
// ============================================================================

/// LSP Request payload
/// Wraps LSP protocol requests (textDocument/*, workspace/*, etc.)
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct LspRequestPayload {
    /// LSP request ID (for request-response correlation)
    pub id: String,
    
    /// LSP method name (e.g., "textDocument/documentSymbol", "textDocument/hover")
    pub method: String,
    
    /// Document URI (e.g., "file:///path/to/file.rs")
    pub uri: String,
    
    /// Language identifier (e.g., "rust", "typescript", "python")
    pub language_id: String,
    
    /// LSP request parameters as JSON
    /// Stored as String to work with rkyv serialization
    pub params_json: String,
}

/// LSP Response payload
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct LspResponsePayload {
    /// Request ID this response corresponds to
    pub id: String,
    
    /// Success flag
    pub ok: bool,
    
    /// Response result as JSON (when ok=true)
    pub result_json: String,
    
    /// Error message (when ok=false)
    pub error: Option<String>,
    
    /// Error code (when ok=false)
    pub error_code: Option<i32>,
}

/// LSP Notification payload
/// For server-initiated notifications (no response expected)
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct LspNotificationPayload {
    /// Method name (e.g., "textDocument/didOpen", "textDocument/didChange")
    pub method: String,
    
    /// Document URI
    pub uri: String,
    
    /// Notification parameters as JSON
    pub params_json: String,
}

/// LSP Diagnostics payload
/// For streaming publishDiagnostics notifications
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct LspDiagnosticsPayload {
    /// Document URI these diagnostics belong to
    pub uri: String,
    
    /// Version number of the document (for ordering)
    pub version: Option<i32>,
    
    /// Diagnostics array as JSON
    /// Each diagnostic has: range, severity, code, source, message
    pub diagnostics_json: String,
}

/// LSP Progress notification payload
/// For long-running operations (indexing, scanning, etc.)
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct LspProgressPayload {
    /// Progress token (identifies the operation)
    pub token: String,
    
    /// Progress value as JSON
    /// Can be WorkDoneProgressBegin, WorkDoneProgressReport, or WorkDoneProgressEnd
    pub value_json: String,
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
                Some(Arc::new(zstd::bulk::Compressor::new(3)
                    .map_err(|e| anyhow::anyhow!("Failed to create compressor: {}", e))
                    .ok()?)),
                Some(Arc::new(zstd::bulk::Decompressor::new()
                    .map_err(|e| anyhow::anyhow!("Failed to create decompressor: {}", e))
                    .ok()?)),
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
        
        // Serialize the full message (including timestamp) with rkyv
        let archived = rkyv::to_bytes::<_, 256>(msg)
            .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;
        
        let payload_bytes = archived.as_slice();
        let mut payload_len = payload_bytes.len();
        
        // Check size limit
        if payload_len > self.max_message_size {
            bail!("Message too large: {} bytes", payload_len);
        }
        
        // Determine if compression needed
        let should_compress = self.enable_compression && payload_len > self.compression_threshold;
        let mut flags = 0u8;
        
        let final_payload = if should_compress {
            #[cfg(feature = "compression")]
            {
                if let Some(ref compressor) = self.compressor {
                    let compressed = compressor.compress(payload_bytes)
                        .map_err(|e| anyhow::anyhow!("Compression failed: {}", e))?;
                    payload_len = compressed.len();
                    flags |= FLAG_COMPRESSED;
                    Bytes::from(compressed)
                } else {
                    Bytes::copy_from_slice(payload_bytes)
                }
            }
            #[cfg(not(feature = "compression"))]
            Bytes::copy_from_slice(payload_bytes)
        } else {
            Bytes::copy_from_slice(payload_bytes)
        };
        
        // Write header (24 bytes) - all Little-Endian as per spec
        self.encode_buffer.put_u32_le(MAGIC_HEADER);
        self.encode_buffer.put_u8(PROTOCOL_VERSION);
        self.encode_buffer.put_u8(flags);
        self.encode_buffer.put_u16_le(msg.msg_type as u16);
        self.encode_buffer.put_u32_le(payload_len as u32);
        self.encode_buffer.put_u64_le(msg.id);
        
        // Reserve space for CRC32
        let crc_offset = self.encode_buffer.len();
        self.encode_buffer.put_u32_le(0);
        
        // Write payload
        self.encode_buffer.extend_from_slice(&final_payload);
        
        // Calculate CRC32 over entire message (header with crc=0 + payload)
        let crc = crc32fast::hash(&self.encode_buffer[..]);
        
        // Write CRC32 at reserved position
        self.encode_buffer[crc_offset..crc_offset+4].copy_from_slice(&crc.to_le_bytes());
        
        Ok(self.encode_buffer.clone().freeze())
    }
    
    /// Decode a message with zero-copy deserialization
    pub fn decode(&mut self, data: &[u8]) -> Result<Message> {
        if data.len() < HEADER_SIZE {
            bail!("Message too small: {} bytes, need at least {}", data.len(), HEADER_SIZE);
        }
        
        // Reject CPAL protocol explicitly (UI sends CPAL-wrapped JSON for streaming)
        // CPAL magic: [67, 80, 65, 76] = "CPAL" in ASCII
        if data[0] == 67 && data[1] == 80 && data[2] == 65 && data[3] == 76 {
            bail!("CPAL protocol detected - not binary codec format");
        }
        
        // Read header - all Little-Endian as per spec
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let version = data[4];
        let flags = data[5];
        let msg_type_raw = u16::from_le_bytes([data[6], data[7]]);
        let length = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let msg_id = u64::from_le_bytes([
            data[12], data[13], data[14], data[15],
            data[16], data[17], data[18], data[19]
        ]);
        let crc32_received = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
        
        // Validate magic and version
        if magic != MAGIC_HEADER {
            bail!("Invalid magic header: {:#x}, expected {:#x}", magic, MAGIC_HEADER);
        }
        if version != PROTOCOL_VERSION {
            bail!("Unsupported protocol version: {}, expected {}", version, PROTOCOL_VERSION);
        }
        
        // Check message size
        if length as usize > self.max_message_size {
            bail!("Message too large: {} bytes, max {}", length, self.max_message_size);
        }
        
        // Verify we have full message
        let total_size = HEADER_SIZE + length as usize;
        if data.len() < total_size {
            bail!("Incomplete message: expected {} bytes, got {}", total_size, data.len());
        }
        
        // Verify CRC32 (calculate with crc field set to 0)
        let mut check_data = data[..total_size].to_vec();
        check_data[20..24].fill(0);  // Zero out CRC field for calculation
        let calculated_crc = crc32fast::hash(&check_data);
        if calculated_crc != crc32_received {
            bail!("CRC32 mismatch: expected {:#x}, got {:#x}", crc32_received, calculated_crc);
        }
        
        // Get payload slice
        let payload_bytes = &data[HEADER_SIZE..total_size];
        
        // Decompress if needed
        let decompressed_payload = if (flags & FLAG_COMPRESSED) != 0 {
            #[cfg(feature = "compression")]
            {
                if let Some(ref decompressor) = self.decompressor {
                    Bytes::from(decompressor.decompress(payload_bytes, self.max_message_size)
                        .map_err(|e| anyhow::anyhow!("Decompression failed: {}", e))?)
                } else {
                    bail!("Message is compressed but decompressor not available");
                }
            }
            #[cfg(not(feature = "compression"))]
            bail!("Message is compressed but compression feature not enabled")
        } else {
            Bytes::copy_from_slice(payload_bytes)
        };
        
        // Deserialize full message (zero-copy with rkyv)
        let archived = rkyv::check_archived_root::<Message>(&decompressed_payload)
            .map_err(|e| anyhow::anyhow!("Archive validation failed: {}", e))?;
        let message: Message = archived.deserialize(&mut rkyv::Infallible)
            .map_err(|e| anyhow::anyhow!("Deserialization failed: {:?}", e))?;
        
        // Verify message type matches header
        let msg_type = MessageType::try_from(msg_type_raw)?;
        if message.msg_type != msg_type {
            bail!("Message type mismatch: header says {:?}, payload says {:?}", msg_type, message.msg_type);
        }
        
        // Verify message ID matches
        if message.id != msg_id {
            bail!("Message ID mismatch: header says {}, payload says {}", msg_id, message.id);
        }
        
        Ok(message)
    }
}

// Implement tokio_util codec traits
impl Decoder for BinaryCodec {
    type Item = Message;
    type Error = anyhow::Error;
    
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < HEADER_SIZE {
            return Ok(None); // Need more data
        }
        
        // Peek at length field (Little-Endian at offset 8)
        let length = u32::from_le_bytes([src[8], src[9], src[10], src[11]]) as usize;
        let total_len = HEADER_SIZE + length;
        
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
        
        let encoded = codec.encode(&msg).expect("Test encoding failed");
        let decoded = codec.decode(&encoded).expect("Test decoding failed");
        
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
