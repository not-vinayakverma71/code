// IPC route handlers for context system operations
// Implements IPC-ROUTES-26/27: Sliding window, condense, and context tracking

use super::ipc_messages::{
    ContextCommand, ContextResponse,
    TruncateConversationRequest, TruncateConversationResponse,
    CondenseConversationRequest, CondenseConversationResponse,
    TrackFileContextRequest, TrackFileContextResponse,
    GetStaleFilesRequest, GetStaleFilesResponse,
    FileContextEventType,
};
use crate::core::sliding_window::{self, ApiMessage, TruncateOptions, MessageContent};
use crate::core::condense;
use crate::core::context_tracking::{FileContextTracker, RecordSource};
use crate::core::token_counter;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::PathBuf;
use anyhow::Result;

/// Context system route handler
pub struct ContextRouteHandler {
    workspace: PathBuf,
    context_tracker: Arc<RwLock<FileContextTracker>>,
}

impl ContextRouteHandler {
    /// Create new context route handler
    pub fn new(workspace: PathBuf, task_id: String) -> Self {
        let task_storage_path = workspace.join(".roo-task");
        let tracker = FileContextTracker::new(
            task_id,
            task_storage_path,
            Some(workspace.clone()),
        );
        
        Self {
            workspace,
            context_tracker: Arc::new(RwLock::new(tracker)),
        }
    }
    
    /// Handle context command
    pub async fn handle_command(&self, command: ContextCommand) -> ContextResponse {
        match command {
            ContextCommand::TruncateConversation(req) => {
                match self.handle_truncate(req).await {
                    Ok(resp) => ContextResponse::TruncateConversation(resp),
                    Err(e) => ContextResponse::Error {
                        message: format!("Truncate failed: {}", e),
                    },
                }
            }
            
            ContextCommand::CondenseConversation(req) => {
                match self.handle_condense(req).await {
                    Ok(resp) => ContextResponse::CondenseConversation(resp),
                    Err(e) => ContextResponse::Error {
                        message: format!("Condense failed: {}", e),
                    },
                }
            }
            
            ContextCommand::TrackFileContext(req) => {
                match self.handle_track_file(req).await {
                    Ok(resp) => ContextResponse::TrackFileContext(resp),
                    Err(e) => ContextResponse::Error {
                        message: format!("Track file failed: {}", e),
                    },
                }
            }
            
            ContextCommand::GetStaleFiles(req) => {
                match self.handle_get_stale_files(req).await {
                    Ok(resp) => ContextResponse::GetStaleFiles(resp),
                    Err(e) => ContextResponse::Error {
                        message: format!("Get stale files failed: {}", e),
                    },
                }
            }
        }
    }
    
