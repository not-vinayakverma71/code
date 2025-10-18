/// Codex Protocol Messages - Direct translation from TypeScript
/// Maintaining exact 1:1 compatibility with Codex protocol

use serde::{Deserialize, Serialize};
use rkyv::{Archive, Serialize as RkyvSerialize, Deserialize as RkyvDeserialize};
use std::collections::HashMap;

/// Protocol version for compatibility checking
pub const CODEX_PROTOCOL_VERSION: u32 = 1;

/// Message types matching Codex exactly
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub enum CodexMessageType {
    // Core messages
    Initialize = 0x01,
    Ready = 0x02,
    Shutdown = 0x03,
    Ping = 0x04,
    Pong = 0x05,
    
    // Request/Response
    Request = 0x10,
    Response = 0x11,
    Error = 0x12,
    Cancel = 0x13,
    
    // Streaming
    StreamStart = 0x20,
    StreamData = 0x21,
    StreamEnd = 0x22,
    StreamError = 0x23,
    
    // Notifications
    Notification = 0x30,
    Event = 0x31,
    Log = 0x32,
    Progress = 0x33,
    
    // File operations
    FileOpen = 0x40,
    FileClose = 0x41,
    FileRead = 0x42,
    FileWrite = 0x43,
    FileSave = 0x44,
    
    // Editor operations
    TextEdit = 0x50,
    CursorMove = 0x51,
    Selection = 0x52,
    Completion = 0x53,
    Hover = 0x54,
    Definition = 0x55,
    References = 0x56,
    
    // Debug
    DebugStart = 0x60,
    DebugStop = 0x61,
    DebugStep = 0x62,
    DebugBreakpoint = 0x63,
}

/// Base message structure - matches Codex wire format
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct CodexMessage {
    pub id: u64,
    pub msg_type: CodexMessageType,
    pub timestamp: u64,
    pub payload: Vec<u8>,
}

/// Initialize message
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct InitializeMessage {
    pub version: u32,
    pub client_id: String,
    pub capabilities: Vec<String>,
    pub config: HashMap<String, String>,  // Use HashMap instead of serde_json::Value
}

/// Request message
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct RequestMessage {
    pub method: String,
    pub params: Vec<u8>,  // Store as bytes for flexibility
    pub timeout_ms: Option<u32>,
}

/// Response message
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct ResponseMessage {
    pub request_id: u64,
    pub result: Option<Vec<u8>>,  // Store as bytes
    pub error: Option<ErrorInfo>,
}

/// Error information
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct ErrorInfo {
    pub code: i32,
    pub message: String,
    pub data: Option<String>,  // Store as string
}

/// Stream data chunk
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct StreamDataMessage {
    pub stream_id: u64,
    pub sequence: u32,
    pub data: Vec<u8>,
    pub is_final: bool,
}

/// Notification message
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct NotificationMessage {
    pub event: String,
    pub data: Vec<u8>,  // Store as bytes
}

/// Text edit operation
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct TextEditMessage {
    pub file_path: String,
    pub edits: Vec<TextEdit>,
    pub version: u32,
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct TextEdit {
    pub range: Range,
    pub text: String,
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Completion request
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct CompletionRequest {
    pub file_path: String,
    pub position: Position,
    pub trigger_character: Option<String>,
    pub context: CompletionContext,
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct CompletionContext {
    pub trigger_kind: u32,
    pub is_incomplete: bool,
}

/// Completion response
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct CompletionResponse {
    pub items: Vec<CompletionItem>,
    pub is_incomplete: bool,
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct CompletionItem {
    pub label: String,
    pub kind: u32,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: String,
    pub score: f32,
}

/// Protocol helpers
impl CodexMessage {
    pub fn new(msg_type: CodexMessageType, payload: Vec<u8>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        Self {
            id: rand::random(),
            msg_type,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            payload,
        }
    }
    
    pub fn serialize_payload<T: RkyvSerialize<rkyv::ser::serializers::AllocSerializer<256>>>(&mut self, data: &T) -> anyhow::Result<()> {
        let bytes = rkyv::to_bytes::<_, 256>(data)?;
        self.payload = bytes.to_vec();
        Ok(())
    }
    
    pub fn deserialize_payload<T>(&self) -> anyhow::Result<T>
    where
        T: rkyv::Archive,
        T::Archived: rkyv::Deserialize<T, rkyv::de::deserializers::SharedDeserializeMap>,
    {
        let archived = unsafe { rkyv::archived_root::<T>(&self.payload) };
        let deserialized: T = archived.deserialize(&mut rkyv::de::deserializers::SharedDeserializeMap::new())?;
        Ok(deserialized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_serialization() {
        let init = InitializeMessage {
            version: CODEX_PROTOCOL_VERSION,
            client_id: "test-client".to_string(),
            capabilities: vec!["completion".to_string(), "hover".to_string()],
            config: HashMap::new(),
        };
        
        let mut msg = CodexMessage::new(CodexMessageType::Initialize, vec![]);
        msg.serialize_payload(&init).unwrap();
        
        let decoded: InitializeMessage = msg.deserialize_payload().unwrap();
        assert_eq!(decoded.client_id, "test-client");
        assert_eq!(decoded.capabilities.len(), 2);
    }
    
    #[test]
    fn test_streaming_message() {
        let stream_data = StreamDataMessage {
            stream_id: 42,
            sequence: 1,
            data: vec![1, 2, 3, 4, 5],
            is_final: false,
        };
        
        let mut msg = CodexMessage::new(CodexMessageType::StreamData, vec![]);
        msg.serialize_payload(&stream_data).unwrap();
        
        let decoded: StreamDataMessage = msg.deserialize_payload().unwrap();
        assert_eq!(decoded.stream_id, 42);
        assert_eq!(decoded.data, vec![1, 2, 3, 4, 5]);
    }
}
