// Context Bridge: High-level API for context system operations
// Wires Lapce UI to lapce-ai context system (sliding window, condense, tracking)

use super::messages::{FileContextSource, InboundMessage, OutboundMessage};
use super::{BridgeClient, BridgeError};
use serde_json::Value as JsonValue;

/// Context Bridge: Provides high-level context system operations
pub struct ContextBridge {
    client: BridgeClient,
}

impl ContextBridge {
    /// Create a new context bridge
    pub fn new(client: BridgeClient) -> Self {
        Self { client }
    }

    /// Truncate conversation with sliding window algorithm
    ///
    /// This should be called before sending messages to the provider to ensure
    /// the conversation fits within the model's context window.
    ///
    /// # Arguments
    /// * `messages` - Full conversation history (as JSON)
    /// * `model_id` - Model identifier (e.g. "claude-3-5-sonnet-20241022")
    /// * `context_window` - Model's context window size in tokens
    /// * `max_tokens` - Optional max tokens for response
    ///
    /// # Returns
    /// Result with truncated messages (poll with `poll_context_response()`)
    pub fn truncate_conversation(
        &self,
        messages: Vec<JsonValue>,
        model_id: String,
        context_window: usize,
        max_tokens: Option<usize>,
    ) -> Result<(), BridgeError> {
        let msg = OutboundMessage::TruncateConversation {
            messages,
            model_id,
            context_window,
            max_tokens,
        };
        
        self.client.send(msg)
    }

    /// Condense conversation with LLM summarization
    ///
    /// This creates a compact summary of older messages to reduce token usage
    /// while preserving conversation context.
    ///
    /// # Arguments
    /// * `messages` - Full conversation history (as JSON)
    /// * `model_id` - Model identifier for summarization
    ///
    /// # Returns
    /// Result (poll with `poll_context_response()`)
    pub fn condense_conversation(
        &self,
        messages: Vec<JsonValue>,
        model_id: String,
    ) -> Result<(), BridgeError> {
        let msg = OutboundMessage::CondenseConversation { messages, model_id };
        self.client.send(msg)
    }

    /// Track file context (read/write/edit event)
    ///
    /// This records that a file has entered the AI's context, enabling
    /// stale file detection and context freshness tracking.
    ///
    /// # Arguments
    /// * `file_path` - Relative path to file (from workspace root)
    /// * `source` - How the file entered context (Read, Write, etc.)
    ///
    /// # Returns
    /// Result (poll with `poll_context_response()`)
    pub fn track_file_context(
        &self,
        file_path: String,
        source: FileContextSource,
    ) -> Result<(), BridgeError> {
        let msg = OutboundMessage::TrackFileContext { file_path, source };
        self.client.send(msg)
    }

    /// Get list of stale files
    ///
    /// Returns files that have been modified since they entered the AI's context.
    /// These files may need to be re-read to keep context fresh.
    ///
    /// # Arguments
    /// * `task_id` - Current task identifier
    ///
    /// # Returns
    /// Result (poll with `poll_context_response()`)
    pub fn get_stale_files(&self, task_id: String) -> Result<(), BridgeError> {
        let msg = OutboundMessage::GetStaleFiles { task_id };
        self.client.send(msg)
    }

    /// Poll for context system responses
    ///
    /// Call this in your event loop to receive context operation results.
    ///
    /// # Returns
    /// * `Some(InboundMessage)` - Response ready (check variants)
    /// * `None` - No response yet (try again later)
    pub fn poll_context_response(&self) -> Option<InboundMessage> {
        self.client.try_receive()
    }
}

// ============================================================================
// Helper functions for common patterns
// ============================================================================

/// Convert API messages to JSON for context operations
pub fn messages_to_json(messages: &[ApiMessage]) -> Vec<JsonValue> {
    messages
        .iter()
        .filter_map(|m| serde_json::to_value(m).ok())
        .collect()
}

/// Extract text content from JSON message
pub fn extract_text_from_message(msg: &JsonValue) -> Option<String> {
    msg.get("content")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

// Placeholder for ApiMessage type (should be imported from your types)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_bridge::transport::NoTransport;
    
    #[test]
    fn test_context_bridge_creation() {
        let transport = NoTransport {};
        let client = BridgeClient::new(Box::new(transport));
        let _context = ContextBridge::new(client);
        // If we get here, construction works
    }
    
    #[test]
    fn test_messages_to_json() {
        let messages = vec![
            ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
            ApiMessage {
                role: "assistant".to_string(),
                content: "Hi there!".to_string(),
            },
        ];
        
        let json_messages = messages_to_json(&messages);
        assert_eq!(json_messages.len(), 2);
        assert!(json_messages[0].get("role").is_some());
        assert_eq!(json_messages[0]["role"], "user");
    }
    
    #[test]
    fn test_extract_text_from_message() {
        let msg = serde_json::json!({
            "role": "user",
            "content": "Test message"
        });
        
        let text = extract_text_from_message(&msg);
        assert_eq!(text, Some("Test message".to_string()));
    }
}
