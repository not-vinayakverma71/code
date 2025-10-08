// P0-3: Tool execution handlers for Lapce integration
// Bridges IPC messages to Lapce internal commands

use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

use crate::ipc_messages::{
    ToolIpcMessage, ToolExecutionStatus, CommandExecutionStatusMessage,
    DiffOperationMessage, ToolApprovalRequest, ToolApprovalResponse,
    IpcOrigin, StreamType,
};

/// Handler for tool execution messages from AI backend
pub struct ToolExecutionHandler {
    /// Channel to send internal commands to Lapce UI
    internal_command_tx: mpsc::Sender<LapceInternalCommand>,
    
    /// Channel to receive approval responses from UI
    approval_rx: mpsc::Receiver<ToolApprovalResponse>,
    
    /// Channel to send approval requests to UI
    approval_tx: mpsc::Sender<ToolApprovalRequest>,
    
    /// Active tool executions
    active_executions: Arc<dashmap::DashMap<String, ExecutionState>>,
}

/// Internal commands that can be sent to Lapce
#[derive(Debug, Clone)]
pub enum LapceInternalCommand {
    OpenDiffFiles {
        left_path: PathBuf,
        right_path: PathBuf,
    },
    ExecuteProcess {
        program: String,
        arguments: Vec<String>,
    },
    OpenTerminal {
        command: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
    },
    ShowNotification {
        title: String,
        message: String,
        level: NotificationLevel,
    },
    ShowApprovalDialog {
        request: ToolApprovalRequest,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// State of an active tool execution
#[derive(Debug, Clone)]
struct ExecutionState {
    pub execution_id: String,
    pub tool_name: String,
    pub start_time: std::time::Instant,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone)]
enum ExecutionStatus {
    Running,
    WaitingApproval,
    Completed,
    Failed,
}

impl ToolExecutionHandler {
    pub fn new(
        internal_command_tx: mpsc::Sender<LapceInternalCommand>,
    ) -> (Self, mpsc::Sender<ToolApprovalRequest>) {
        let (approval_tx, approval_rx) = mpsc::channel(32);
        
        let handler = Self {
            internal_command_tx,
            approval_rx,
            approval_tx: approval_tx.clone(),
            active_executions: Arc::new(dashmap::DashMap::new()),
        };
        
        (handler, approval_tx)
    }
    
    /// Process incoming tool IPC message
    pub async fn handle_message(&mut self, message: ToolIpcMessage) -> Result<()> {
        match message {
            ToolIpcMessage::ToolExecutionStatus { origin: _, data } => {
                self.handle_tool_status(data).await?;
            }
            ToolIpcMessage::CommandExecutionStatus { origin: _, data } => {
                self.handle_command_status(data).await?;
            }
            ToolIpcMessage::DiffOperation { origin: _, data } => {
                self.handle_diff_operation(data).await?;
            }
            ToolIpcMessage::ToolApprovalRequest { origin: _, data } => {
                self.handle_approval_request(data).await?;
            }
            ToolIpcMessage::ToolApprovalResponse { origin: _, data } => {
                self.handle_approval_response(data).await?;
            }
        }
        Ok(())
    }
    
    /// Handle tool execution status updates
    async fn handle_tool_status(&mut self, status: ToolExecutionStatus) -> Result<()> {
        match status {
            ToolExecutionStatus::Started { execution_id, tool_name, timestamp } => {
                info!("Tool execution started: {} ({})", tool_name, execution_id);
                
                // Track execution
                self.active_executions.insert(
                    execution_id.clone(),
                    ExecutionState {
                        execution_id: execution_id.clone(),
                        tool_name: tool_name.clone(),
                        start_time: std::time::Instant::now(),
                        status: ExecutionStatus::Running,
                    },
                );
                
                // Show notification
                self.send_notification(
                    format!("Tool: {}", tool_name),
                    format!("Execution started (ID: {})", &execution_id[..8]),
                    NotificationLevel::Info,
                ).await?;
            }
            
            ToolExecutionStatus::Progress { execution_id, message, percentage } => {
                debug!("Tool progress: {} - {}", execution_id, message);
                
                if let Some(pct) = percentage {
                    // Could update a progress bar in UI
                    debug!("Progress: {}%", pct);
                }
            }
            
            ToolExecutionStatus::Completed { execution_id, result, duration_ms } => {
                info!("Tool completed: {} ({}ms)", execution_id, duration_ms);
                
                // Update state
                if let Some(mut state) = self.active_executions.get_mut(&execution_id) {
                    state.status = ExecutionStatus::Completed;
                }
                
                // Show success notification
                self.send_notification(
                    "Tool Execution Complete",
                    format!("Completed in {}ms", duration_ms),
                    NotificationLevel::Success,
                ).await?;
            }
            
            ToolExecutionStatus::Failed { execution_id, error, duration_ms } => {
                error!("Tool failed: {} - {}", execution_id, error);
                
                // Update state
                if let Some(mut state) = self.active_executions.get_mut(&execution_id) {
                    state.status = ExecutionStatus::Failed;
                }
                
                // Show error notification
                self.send_notification(
                    "Tool Execution Failed",
                    error,
                    NotificationLevel::Error,
                ).await?;
            }
        }
        Ok(())
    }
    
