// Context Integration Example
// Shows how to integrate context system into Lapce AI chat panel

use super::context_bridge::{ApiMessage, ContextBridge};
use super::messages::{FileContextSource, InboundMessage};
use super::{BridgeClient, ShmTransport};

/// Example: AI Chat Panel with Context System Integration
pub struct AiChatPanelExample {
    context_bridge: ContextBridge,
    current_task_id: String,
    model_id: String,
    context_window: usize,
}

impl AiChatPanelExample {
    /// Create new AI chat panel
    pub fn new(workspace_path: String) -> Self {
        // Create shared memory transport
        let transport = ShmTransport::new(workspace_path);
        let client = BridgeClient::new(Box::new(transport));
        
        Self {
            context_bridge: ContextBridge::new(client),
            current_task_id: "task-001".to_string(),
            model_id: "claude-3-5-sonnet-20241022".to_string(),
            context_window: 200000, // Claude 3.5 Sonnet context window
        }
    }
    
    /// Called before sending message to AI provider
    /// Returns truncated messages if needed
    pub fn prepare_conversation_for_send(
        &mut self,
        messages: Vec<ApiMessage>,
    ) -> Result<Vec<ApiMessage>, String> {
        // Convert to JSON
        let json_messages = super::context_bridge::messages_to_json(&messages);
        
        // Request truncation
        self.context_bridge
            .truncate_conversation(
                json_messages,
                self.model_id.clone(),
                self.context_window,
                None, // Use default max_tokens from model
            )
            .map_err(|e| format!("Failed to truncate: {}", e))?;
        
        // Poll for response (in real code, this would be async)
        loop {
            if let Some(response) = self.context_bridge.poll_context_response() {
                match response {
                    InboundMessage::TruncateConversationResponse {
                        messages: truncated_msgs,
                        summary,
                        cost,
                        new_context_tokens,
                        prev_context_tokens,
                    } => {
                        // Show notification if truncated
                        if prev_context_tokens > new_context_tokens.unwrap_or(prev_context_tokens) {
                            println!(
                                "✂️ Truncated: {} → {} tokens. Cost: ${:.4}",
                                prev_context_tokens,
                                new_context_tokens.unwrap_or(0),
                                cost
                            );
                            println!("📝 Summary: {}", summary);
                        }
                        
                        // Convert back to ApiMessage
                        let result: Vec<ApiMessage> = truncated_msgs
                            .into_iter()
                            .filter_map(|m| serde_json::from_value(m).ok())
                            .collect();
                        
                        return Ok(result);
                    }
                    
                    InboundMessage::ContextError { operation, message } => {
                        return Err(format!("Context operation '{}' failed: {}", operation, message));
                    }
                    
                    _ => {} // Ignore other messages
                }
            }
            
            // In real code: await, timeout, or return pending
            break;
        }
        
        // Fallback: return original if truncation didn't complete
        Ok(messages)
    }
    
    /// Called when a file is opened in the editor
    pub fn on_file_opened(&mut self, file_path: String) {
        let _ = self.context_bridge.track_file_context(
            file_path,
            FileContextSource::Read,
        );
    }
    
    /// Called when user edits and saves a file
    pub fn on_file_saved_by_user(&mut self, file_path: String) {
        let _ = self.context_bridge.track_file_context(
            file_path,
            FileContextSource::UserEdit,
        );
    }
    
    /// Called when AI writes to a file
    pub fn on_file_written_by_ai(&mut self, file_path: String) {
        let _ = self.context_bridge.track_file_context(
            file_path,
            FileContextSource::Write,
        );
    }
    
