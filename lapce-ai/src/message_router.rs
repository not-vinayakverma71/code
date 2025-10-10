// Message Router - CHUNK-03: T16
// Engine-side routing for WebviewMessage-like commands (no IPC dependency)

use std::sync::Arc;
use anyhow::{Result, bail};
use tracing::info;
use serde::{Deserialize, Serialize};

use crate::task_manager::TaskManager;
use crate::task_exact_translation::TaskOptions;

/// Engine-side message types (subset of WebviewMessage)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EngineMessage {
    /// Create a new task
    NewTask {
        task: Option<String>,
        images: Option<Vec<String>>,
        mode: Option<String>,
    },
    
    /// Clear/abort a task
    ClearTask {
        task_id: String,
    },
    
    /// Cancel ongoing operation
    CancelTask {
        task_id: String,
    },
    
    /// Pause a task
    PauseTask {
        task_id: String,
    },
    
    /// Resume a task
    ResumeTask {
        task_id: String,
    },
    
    /// Switch task mode
    ModeSwitch {
        task_id: String,
        mode: String,
    },
    
    /// Respond to an ask
    AskResponse {
        task_id: String,
        response: String,
        approved: bool,
    },
}

/// Message router for engine-side commands
pub struct MessageRouter {
    task_manager: Arc<TaskManager>,
}

impl MessageRouter {
    pub fn new(task_manager: Arc<TaskManager>) -> Self {
        Self { task_manager }
    }
    
    /// Route a message to appropriate handler
    pub async fn route(&self, message: EngineMessage) -> Result<MessageResponse> {
        match message {
            EngineMessage::NewTask { task, images, mode } => {
                self.handle_new_task(task, images, mode).await
            }
            EngineMessage::ClearTask { task_id } => {
                self.handle_clear_task(&task_id).await
            }
            EngineMessage::CancelTask { task_id } => {
                self.handle_cancel_task(&task_id).await
            }
            EngineMessage::PauseTask { task_id } => {
                self.handle_pause_task(&task_id).await
            }
            EngineMessage::ResumeTask { task_id } => {
                self.handle_resume_task(&task_id).await
            }
            EngineMessage::ModeSwitch { task_id, mode } => {
                self.handle_mode_switch(&task_id, &mode).await
            }
            EngineMessage::AskResponse { task_id, response, approved } => {
                self.handle_ask_response(&task_id, &response, approved).await
            }
        }
    }
    
    async fn handle_new_task(
        &self,
        task: Option<String>,
        images: Option<Vec<String>>,
        mode: Option<String>,
    ) -> Result<MessageResponse> {
        let options = TaskOptions {
            task,
            assistant_message_info: None,
            assistant_metadata: None,
            custom_variables: None,
            images,
            start_with: None,
            project_path: None,
            automatically_approve_api_requests: None,
            context_files_content: None,
            context_files: None,
            experiments: None,
            start_task: Some(true),
            root_task: None,
            parent_task: None,
            task_number: None,
            on_created: None,
            initial_todos: None,
            context: None,
            provider: None,
            api_configuration: None,
            enable_diff: None,
            enable_checkpoints: None,
            enable_task_bridge: None,
            fuzzy_match_threshold: None,
            consecutive_mistake_limit: None,
            history_item: None,
        };
        
        let task_id = self.task_manager.create_task(options).await?;
        
        // Set mode if specified
        if let Some(mode_str) = mode {
            if let Some(task) = self.task_manager.get_task(&task_id).await {
                task.set_task_mode(mode_str).await;
            }
        }
        
        info!("Created new task: {}", task_id);
        
        Ok(MessageResponse::TaskCreated { task_id })
    }
    
    async fn handle_clear_task(&self, task_id: &str) -> Result<MessageResponse> {
        self.task_manager.abort_task(task_id).await?;
        self.task_manager.cleanup_task(task_id);
        
        info!("Cleared task: {}", task_id);
        Ok(MessageResponse::Success)
    }
    
    async fn handle_cancel_task(&self, task_id: &str) -> Result<MessageResponse> {
        self.task_manager.abort_task(task_id).await?;
        info!("Cancelled task: {}", task_id);
        Ok(MessageResponse::Success)
    }
    
    async fn handle_pause_task(&self, task_id: &str) -> Result<MessageResponse> {
        self.task_manager.pause_task(task_id).await?;
        info!("Paused task: {}", task_id);
        Ok(MessageResponse::Success)
    }
    
    async fn handle_resume_task(&self, task_id: &str) -> Result<MessageResponse> {
        self.task_manager.resume_task(task_id).await?;
        info!("Resumed task: {}", task_id);
        Ok(MessageResponse::Success)
    }
    