    /// Handle command execution status updates
    async fn handle_command_status(&mut self, status: CommandExecutionStatusMessage) -> Result<()> {
        match status {
            CommandExecutionStatusMessage::Started { execution_id, command, args, cwd } => {
                info!("Command started: {} {:?}", command, args);
                
                // Open a terminal for the command
                self.internal_command_tx.send(
                    LapceInternalCommand::OpenTerminal {
                        command: command.clone(),
                        args,
                        cwd,
                    }
                ).await?;
            }
            
            CommandExecutionStatusMessage::Output { execution_id, stream_type, line, timestamp } => {
                // Log output (could be sent to terminal view)
                match stream_type {
                    StreamType::Stdout => debug!("[{}] stdout: {}", execution_id, line),
                    StreamType::Stderr => warn!("[{}] stderr: {}", execution_id, line),
                }
            }
            
            CommandExecutionStatusMessage::Completed { execution_id, exit_code, duration_ms } => {
                info!("Command completed: {} (exit: {}, {}ms)", execution_id, exit_code, duration_ms);
                
                if exit_code != 0 {
                    self.send_notification(
                        "Command Failed",
                        format!("Exit code: {}", exit_code),
                        NotificationLevel::Warning,
                    ).await?;
                }
            }
            
            CommandExecutionStatusMessage::Timeout { execution_id, duration_ms } => {
                warn!("Command timed out: {} after {}ms", execution_id, duration_ms);
                
                self.send_notification(
                    "Command Timeout",
                    format!("Timed out after {}ms", duration_ms),
                    NotificationLevel::Warning,
                ).await?;
            }
        }
        Ok(())
    }
    
    /// Handle diff operations
    async fn handle_diff_operation(&mut self, operation: DiffOperationMessage) -> Result<()> {
        match operation {
            DiffOperationMessage::OpenDiffFiles { left_path, right_path, title } => {
                info!("Opening diff: {:?} vs {:?}", left_path, right_path);
                
                // Send command to open diff view
                self.internal_command_tx.send(
                    LapceInternalCommand::OpenDiffFiles {
                        left_path,
                        right_path,
                    }
                ).await?;
                
                if let Some(title) = title {
                    self.send_notification(
                        "Diff View",
                        title,
                        NotificationLevel::Info,
                    ).await?;
                }
            }
            
            DiffOperationMessage::DiffSave { file_path, content } => {
                info!("Saving diff changes to {:?}", file_path);
                // This would trigger a file save operation
                // For now just log it
            }
            
            DiffOperationMessage::DiffRevert { file_path } => {
                info!("Reverting diff changes for {:?}", file_path);
                // This would revert changes
            }
            
            DiffOperationMessage::CloseDiff { left_path, right_path } => {
                info!("Closing diff view");
                // This would close the diff tab
            }
        }
        Ok(())
    }
    
    /// Handle approval request
    async fn handle_approval_request(&mut self, request: ToolApprovalRequest) -> Result<()> {
        info!("Approval requested: {} - {}", request.tool_name, request.operation);
        
        // Update execution state
        if let Some(mut state) = self.active_executions.get_mut(&request.execution_id) {
            state.status = ExecutionStatus::WaitingApproval;
        }
        
        // Send to UI for user approval
        self.internal_command_tx.send(
            LapceInternalCommand::ShowApprovalDialog {
                request: request.clone(),
            }
        ).await?;
        
        // The response will come back via handle_approval_response
        Ok(())
    }
    
    /// Handle approval response from user
    async fn handle_approval_response(&mut self, response: ToolApprovalResponse) -> Result<()> {
        info!("Approval response: {} - approved: {}", response.execution_id, response.approved);
        
        // Update execution state
        if let Some(mut state) = self.active_executions.get_mut(&response.execution_id) {
            if response.approved {
                state.status = ExecutionStatus::Running;
            } else {
                state.status = ExecutionStatus::Failed;
            }
        }
        
        // Notify about the decision
        let message = if response.approved {
            "Operation approved"
        } else {
            &response.reason.as_deref().unwrap_or("Operation denied")
        };
        
        self.send_notification(
            "Tool Approval",
            message.to_string(),
            if response.approved { NotificationLevel::Success } else { NotificationLevel::Warning },
        ).await?;
        
        Ok(())
    }
    