    /// Check for stale files and display warning
    pub fn check_stale_files(&mut self) -> Result<Vec<String>, String> {
        // Request stale files
        self.context_bridge
            .get_stale_files(self.current_task_id.clone())
            .map_err(|e| format!("Failed to get stale files: {}", e))?;
        
        // Poll for response
        loop {
            if let Some(response) = self.context_bridge.poll_context_response() {
                match response {
                    InboundMessage::StaleFilesResponse { stale_files } => {
                        if !stale_files.is_empty() {
                            println!(
                                "⚠️ {} file(s) may be outdated: {}",
                                stale_files.len(),
                                stale_files.join(", ")
                            );
                        }
                        return Ok(stale_files);
                    }
                    
                    InboundMessage::ContextError { operation, message } => {
                        return Err(format!("Context operation '{}' failed: {}", operation, message));
                    }
                    
                    _ => {} // Ignore other messages
                }
            }
            
            break;
        }
        
        Ok(vec![])
    }
    
    /// Event loop: Poll for context system events
    pub fn poll_context_events(&mut self) {
        while let Some(event) = self.context_bridge.poll_context_response() {
            match event {
                InboundMessage::TruncateConversationResponse { summary, .. } => {
                    println!("✅ Conversation truncated. Summary: {}", summary);
                }
                
                InboundMessage::CondenseConversationResponse {
                    summary,
                    messages_condensed,
                    cost,
                } => {
                    println!(
                        "✅ Condensed {} messages. Cost: ${:.4}\nSummary: {}",
                        messages_condensed, cost, summary
                    );
                }
                
                InboundMessage::TrackFileContextResponse { success, error } => {
                    if !success {
                        if let Some(err) = error {
                            eprintln!("❌ Failed to track file: {}", err);
                        }
                    }
                }
                
                InboundMessage::StaleFilesResponse { stale_files } => {
                    if !stale_files.is_empty() {
                        println!("⚠️ Stale files detected: {:?}", stale_files);
                    }
                }
                
                InboundMessage::ContextError { operation, message } => {
                    eprintln!("❌ Context operation '{}' failed: {}", operation, message);
                }
                
                _ => {} // Ignore non-context messages
            }
        }
    }
}

// ============================================================================
// Usage Example
// ============================================================================

#[cfg(test)]
mod usage_example {
    use super::*;
    
    #[test]
    #[ignore] // This is an example, not a real test
    fn example_usage() {
        // 1. Create AI chat panel with context system
        let mut chat = AiChatPanelExample::new("/home/user/project".to_string());
        
        // 2. User sends a long conversation
        let messages = vec![
            // ... 100 messages
        ];
        
        // 3. Before sending to provider, truncate if needed
        match chat.prepare_conversation_for_send(messages) {
            Ok(truncated_messages) => {
                println!("Sending {} messages to provider", truncated_messages.len());
                // send_to_provider(truncated_messages);
            }
            Err(e) => {
                eprintln!("Failed to prepare conversation: {}", e);
            }
        }
        
        // 4. User opens a file
        chat.on_file_opened("src/main.rs".to_string());
        
        // 5. AI writes to a file
        chat.on_file_written_by_ai("src/lib.rs".to_string());
        
        // 6. Check for stale files
        if let Ok(stale) = chat.check_stale_files() {
            if !stale.is_empty() {
                println!("⚠️ Please review: {:?}", stale);
            }
        }
        
        // 7. Poll for events in main loop
        chat.poll_context_events();
    }
}

// ============================================================================
// Integration Points for Floem UI
// ============================================================================

/// Example: Wiring context system into Floem AI chat view
#[allow(dead_code)]
fn floem_integration_example() {
    // Pseudo-code showing integration points:
    
    // 1. In ChatView::new()
    //    self.context_bridge = ContextBridge::new(bridge_client);
    
    // 2. In send_message()
    //    let truncated = self.context_bridge.truncate_conversation(...)?;
    //    self.provider.send(truncated).await?;
    
    // 3. In view() method
    //    if stale_files_count > 0 {
    //        badge("⚠️ Files outdated", WarningStyle)
    //    }
    
    // 4. In update() method (event loop)
    //    if let Some(response) = self.context_bridge.poll_context_response() {
    //        match response {
    //            InboundMessage::TruncateConversationResponse { ... } => {
    //                // Update UI
    //            }
    //            ...
    //        }
    //    }
    
    println!("See IPC_INTEGRATION_GUIDE.md for full Floem examples");
}
