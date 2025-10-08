/// AI Bridge for IPC communication - P0-E2E
/// Connects Lapce app with lapce-ai backend for tool execution

use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Internal commands that can be triggered by AI tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InternalCommand {
    /// Open diff viewer with two files
    OpenDiffFiles {
        left_path: PathBuf,   // Original file
        right_path: PathBuf,  // Modified file (temp)
    },
    
    /// Execute a process in terminal
    ExecuteProcess {
        program: String,
        arguments: Vec<String>,
        working_dir: Option<PathBuf>,
    },
    
    /// Show notification to user
    ShowNotification {
        title: String,
        message: String,
        level: NotificationLevel,
    },
    
    /// Request approval from user
    RequestApproval {
        tool_name: String,
        operation: String,
        details: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// AI Bridge for handling IPC messages from AI backend
pub struct AiBridge {
    /// IPC client for communication with AI backend
    ipc_client: Arc<dyn IpcClient>,
    
    /// Handler for internal commands
    command_handler: Arc<CommandHandler>,
    
    /// Current workspace path
    workspace: PathBuf,
}

impl AiBridge {
    pub fn new(
        ipc_client: Arc<dyn IpcClient>,
        command_handler: Arc<CommandHandler>,
        workspace: PathBuf,
    ) -> Self {
        Self {
            ipc_client,
            command_handler,
            workspace,
        }
    }
    
    /// Handle incoming IPC message from AI backend
    pub async fn handle_message(&self, message: IpcMessage) -> Result<IpcResponse> {
        match message {
            IpcMessage::InternalCommand(cmd) => {
                self.handle_internal_command(cmd).await
            }
            IpcMessage::ToolExecution { tool, args } => {
                self.handle_tool_execution(tool, args).await
            }
            IpcMessage::ApprovalRequest { id, details } => {
                self.handle_approval_request(id, details).await
            }
            _ => Ok(IpcResponse::Error {
                message: "Unsupported message type".to_string(),
            }),
        }
    }
    
    /// Handle internal command from AI
    async fn handle_internal_command(&self, cmd: InternalCommand) -> Result<IpcResponse> {
        match cmd {
            InternalCommand::OpenDiffFiles { left_path, right_path } => {
                // Trigger diff viewer in Lapce
                self.command_handler.open_diff_files(left_path, right_path).await?;
                Ok(IpcResponse::Success)
            }
            
            InternalCommand::ExecuteProcess { program, arguments, working_dir } => {
                // Execute in terminal
                let cwd = working_dir.unwrap_or_else(|| self.workspace.clone());
                self.command_handler.execute_process(program, arguments, cwd).await?;
                Ok(IpcResponse::Success)
            }
            
            InternalCommand::ShowNotification { title, message, level } => {
                // Show notification
                self.command_handler.show_notification(title, message, level).await?;
                Ok(IpcResponse::Success)
            }
            
            InternalCommand::RequestApproval { tool_name, operation, details } => {
                // Request user approval
                let approved = self.command_handler.request_approval(tool_name, operation, details).await?;
                Ok(IpcResponse::Approval { approved })
            }
        }
    }
    
    /// Handle tool execution request
    async fn handle_tool_execution(&self, tool: String, args: serde_json::Value) -> Result<IpcResponse> {
        // Forward to AI backend for execution
        let response = self.ipc_client.execute_tool(tool, args).await?;
        Ok(IpcResponse::ToolResult { result: response })
    }
    
    /// Handle approval request
    async fn handle_approval_request(&self, id: String, details: String) -> Result<IpcResponse> {
        let approved = self.command_handler.show_approval_dialog(id, details).await?;
        Ok(IpcResponse::Approval { approved })
    }
}

/// Command handler for executing internal commands
#[async_trait::async_trait]
pub trait CommandHandler: Send + Sync {
    /// Open diff viewer with two files
    async fn open_diff_files(&self, left: PathBuf, right: PathBuf) -> Result<()>;
    
    /// Execute process in terminal
    async fn execute_process(&self, program: String, args: Vec<String>, cwd: PathBuf) -> Result<()>;
    
    /// Show notification
    async fn show_notification(&self, title: String, message: String, level: NotificationLevel) -> Result<()>;
    
    /// Request approval from user
    async fn request_approval(&self, tool: String, operation: String, details: String) -> Result<bool>;
    
    /// Show approval dialog
    async fn show_approval_dialog(&self, id: String, details: String) -> Result<bool>;
}

/// IPC client trait for communication
#[async_trait::async_trait]
pub trait IpcClient: Send + Sync {
    /// Execute tool on AI backend
    async fn execute_tool(&self, tool: String, args: serde_json::Value) -> Result<serde_json::Value>;
    
    /// Send message to AI backend
    async fn send_message(&self, message: IpcMessage) -> Result<IpcResponse>;
}

/// IPC message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    InternalCommand(InternalCommand),
    ToolExecution {
        tool: String,
        args: serde_json::Value,
    },
    ApprovalRequest {
        id: String,
        details: String,
    },
    HealthCheck,
}

/// IPC response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcResponse {
    Success,
    ToolResult {
        result: serde_json::Value,
    },
    Approval {
        approved: bool,
    },
    Error {
        message: String,
    },
}

