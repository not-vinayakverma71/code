/// Message Dispatcher - Routes messages from bridge to AI components
/// 
/// Architecture:
/// Bridge → IPC → Dispatcher → Tools/Providers → Response → IPC → Bridge → UI

use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// TODO: Import from lapce-app once bridge messages are exposed
// For now, redefine the core message types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum InboundMessage {
    NewTask {
        text: String,
        images: Vec<String>,
        model: Option<String>,
        mode: Option<String>,
    },
    AskResponse {
        ask_ts: u64,
        response: AskResponseType,
    },
    CancelTask,
    UpdateSettings {
        settings: HashMap<String, JsonValue>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AskResponseType {
    Approve,
    Reject,
    MessageResponse { text: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OutboundMessage {
    TextStreamChunk {
        text: String,
        index: u64,
    },
    Ask {
        question: String,
        ask_ts: u64,
        ask_type: String,
    },
    TaskComplete {
        success: bool,
        message: Option<String>,
    },
    Error {
        message: String,
    },
    ConnectionStatus {
        status: String,
    },
}

/// Central dispatcher that routes messages to appropriate handlers
pub struct MessageDispatcher {
    // Tool execution system (MCP tools, file ops, etc)
    // tool_executor: Arc<RwLock<ToolExecutor>>,
    
    // AI provider interface
    // provider_manager: Arc<RwLock<ProviderManager>>,
    
    // Active tasks tracking
    active_tasks: Arc<RwLock<HashMap<String, TaskState>>>,
    
    // Settings
    settings: Arc<RwLock<HashMap<String, JsonValue>>>,
}

#[derive(Debug, Clone)]
struct TaskState {
    task_id: String,
    status: TaskStatus,
    model: String,
    mode: String,
}

#[derive(Debug, Clone)]
enum TaskStatus {
    Processing,
    WaitingForApproval,
    Completed,
    Failed,
}

impl MessageDispatcher {
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            settings: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Main entry point: Route incoming message to appropriate handler
    pub async fn dispatch(&self, message: InboundMessage) -> Result<Vec<OutboundMessage>> {
        match message {
            InboundMessage::NewTask { text, images, model, mode } => {
                self.handle_new_task(text, images, model, mode).await
            }
            InboundMessage::AskResponse { ask_ts, response } => {
                self.handle_ask_response(ask_ts, response).await
            }
            InboundMessage::CancelTask => {
                self.handle_cancel_task().await
            }
            InboundMessage::UpdateSettings { settings } => {
                self.handle_update_settings(settings).await
            }
        }
    }
    
    /// Handle new task from user
    async fn handle_new_task(
        &self,
        text: String,
        images: Vec<String>,
        model: Option<String>,
        mode: Option<String>,
    ) -> Result<Vec<OutboundMessage>> {
        let task_id = format!("task_{}", uuid::Uuid::new_v4());
        let model = model.unwrap_or_else(|| "claude-sonnet-4".to_string());
        let mode = mode.unwrap_or_else(|| "Code".to_string());
        
        eprintln!("[DISPATCHER] New task: {} (model={}, mode={})", task_id, model, mode);
        
        // Create task state
        let task_state = TaskState {
            task_id: task_id.clone(),
            status: TaskStatus::Processing,
            model: model.clone(),
            mode: mode.clone(),
        };
        
        self.active_tasks.write().await.insert(task_id.clone(), task_state);
        
        // TODO Phase C: Route to actual AI provider
        // For now, return a stub response
        let responses = vec![
            OutboundMessage::TextStreamChunk {
                text: format!("Received: {}", text),
                index: 0,
            },
            OutboundMessage::TaskComplete {
                success: true,
                message: Some("Task dispatched (stub)".to_string()),
            },
        ];
        
        Ok(responses)
    }
    
    /// Handle user response to an ask prompt
    async fn handle_ask_response(
        &self,
        ask_ts: u64,
        response: AskResponseType,
    ) -> Result<Vec<OutboundMessage>> {
        eprintln!("[DISPATCHER] Ask response: ts={}, type={:?}", ask_ts, response);
        
        // TODO: Route to appropriate task handler
        Ok(vec![])
    }
    
    /// Handle task cancellation
    async fn handle_cancel_task(&self) -> Result<Vec<OutboundMessage>> {
        eprintln!("[DISPATCHER] Cancel task");
        
        // TODO: Cancel active task
        Ok(vec![
            OutboundMessage::TaskComplete {
                success: false,
                message: Some("Task cancelled".to_string()),
            }
        ])
    }
    
    /// Handle settings update
    async fn handle_update_settings(
        &self,
        settings: HashMap<String, JsonValue>,
    ) -> Result<Vec<OutboundMessage>> {
        eprintln!("[DISPATCHER] Update settings: {} keys", settings.len());
        
        *self.settings.write().await = settings;
        
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dispatcher_new_task() {
        let dispatcher = MessageDispatcher::new();
        
        let message = InboundMessage::NewTask {
            text: "Hello AI".to_string(),
            images: vec![],
            model: None,
            mode: None,
        };
        
        let responses = dispatcher.dispatch(message).await.unwrap();
        
        assert!(!responses.is_empty());
        assert!(matches!(responses.last().unwrap(), OutboundMessage::TaskComplete { .. }));
    }
    
    #[tokio::test]
    async fn test_dispatcher_cancel() {
        let dispatcher = MessageDispatcher::new();
        
        let responses = dispatcher.dispatch(InboundMessage::CancelTask).await.unwrap();
        
        assert!(!responses.is_empty());
    }
}