    /// Send notification to UI
    async fn send_notification(
        &self,
        title: String,
        message: String,
        level: NotificationLevel,
    ) -> Result<()> {
        self.internal_command_tx.send(
            LapceInternalCommand::ShowNotification {
                title,
                message,
                level,
            }
        ).await?;
        Ok(())
    }
    
    /// Clean up completed executions periodically
    pub fn cleanup_executions(&self) {
        let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(300); // 5 minutes
        
        self.active_executions.retain(|_, state| {
            match state.status {
                ExecutionStatus::Completed | ExecutionStatus::Failed => {
                    state.start_time > cutoff
                }
                _ => true,
            }
        });
    }
}

/// Integration point with Lapce UI
pub struct LapceToolIntegration {
    handler: Arc<tokio::sync::Mutex<ToolExecutionHandler>>,
    message_rx: mpsc::Receiver<ToolIpcMessage>,
}

impl LapceToolIntegration {
    pub fn new(
        internal_command_tx: mpsc::Sender<LapceInternalCommand>,
    ) -> (Self, mpsc::Sender<ToolIpcMessage>) {
        let (message_tx, message_rx) = mpsc::channel(100);
        let (handler, _approval_tx) = ToolExecutionHandler::new(internal_command_tx);
        
        let integration = Self {
            handler: Arc::new(tokio::sync::Mutex::new(handler)),
            message_rx,
        };
        
        (integration, message_tx)
    }
    
    /// Run the integration loop
    pub async fn run(mut self) {
        info!("Starting Lapce tool integration");
        
        // Cleanup task
        let handler_clone = self.handler.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                handler_clone.lock().await.cleanup_executions();
            }
        });
        
        // Message processing loop
        while let Some(message) = self.message_rx.recv().await {
            let handler = self.handler.clone();
            tokio::spawn(async move {
                if let Err(e) = handler.lock().await.handle_message(message).await {
                    error!("Error handling tool message: {}", e);
                }
            });
        }
        
        info!("Lapce tool integration stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tool_execution_handler() {
        let (tx, mut rx) = mpsc::channel(10);
        let (mut handler, _approval_tx) = ToolExecutionHandler::new(tx);
        
        // Test handling a tool started message
        let status = ToolExecutionStatus::Started {
            execution_id: "test-123".to_string(),
            tool_name: "readFile".to_string(),
            timestamp: 1234567890,
        };
        
        handler.handle_tool_status(status).await.unwrap();
        
        // Should receive a notification
        let cmd = rx.recv().await.unwrap();
        match cmd {
            LapceInternalCommand::ShowNotification { title, .. } => {
                assert!(title.contains("readFile"));
            }
            _ => panic!("Expected notification"),
        }
    }
    
    #[tokio::test]
    async fn test_diff_operation_handler() {
        let (tx, mut rx) = mpsc::channel(10);
        let (mut handler, _) = ToolExecutionHandler::new(tx);
        
        // Test opening diff files
        let op = DiffOperationMessage::OpenDiffFiles {
            left_path: PathBuf::from("/tmp/left.txt"),
            right_path: PathBuf::from("/tmp/right.txt"),
            title: Some("Test Diff".to_string()),
        };
        
        handler.handle_diff_operation(op).await.unwrap();
        
        // Should receive open diff command
        let cmd = rx.recv().await.unwrap();
        match cmd {
            LapceInternalCommand::OpenDiffFiles { left_path, right_path } => {
                assert_eq!(left_path, PathBuf::from("/tmp/left.txt"));
                assert_eq!(right_path, PathBuf::from("/tmp/right.txt"));
            }
            _ => panic!("Expected OpenDiffFiles command"),
        }
        
        // Should also receive notification with title
        let cmd = rx.recv().await.unwrap();
        match cmd {
            LapceInternalCommand::ShowNotification { message, .. } => {
                assert_eq!(message, "Test Diff");
            }
            _ => panic!("Expected notification"),
        }
    }
    
    #[tokio::test]
    async fn test_command_execution_handler() {
        let (tx, mut rx) = mpsc::channel(10);
        let (mut handler, _) = ToolExecutionHandler::new(tx);
        
        // Test command started
        let status = CommandExecutionStatusMessage::Started {
            execution_id: "cmd-456".to_string(),
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            cwd: Some(PathBuf::from("/tmp")),
        };
        
        handler.handle_command_status(status).await.unwrap();
        
        // Should receive open terminal command
        let cmd = rx.recv().await.unwrap();
        match cmd {
            LapceInternalCommand::OpenTerminal { command, args, cwd } => {
                assert_eq!(command, "echo");
                assert_eq!(args, vec!["hello"]);
                assert_eq!(cwd, Some(PathBuf::from("/tmp")));
            }
            _ => panic!("Expected OpenTerminal command"),
        }
    }
}