/// Lapce command handler implementation
pub struct LapceCommandHandler {
    /// Reference to Lapce's internal command system
    internal_commands: Arc<RwLock<InternalCommandSystem>>,
    
    /// Reference to terminal panel
    terminal_panel: Arc<RwLock<TerminalPanel>>,
    
    /// Reference to notification system
    notifications: Arc<RwLock<NotificationSystem>>,
}

impl LapceCommandHandler {
    pub fn new(
        internal_commands: Arc<RwLock<InternalCommandSystem>>,
        terminal_panel: Arc<RwLock<TerminalPanel>>,
        notifications: Arc<RwLock<NotificationSystem>>,
    ) -> Self {
        Self {
            internal_commands,
            terminal_panel,
            notifications,
        }
    }
}

#[async_trait::async_trait]
impl CommandHandler for LapceCommandHandler {
    async fn open_diff_files(&self, left: PathBuf, right: PathBuf) -> Result<()> {
        let commands = self.internal_commands.read().await;
        commands.execute(LapceInternalCommand::OpenDiffFiles {
            left_path: left,
            right_path: right,
        })?;
        Ok(())
    }
    
    async fn execute_process(&self, program: String, args: Vec<String>, cwd: PathBuf) -> Result<()> {
        let terminal = self.terminal_panel.write().await;
        terminal.execute_command(program, args, cwd)?;
        Ok(())
    }
    
    async fn show_notification(&self, title: String, message: String, level: NotificationLevel) -> Result<()> {
        let notifications = self.notifications.write().await;
        notifications.show(title, message, level)?;
        Ok(())
    }
    
    async fn request_approval(&self, tool: String, operation: String, details: String) -> Result<bool> {
        // Show approval dialog
        let notifications = self.notifications.read().await;
        let approved = notifications.show_approval_dialog(
            format!("Tool Approval: {}", tool),
            format!("{}\n\nDetails:\n{}", operation, details),
        ).await?;
        Ok(approved)
    }
    
    async fn show_approval_dialog(&self, id: String, details: String) -> Result<bool> {
        let notifications = self.notifications.read().await;
        let approved = notifications.show_approval_dialog(
            format!("Approval Request: {}", id),
            details,
        ).await?;
        Ok(approved)
    }
}

// Placeholder types for Lapce integration
// These would be replaced with actual Lapce types
struct InternalCommandSystem;
struct TerminalPanel;
struct NotificationSystem;
struct LapceInternalCommand {
    // Lapce's actual internal command type
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ai_bridge_creation() {
        // Test bridge can be created
        // This would need mock implementations for real testing
    }
}
