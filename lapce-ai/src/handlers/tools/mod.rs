// Tool execution handlers - P1-1
// Routes IPC messages to appropriate tool implementations

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use serde_json::Value;

use crate::core::tools::{Tool, ToolContext, ToolOutput, ToolError, ToolRegistry};
use crate::ipc::ipc_messages::{ToolExecutionStatus, ApprovalMessage};

// Tool handler modules
pub mod file;
pub mod terminal;
pub mod diff;
pub mod search;

/// Tool execution request from IPC
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_name: String,
    pub params: Value,
    pub workspace_path: String,
    pub user_id: String,
    pub correlation_id: String,
    pub require_approval: bool,
}

/// Tool execution response to IPC
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolExecutionResponse {
    pub correlation_id: String,
    pub status: ToolExecutionStatus,
    pub result: Option<Value>,
    pub error: Option<String>,
}

/// Main tool execution handler
pub struct ToolExecutionHandler {
    registry: Arc<ToolRegistry>,
}

impl ToolExecutionHandler {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(ToolRegistry::new()),
        }
    }
    
    /// Execute a tool and return response
    pub async fn execute(&self, request: ToolExecutionRequest) -> AnyhowResult<ToolExecutionResponse> {
        let start_time = std::time::Instant::now();
        let correlation_id = request.correlation_id.clone();
        
        // Create tool context
        let mut context = ToolContext::new(
            std::path::PathBuf::from(&request.workspace_path),
            request.user_id.clone(),
        );
        context.execution_id = correlation_id.clone();
        context.require_approval = request.require_approval;
        
        // Execute tool
        let result = self.registry.execute(
            &request.tool_name,
            request.params,
            context,
        ).await;
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        // Build response
        match result {
            Ok(tool_output) => {
                let result_value = serde_json::to_value(&tool_output).unwrap_or(serde_json::json!({}));
                Ok(ToolExecutionResponse {
                    correlation_id: correlation_id.clone(),
                    status: ToolExecutionStatus::Completed {
                        correlation_id,
                        result: result_value.clone(),
                        duration_ms,
                    },
                    result: Some(result_value),
                    error: None,
                })
            }
            Err(e) => {
                Ok(ToolExecutionResponse {
                    correlation_id: correlation_id.clone(),
                    status: ToolExecutionStatus::Failed {
                        correlation_id,
                        error: e.to_string(),
                        duration_ms,
                    },
                    result: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }
    
    /// Execute with streaming support
    pub async fn execute_streaming(
        &self,
        request: ToolExecutionRequest,
        progress_tx: tokio::sync::mpsc::UnboundedSender<ToolExecutionStatus>,
    ) -> AnyhowResult<ToolExecutionResponse> {
        let correlation_id = request.correlation_id.clone();
        
        // Send started event
        let _ = progress_tx.send(ToolExecutionStatus::Started {
            tool_name: request.tool_name.clone(),
            correlation_id: correlation_id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        
        // Execute (this will be enhanced with more streaming later)
        self.execute(request).await
    }
}

impl Default for ToolExecutionHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_tool_execution_handler_creation() {
        let handler = ToolExecutionHandler::new();
        assert!(Arc::strong_count(&handler.registry) == 1);
    }
    
    #[tokio::test]
    async fn test_execute_read_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();
        
        let handler = ToolExecutionHandler::new();
        let request = ToolExecutionRequest {
            tool_name: "read_file".to_string(),
            params: serde_json::json!({
                "path": "test.txt"
            }),
            workspace_path: temp_dir.path().to_string_lossy().to_string(),
            user_id: "test_user".to_string(),
            correlation_id: "test-123".to_string(),
            require_approval: false,
        };
        
        let response = handler.execute(request).await.unwrap();
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
}