    /// Handle truncate conversation request
    async fn handle_truncate(
        &self,
        req: TruncateConversationRequest,
    ) -> Result<TruncateConversationResponse> {
        // Convert JSON messages to ApiMessage
        let messages: Vec<ApiMessage> = req.messages
            .into_iter()
            .filter_map(|msg| serde_json::from_value(msg).ok())
            .collect();
        
        // Get total tokens first (simple text extraction for now)
        let total_tokens = messages.iter()
            .map(|m| {
                let text = match &m.content {
                    MessageContent::Text(s) => s.clone(),
                    MessageContent::ContentBlocks(blocks) => {
                        blocks.iter()
                            .filter_map(|b| match b {
                                crate::core::sliding_window::ContentBlock::Text { text } => Some(text.clone()),
                                crate::core::sliding_window::ContentBlock::ToolResult { content, .. } => Some(content.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                };
                token_counter::count_tokens(&text, &req.model_id).unwrap_or(0)
            })
            .sum();
        
        // Create truncate options
        let options = TruncateOptions {
            messages,
            total_tokens,
            model_id: req.model_id.clone(),
            context_window: req.context_window,
            max_tokens: req.max_tokens,
            auto_condense_context: false,  // Will be configurable in Phase C
            auto_condense_context_percent: 0.5,
            system_prompt: String::new(),
            task_id: "current-task".to_string(),  // Will come from context in Phase C
            custom_condensing_prompt: None,
            profile_thresholds: std::collections::HashMap::new(),
            current_profile_id: "default".to_string(),
        };
        
        // Call sliding window truncation
        let result = sliding_window::truncate_conversation_if_needed(options)
            .await
            .map_err(|e| anyhow::anyhow!("Truncate failed: {}", e))?;
        
        // Convert response
        let response_messages: Vec<serde_json::Value> = result.messages
            .into_iter()
            .filter_map(|msg| serde_json::to_value(msg).ok())
            .collect();
        
        Ok(TruncateConversationResponse {
            messages: response_messages,
            summary: result.summary,
            cost: result.cost,
            new_context_tokens: result.new_context_tokens,
            prev_context_tokens: result.prev_context_tokens,
        })
    }
    
    /// Handle condense conversation request
    async fn handle_condense(
        &self,
        req: CondenseConversationRequest,
    ) -> Result<CondenseConversationResponse> {
        // Convert JSON messages to ApiMessage
        let messages: Vec<ApiMessage> = req.messages
            .into_iter()
            .filter_map(|msg| serde_json::from_value(msg).ok())
            .collect();
        
        // Get messages since last summary
        let messages_to_condense = condense::get_messages_since_last_summary(&messages);
        
        // For now, return a placeholder response
        // Full implementation requires provider integration for streaming summarization
        // This will be completed in PORT-CD-11
        Ok(CondenseConversationResponse {
            summary: format!(
                "Conversation summary (condensing {} messages)",
                messages_to_condense.len()
            ),
            messages_condensed: messages_to_condense.len(),
            cost: 0.0, // Will be calculated when provider integration is complete
        })
    }
    
    /// Handle track file context request
    async fn handle_track_file(
        &self,
        req: TrackFileContextRequest,
    ) -> Result<TrackFileContextResponse> {
        let source = match req.source {
            FileContextEventType::Read => RecordSource::ReadTool,
            FileContextEventType::Write => RecordSource::WriteTool,
            FileContextEventType::DiffApply => RecordSource::DiffApply,
            FileContextEventType::Mention => RecordSource::Mention,
            FileContextEventType::UserEdit => RecordSource::UserEdited,
            FileContextEventType::RooEdit => RecordSource::RooEdited,
        };
        
        let mut tracker = self.context_tracker.write().await;
        match tracker.track_file_context(req.file_path, source).await {
            Ok(_) => Ok(TrackFileContextResponse {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TrackFileContextResponse {
                success: false,
                error: Some(e),
            }),
        }
    }
    
    /// Handle get stale files request
    async fn handle_get_stale_files(
        &self,
        req: GetStaleFilesRequest,
    ) -> Result<GetStaleFilesResponse> {
        let tracker = self.context_tracker.read().await;
        let metadata = tracker.get_task_metadata(&req.task_id).await;
        
        let stale_files: Vec<String> = metadata
            .files_in_context
            .into_iter()
            .filter(|entry| {
                entry.record_state == crate::core::context_tracking::RecordState::Stale
            })
            .map(|entry| entry.path)
            .collect();
        
        Ok(GetStaleFilesResponse { stale_files })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_truncate_conversation() {
        let temp_dir = TempDir::new().unwrap();
        let handler = ContextRouteHandler::new(
            temp_dir.path().to_path_buf(),
            "test-task".to_string(),
        );
        
        // Create a simple conversation
        let messages = vec![
            serde_json::json!({
                "role": "user",
                "content": "Hello",
            }),
            serde_json::json!({
                "role": "assistant",
                "content": "Hi there!",
            }),
        ];
        
        let req = TruncateConversationRequest {
            messages,
            model_id: "claude-3-5-sonnet-20241022".to_string(),
            context_window: 200000,
            max_tokens: None,
            reserved_output_tokens: None,
        };
        
        let command = ContextCommand::TruncateConversation(req);
        let response = handler.handle_command(command).await;
        
        match response {
            ContextResponse::TruncateConversation(resp) => {
                assert_eq!(resp.messages.len(), 2);
                assert!(resp.prev_context_tokens > 0);
            }
            ContextResponse::Error { message } => {
                panic!("Expected success, got error: {}", message);
            }
            _ => panic!("Expected TruncateConversation response"),
        }
    }
    
    #[tokio::test]
    async fn test_track_file_context() {
        let temp_dir = TempDir::new().unwrap();
        let task_dir = temp_dir.path().join(".roo-task");
        std::fs::create_dir(&task_dir).unwrap();
        
        let handler = ContextRouteHandler::new(
            temp_dir.path().to_path_buf(),
            "test-task".to_string(),
        );
        
        let req = TrackFileContextRequest {
            file_path: "src/main.rs".to_string(),
            source: FileContextEventType::Read,
        };
        
        let command = ContextCommand::TrackFileContext(req);
        let response = handler.handle_command(command).await;
        
        match response {
            ContextResponse::TrackFileContext(resp) => {
                assert!(resp.success);
                assert!(resp.error.is_none());
            }
            _ => panic!("Expected TrackFileContext response"),
        }
    }
    
    #[tokio::test]
    async fn test_get_stale_files() {
        let temp_dir = TempDir::new().unwrap();
        let task_dir = temp_dir.path().join(".roo-task");
        std::fs::create_dir(&task_dir).unwrap();
        
        let handler = ContextRouteHandler::new(
            temp_dir.path().to_path_buf(),
            "test-task".to_string(),
        );
        
        // Track a file twice to make it stale
        let track_req = TrackFileContextRequest {
            file_path: "src/lib.rs".to_string(),
            source: FileContextEventType::Read,
        };
        
        handler.handle_command(ContextCommand::TrackFileContext(track_req.clone())).await;
        handler.handle_command(ContextCommand::TrackFileContext(track_req)).await;
        
        // Get stale files
        let req = GetStaleFilesRequest {
            task_id: "test-task".to_string(),
        };
        
        let command = ContextCommand::GetStaleFiles(req);
        let response = handler.handle_command(command).await;
        
        match response {
            ContextResponse::GetStaleFiles(resp) => {
                assert_eq!(resp.stale_files.len(), 1);
                assert_eq!(resp.stale_files[0], "src/lib.rs");
            }
            _ => panic!("Expected GetStaleFiles response"),
        }
    }
}
