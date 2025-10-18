// API Provider Integration - Streaming chunks to AssistantMessageParser
// Connects external API providers (Anthropic, OpenAI, etc.) to the orchestrator

use std::sync::Arc;
use anyhow::{Result, Context};
use tokio::sync::mpsc;
use tracing::{info, debug, warn, error};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::assistant_message_parser::{AssistantMessageParser, StreamChunk};
use crate::task_exact_translation::{Task, ApiMessage, AssistantMessageContent};
use crate::backoff_util::RetryExecutor;
use crate::task_orchestrator_metrics::global_metrics;

/// API Provider type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiProvider {
    Anthropic,
    OpenAI,
    Gemini,
    Ollama,
}

/// API streaming event from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApiStreamEvent {
    /// Text delta
    TextDelta { text: String },
    
    /// Tool use start
    ToolUseStart { 
        id: String,
        name: String,
    },
    
    /// Tool use input delta
    InputJsonDelta { partial_json: String },
    
    /// Tool use end
    ToolUseEnd,
    
    /// Stream complete
    MessageStop,
    
    /// Error occurred
    Error { message: String },
}

/// API request configuration
#[derive(Debug, Clone)]
pub struct ApiRequest {
    pub provider: ApiProvider,
    pub model: String,
    pub messages: Vec<ApiMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

/// API response handler
pub struct ApiResponseHandler {
    parser: AssistantMessageParser,
    task: Arc<Task>,
}

impl ApiResponseHandler {
    pub fn new(task: Arc<Task>) -> Self {
        Self {
            parser: AssistantMessageParser::new(),
            task,
        }
    }
    
    /// Handle a streaming event from API
    pub async fn handle_stream_event(&mut self, event: ApiStreamEvent) -> Result<()> {
        match event {
            ApiStreamEvent::TextDelta { text } => {
                debug!("Received text delta: {} chars", text.len());
                self.parser.process_chunk(StreamChunk::Text(text.clone()))?;
                
                // Send partial update to task
                self.task.say_partial(text)?;
                
                Ok(())
            }
            
            ApiStreamEvent::ToolUseStart { id, name } => {
                debug!("Tool use started: {} ({})", name, id);
                self.parser.process_chunk(StreamChunk::ToolUseStart {
                    name: name.clone(),
                    id: Some(id),
                })?;
                
                // Track tool usage
                self.task.track_tool_usage(&name);
                global_metrics().record_tool_invocation(&name);
                
                Ok(())
            }
            
            ApiStreamEvent::InputJsonDelta { partial_json } => {
                debug!("Tool input delta: {} chars", partial_json.len());
                self.parser.process_chunk(StreamChunk::ToolUseInput(partial_json))?;
                Ok(())
            }
            
            ApiStreamEvent::ToolUseEnd => {
                debug!("Tool use ended");
                self.parser.process_chunk(StreamChunk::ToolUseEnd)?;
                Ok(())
            }
            
            ApiStreamEvent::MessageStop => {
                info!("Stream completed");
                self.parser.process_chunk(StreamChunk::EndOfStream)?;
                
                // Finalize any partial messages
                self.task.finalize_partial()?;
                
                Ok(())
            }
            
            ApiStreamEvent::Error { message } => {
                error!("API stream error: {}", message);
                self.task.increment_mistakes();
                Err(anyhow::anyhow!("API error: {}", message))
            }
        }
    }
    
    /// Get parsed content blocks
    pub fn get_content(&self) -> Vec<AssistantMessageContent> {
        self.parser.get_content()
    }
    
    /// Finalize and return all content
    pub fn finalize(self) -> Vec<AssistantMessageContent> {
        self.parser.finalize()
    }
}

/// API client for making streaming requests
pub struct ApiClient {
    retry_executor: RetryExecutor,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            retry_executor: RetryExecutor::default(),
        }
    }
    
    /// Make a streaming API request
    /// Returns a channel receiver for streaming events
    pub async fn stream_request(
        &self,
        request: ApiRequest,
    ) -> Result<mpsc::Receiver<ApiStreamEvent>> {
        let (tx, rx) = mpsc::channel(100);
        
        // In production, this would call actual API
        // For now, simulate a streaming response
        tokio::spawn(async move {
            Self::simulate_streaming_response(tx, request).await;
        });
        
        Ok(rx)
    }
    
    /// Simulate a streaming response (placeholder for real API integration)
    async fn simulate_streaming_response(
        tx: mpsc::Sender<ApiStreamEvent>,
        request: ApiRequest,
    ) {
        // Simulate text response
        let chunks = vec![
            "I'll help you with that task. ",
            "Let me start by reading the relevant files.",
        ];
        
        for chunk in chunks {
            if tx.send(ApiStreamEvent::TextDelta {
                text: chunk.to_string(),
            }).await.is_err() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        
        // Simulate tool use
        let _ = tx.send(ApiStreamEvent::ToolUseStart {
            id: "tool_1".to_string(),
            name: "read_file".to_string(),
        }).await;
        
        let _ = tx.send(ApiStreamEvent::InputJsonDelta {
            partial_json: r#"{"path":"#.to_string(),
        }).await;
        
        let _ = tx.send(ApiStreamEvent::InputJsonDelta {
            partial_json: r#"src/main.rs"}"#.to_string(),
        }).await;
        
        let _ = tx.send(ApiStreamEvent::ToolUseEnd).await;
        
        // Complete stream
        let _ = tx.send(ApiStreamEvent::MessageStop).await;
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Integration bridge between API and orchestrator
pub struct ApiOrchestrationBridge {
    client: ApiClient,
}

impl ApiOrchestrationBridge {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
        }
    }
    
    /// Execute an API request and process streaming response
    pub async fn execute_request(
        &self,
        task: Arc<Task>,
        request: ApiRequest,
    ) -> Result<Vec<AssistantMessageContent>> {
        info!("Executing API request for task {}", task.task_id);
        
        // Mark as streaming
        task.set_streaming(true);
        
        // Get streaming channel
        let mut rx = self.client.stream_request(request).await?;
        
        // Create response handler
        let mut handler = ApiResponseHandler::new(task.clone());
        
        // Process stream events
        while let Some(event) = rx.recv().await {
            if let Err(e) = handler.handle_stream_event(event).await {
                error!("Error handling stream event: {}", e);
                task.set_streaming(false);
                return Err(e);
            }
            
            // Check abort flag
            if task.is_aborted() {
                warn!("Task aborted during streaming");
                break;
            }
        }
        
        task.set_streaming(false);
        
        // Get parsed content
        let content = handler.finalize();
        
        info!("API request completed, {} content blocks", content.len());
        Ok(content)
    }
}

