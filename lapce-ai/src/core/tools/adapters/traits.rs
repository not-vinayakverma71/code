// Adapter traits for tool integrations - P0-Adapters

use async_trait::async_trait;
use anyhow::Result;

/// Base adapter trait for all tool adapters
#[async_trait]
pub trait Adapter: Send + Sync {
    /// Adapter name for identification
    fn name(&self) -> &'static str;
    
    /// Check if adapter is available and ready
    fn is_available(&self) -> bool {
        true
    }
}

/// Event emitter trait for IPC/messaging adapters
#[async_trait]
pub trait EventEmitter: Adapter {
    /// Emit an event as JSON value
    async fn emit_event(&self, event: serde_json::Value) -> Result<()>;
    
    /// Emit event with correlation ID
    async fn emit_correlated(
        &self,
        correlation_id: String,
        event: serde_json::Value,
    ) -> Result<()>;
}

/// Command executor trait for terminal adapters
#[async_trait]
pub trait CommandExecutor: Adapter {
    /// Execute a command in the terminal
    async fn execute_command(
        &self,
        command: String,
        args: Vec<String>,
        cwd: Option<String>,
    ) -> Result<()>;
    
    /// Check if terminal is available
    fn has_terminal(&self) -> bool;
}

/// Diff view controller trait for editor integration
#[async_trait]
pub trait DiffController: Adapter {
    /// Open diff view with two files
    async fn open_diff(
        &self,
        left_path: String,
        right_path: String,
        title: Option<String>,
    ) -> Result<()>;
    
    /// Close diff view
    async fn close_diff(&self, left_path: String, right_path: String) -> Result<()>;
    
    /// Save diff to target file
    async fn save_diff(&self, target_path: String, content: String) -> Result<()>;
}

/// Approval handler trait for user interaction
#[async_trait]
pub trait ApprovalHandler: Adapter {
    /// Request user approval
    async fn request_approval(
        &self,
        operation: String,
        details: serde_json::Value,
        timeout_ms: Option<u64>,
    ) -> Result<bool>;
    
    /// Send approval response
    async fn send_approval_response(
        &self,
        correlation_id: String,
        approved: bool,
        reason: Option<String>,
    ) -> Result<()>;
}