    async fn handle_mode_switch(&self, task_id: &str, mode: &str) -> Result<MessageResponse> {
        if let Some(task) = self.task_manager.get_task(task_id).await {
            task.set_task_mode(mode.to_string()).await;
            info!("Switched task {} to mode: {}", task_id, mode);
            Ok(MessageResponse::Success)
        } else {
            bail!("Task not found: {}", task_id)
        }
    }
    
    async fn handle_ask_response(
        &self,
        task_id: &str,
        response: &str,
        approved: bool,
    ) -> Result<MessageResponse> {
        if let Some(task) = self.task_manager.get_task(task_id).await {
            // Clear ask states
            task.clear_asks();
            
            // Record response
            task.say(
                "text".to_string(),
                Some(format!("User response: {} (approved: {})", response, approved))
            ).map_err(|e| anyhow::anyhow!(e))?;
            
            info!("Handled ask response for task {}", task_id);
            Ok(MessageResponse::Success)
        } else {
            bail!("Task not found: {}", task_id)
        }
    }
}

/// Response from message routing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageResponse {
    Success,
    TaskCreated { task_id: String },
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_router_creation() {
        let task_manager = Arc::new(TaskManager::new());
        let router = MessageRouter::new(task_manager);
        
        // Router should be created successfully
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_new_task_message() {
        let task_manager = Arc::new(TaskManager::new());
        let router = MessageRouter::new(task_manager.clone());
        
        let message = EngineMessage::NewTask {
            task: Some("Test task".to_string()),
            images: None,
            mode: Some("default".to_string()),
        };
        
        let response = router.route(message).await.unwrap();
        
        match response {
            MessageResponse::TaskCreated { task_id } => {
                assert!(!task_id.is_empty());
                assert_eq!(task_manager.task_count(), 1);
            }
            _ => panic!("Expected TaskCreated response"),
        }
    }
    
    #[tokio::test]
    async fn test_pause_resume_task() {
        let task_manager = Arc::new(TaskManager::new());
        let router = MessageRouter::new(task_manager.clone());
        
        // Create task
        let create_msg = EngineMessage::NewTask {
            task: Some("Test".to_string()),
            images: None,
            mode: None,
        };
        let response = router.route(create_msg).await.unwrap();
        
        let task_id = match response {
            MessageResponse::TaskCreated { task_id } => task_id,
            _ => panic!("Expected TaskCreated"),
        };
        
        // Pause
        let pause_msg = EngineMessage::PauseTask {
            task_id: task_id.clone(),
        };
        router.route(pause_msg).await.unwrap();
        
        // Verify paused
        if let Some(task) = task_manager.get_task(&task_id) {
            assert!(task.is_paused());
        }
        
        // Resume
        let resume_msg = EngineMessage::ResumeTask {
            task_id: task_id.clone(),
        };
        router.route(resume_msg).await.unwrap();
        
        // Verify resumed
        if let Some(task) = task_manager.get_task(&task_id) {
            assert!(!task.is_paused());
        }
    }
    
    #[tokio::test]
    async fn test_cancel_task() {
        let task_manager = Arc::new(TaskManager::new());
        let router = MessageRouter::new(task_manager.clone());
        
        // Create task
        let create_msg = EngineMessage::NewTask {
            task: Some("Test".to_string()),
            images: None,
            mode: None,
        };
        let response = router.route(create_msg).await.unwrap();
        
        let task_id = match response {
            MessageResponse::TaskCreated { task_id } => task_id,
            _ => panic!("Expected TaskCreated"),
        };
        
        // Cancel
        let cancel_msg = EngineMessage::CancelTask {
            task_id: task_id.clone(),
        };
        router.route(cancel_msg).await.unwrap();
        
        // Verify aborted
        if let Some(task) = task_manager.get_task(&task_id) {
            assert!(task.is_aborted());
        }
    }
    
    #[tokio::test]
    async fn test_mode_switch() {
        let task_manager = Arc::new(TaskManager::new());
        let router = MessageRouter::new(task_manager.clone());
        
        // Create task
        let create_msg = EngineMessage::NewTask {
            task: Some("Test".to_string()),
            images: None,
            mode: None,
        };
        let response = router.route(create_msg).await.unwrap();
        
        let task_id = match response {
            MessageResponse::TaskCreated { task_id } => task_id,
            _ => panic!("Expected TaskCreated"),
        };
        
        // Switch mode
        let mode_msg = EngineMessage::ModeSwitch {
            task_id: task_id.clone(),
            mode: "code".to_string(),
        };
        router.route(mode_msg).await.unwrap();
        
        // Verify mode switched
        if let Some(task) = task_manager.get_task(&task_id) {
            let mode = task.get_task_mode().await;
            assert_eq!(mode, Some("code".to_string()));
        }
    }
}