impl Default for ApiOrchestrationBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_exact_translation::{TaskOptions, ExtensionContext};
    use std::path::PathBuf;
    use parking_lot::RwLock;
    use std::collections::HashMap;
    
    fn create_test_task() -> Arc<Task> {
        let options = TaskOptions {
            task: Some("Test task".to_string()),
            assistant_message_info: None,
            assistant_metadata: None,
            custom_variables: None,
            images: None,
            start_with: None,
            project_path: None,
            automatically_approve_api_requests: None,
            context_files_content: None,
            context_files: None,
            experiments: None,
            start_task: Some(false),
            root_task: None,
            parent_task: None,
            task_number: None,
            on_created: None,
            initial_todos: None,
            context: Some(ExtensionContext {
                global_storage_uri: PathBuf::from("/tmp"),
                workspace_state: Arc::new(RwLock::new(HashMap::new())),
            }),
            provider: None,
            api_configuration: None,
            enable_diff: None,
            enable_checkpoints: None,
            enable_task_bridge: None,
            fuzzy_match_threshold: None,
            consecutive_mistake_limit: None,
            history_item: None,
        };
        
        Task::new(options)
    }
    
    #[tokio::test]
    async fn test_response_handler_text() {
        let task = create_test_task();
        let mut handler = ApiResponseHandler::new(task);
        
        handler.handle_stream_event(ApiStreamEvent::TextDelta {
            text: "Hello".to_string(),
        }).await.unwrap();
        
        handler.handle_stream_event(ApiStreamEvent::MessageStop).await.unwrap();
        
        let content = handler.get_content();
        assert_eq!(content.len(), 1);
    }
    
    #[tokio::test]
    async fn test_response_handler_tool_use() {
        let task = create_test_task();
        let mut handler = ApiResponseHandler::new(task);
        
        handler.handle_stream_event(ApiStreamEvent::ToolUseStart {
            id: "tool_1".to_string(),
            name: "read_file".to_string(),
        }).await.unwrap();
        
        handler.handle_stream_event(ApiStreamEvent::InputJsonDelta {
            partial_json: r#"{"path":"test.txt"}"#.to_string(),
        }).await.unwrap();
        
        handler.handle_stream_event(ApiStreamEvent::ToolUseEnd).await.unwrap();
        handler.handle_stream_event(ApiStreamEvent::MessageStop).await.unwrap();
        
        let content = handler.get_content();
        assert_eq!(content.len(), 1);
        
        match &content[0] {
            AssistantMessageContent::ToolUse { name, .. } => {
                assert_eq!(name, "read_file");
            }
            _ => panic!("Expected ToolUse"),
        }
    }
    
    #[tokio::test]
    async fn test_api_client_stream() {
        let client = ApiClient::new();
        let request = ApiRequest {
            provider: ApiProvider::Anthropic,
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![],
            max_tokens: Some(1024),
            temperature: Some(0.7),
            system_prompt: None,
        };
        
        let mut rx = client.stream_request(request).await.unwrap();
        
        let mut event_count = 0;
        while let Some(_event) = rx.recv().await {
            event_count += 1;
        }
        
        assert!(event_count > 0);
    }
    
    #[tokio::test]
    async fn test_orchestration_bridge() {
        let bridge = ApiOrchestrationBridge::new();
        let task = create_test_task();
        
        let request = ApiRequest {
            provider: ApiProvider::Anthropic,
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![],
            max_tokens: Some(1024),
            temperature: Some(0.7),
            system_prompt: None,
        };
        
        let content = bridge.execute_request(task, request).await.unwrap();
        assert!(!content.is_empty());
    }
}
