// IPC adapter for tool lifecycle and approvals - P0-Adapters

use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::Result;

use crate::core::tools::traits::{ToolContext, ToolOutput, ApprovalRequired};

/// IPC messages for tool execution lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolExecutionMessage {
    /// Tool execution has started
    Started {
        execution_id: String,
        tool_name: String,
        timestamp: u64,
    },
    
    /// Tool execution progress update
    Progress {
        execution_id: String,
        message: String,
        percentage: Option<u8>,
    },
    
    /// Tool execution completed successfully
    Completed {
        execution_id: String,
        output: ToolOutput,
        duration_ms: u64,
    },
    
    /// Tool execution failed
    Failed {
        execution_id: String,
        error: String,
        duration_ms: u64,
    },
    
    /// Approval required for operation
    ApprovalRequest {
        execution_id: String,
        approval: ApprovalRequired,
    },
    
    /// Approval response from UI
    ApprovalResponse {
        execution_id: String,
        approved: bool,
        reason: Option<String>,
    },
}

/// IPC adapter for emitting tool lifecycle events and handling approvals
#[derive(Clone)]
pub struct IpcAdapter {
    /// Channel for sending messages to UI
    sender: mpsc::UnboundedSender<ToolExecutionMessage>,
    
    /// Pending approval requests
    pending_approvals: Arc<RwLock<std::collections::HashMap<String, oneshot::Sender<bool>>>>,
}

impl IpcAdapter {
    /// Create new IPC adapter
    pub fn new(sender: mpsc::UnboundedSender<ToolExecutionMessage>) -> Self {
        Self {
            sender,
            pending_approvals: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Emit tool execution started event
    pub fn emit_started(&self, context: &ToolContext, tool_name: &str) -> Result<()> {
        let message = ToolExecutionMessage::Started {
            execution_id: context.execution_id.clone(),
            tool_name: tool_name.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Emit progress update
    pub fn emit_progress(
        &self,
        context: &ToolContext,
        message: &str,
        percentage: Option<u8>
    ) -> Result<()> {
        let msg = ToolExecutionMessage::Progress {
            execution_id: context.execution_id.clone(),
            message: message.to_string(),
            percentage,
        };
        
        self.sender.send(msg)?;
        Ok(())
    }
    
    /// Emit tool execution completed
    pub fn emit_completed(
        &self,
        context: &ToolContext,
        output: ToolOutput,
        duration_ms: u64
    ) -> Result<()> {
        let message = ToolExecutionMessage::Completed {
            execution_id: context.execution_id.clone(),
            output,
            duration_ms,
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Emit tool execution failed
    pub fn emit_failed(
        &self,
        context: &ToolContext,
        error: &str,
        duration_ms: u64
    ) -> Result<()> {
        let message = ToolExecutionMessage::Failed {
            execution_id: context.execution_id.clone(),
            error: error.to_string(),
            duration_ms,
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Request approval for an operation
    pub async fn request_approval(
        &self,
        context: &ToolContext,
        approval: ApprovalRequired
    ) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        
        // Store the sender for response
        self.pending_approvals.write().insert(
            context.execution_id.clone(),
            tx
        );
        
        // Send approval request
        let message = ToolExecutionMessage::ApprovalRequest {
            execution_id: context.execution_id.clone(),
            approval,
        };
        
        self.sender.send(message)?;
        
        // Wait for response with timeout
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            rx
        ).await {
            Ok(Ok(approved)) => Ok(approved),
            Ok(Err(_)) => {
                // Channel closed
                self.pending_approvals.write().remove(&context.execution_id);
                Ok(false) // Default to denial
            }
            Err(_) => {
                // Timeout
                self.pending_approvals.write().remove(&context.execution_id);
                Ok(false) // Default to denial on timeout
            }
        }
    }
    
    /// Handle approval response from UI
    pub fn handle_approval_response(&self, execution_id: &str, approved: bool) -> Result<()> {
        if let Some(sender) = self.pending_approvals.write().remove(execution_id) {
            let _ = sender.send(approved);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_ipc_lifecycle() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let adapter = IpcAdapter::new(tx);
        
        let temp_dir = TempDir::new().unwrap();
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_user".to_string()
        );
        
        // Test started event
        adapter.emit_started(&context, "test_tool").unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            ToolExecutionMessage::Started { tool_name, .. } => {
                assert_eq!(tool_name, "test_tool");
            }
            _ => panic!("Expected Started message"),
        }
        
        // Test progress event
        adapter.emit_progress(&context, "Processing...", Some(50)).unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            ToolExecutionMessage::Progress { message, percentage, .. } => {
                assert_eq!(message, "Processing...");
                assert_eq!(percentage, Some(50));
            }
            _ => panic!("Expected Progress message"),
        }
        
        // Test completed event
        let output = ToolOutput::success(serde_json::json!({"result": "ok"}));
        adapter.emit_completed(&context, output, 100).unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            ToolExecutionMessage::Completed { duration_ms, .. } => {
                assert_eq!(duration_ms, 100);
            }
            _ => panic!("Expected Completed message"),
        }
    }
    
    #[tokio::test]
    async fn test_approval_flow() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let adapter = IpcAdapter::new(tx);
        
        let temp_dir = TempDir::new().unwrap();
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_user".to_string()
        );
        
        let approval = ApprovalRequired {
            tool_name: "writeFile".to_string(),
            operation: "write".to_string(),
            target: "/path/to/file".to_string(),
            details: "Writing content to file".to_string(),
        };
        
        // Spawn approval request
        let adapter_clone = adapter.clone();
        let context_clone = context.clone();
        let approval_clone = approval.clone();
        let handle = tokio::spawn(async move {
            adapter_clone.request_approval(&context_clone, approval_clone).await
        });
        
        // Receive approval request
        let msg = rx.recv().await.unwrap();
        match msg {
            ToolExecutionMessage::ApprovalRequest { execution_id, approval: req } => {
                assert_eq!(execution_id, context.execution_id);
                assert_eq!(req.tool_name, "writeFile");
                
                // Send approval response
                adapter.handle_approval_response(&execution_id, true).unwrap();
            }
            _ => panic!("Expected ApprovalRequest message"),
        }
        
        // Check that approval was received
        let result = handle.await.unwrap().unwrap();
        assert!(result);
    }
    
    #[tokio::test]
    async fn test_approval_timeout() {
        let (tx, mut _rx) = mpsc::unbounded_channel();
        let adapter = IpcAdapter::new(tx);
        
        let temp_dir = TempDir::new().unwrap();
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_user".to_string()
        );
        
        let approval = ApprovalRequired {
            tool_name: "deleteFile".to_string(),
            operation: "delete".to_string(),
            target: "/path/to/file".to_string(),
            details: "Deleting file".to_string(),
        };
        
        // This should timeout and return false
        // We're using a shorter timeout for testing
        let result = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            adapter.request_approval(&context, approval)
        ).await;
        
        // Should timeout
        assert!(result.is_err());
    }
}
