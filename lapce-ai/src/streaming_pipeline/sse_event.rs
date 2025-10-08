/// SSE Event Types - Core streaming infrastructure
/// Phase 1, Task 2: Create SseEvent type

use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Server-Sent Event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    /// Event type field (optional)
    pub event_type: Option<String>,
    
    /// Data payload
    pub data: Bytes,
    
    /// Event ID (optional)
    pub id: Option<String>,
    
    /// Retry delay in milliseconds (optional)
    pub retry: Option<u64>,
}

impl SseEvent {
    /// Create a new SSE event with data only
    pub fn new(data: impl Into<Bytes>) -> Self {
        Self {
            event_type: None,
            data: data.into(),
            id: None,
            retry: None,
        }
    }
    
    /// Create event with type and data
    pub fn with_type(event_type: impl Into<String>, data: impl Into<Bytes>) -> Self {
        Self {
            event_type: Some(event_type.into()),
            data: data.into(),
            id: None,
            retry: None,
        }
    }
    
    /// Check if this is a DONE event (OpenAI format)
    pub fn is_done(&self) -> bool {
        self.data.starts_with(b"[DONE]")
    }
    
    /// Parse data as JSON
    pub fn parse_json<T: for<'a> Deserialize<'a>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sse_event_creation() {
        let event = SseEvent::new(&b"test data"[..]);
        assert_eq!(event.data, Bytes::from(&b"test data"[..]));
        assert!(event.event_type.is_none());
    }
    
    #[test]
    fn test_done_detection() {
        let done_bytes = b"[DONE]";
        let event = SseEvent::new(&done_bytes[..]);
        assert!(event.is_done());
        
        let not_done_bytes = b"not done";
        let event2 = SseEvent::new(&not_done_bytes[..]);
        assert!(!event2.is_done());
    }
}
