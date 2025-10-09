// Core traits for tool system - P0-0: Scaffold core layer

use crate::core::tools::permissions::rooignore::RooIgnore;
use crate::core::tools::permissions::PermissionManager;
use crate::core::tools::config::ToolConfig;
use crate::core::tools::logging::{LogContext, log_tool_start, log_tool_complete, log_approval_request, log_file_operation};
use crate::core::permissions::Permission;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use async_trait::async_trait;
use std::time::Instant;

/// Tool permissions
#[derive(Debug, Clone)]
pub struct ToolPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub file_read: bool,
    pub file_write: bool,
    pub network: bool,
    pub command_execute: bool,
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self {
            read: true,
            write: false,
            execute: true,
            file_read: true,
            file_write: false,
            network: false,
            command_execute: false,
        }
    }
}

/// Tool execution context providing workspace info, permissions, and adapters
#[derive(Clone)]
pub struct ToolContext {
    /// Root workspace directory
    pub workspace: PathBuf,
    
    /// User ID for permission checks
    pub user_id: String,
    
    /// Session ID for tracking
    pub session_id: String,
    
    /// Tool execution ID for tracking
    pub execution_id: String,
    
    /// Whether to request approval for destructive operations
    pub require_approval: bool,
    
    /// Dry-run mode (simulate without actual changes)
    pub dry_run: bool,
    
    /// Additional context data
    pub metadata: HashMap<String, Value>,
    
    /// Tool permissions
    pub permissions: ToolPermissions,
    
    /// RooIgnore for path filtering
    pub rooignore: Option<Arc<RooIgnore>>,
    
    /// Permission manager for enforcing permissions
    pub permission_manager: Option<Arc<PermissionManager>>,
    
    /// Tool configuration
    pub config: Arc<ToolConfig>,
    
    /// Logging context for audit trails
    pub log_context: Option<LogContext>,
    
    /// Adapters for external integrations (IPC, diff view, terminal, etc.)
    pub adapters: HashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
    
    /// Event emitter adapters (IPC, webhooks, etc.)
    pub event_emitters: Vec<Arc<dyn crate::core::tools::adapters::EventEmitter>>,
    
    /// Diff controller adapters
    pub diff_controllers: Vec<Arc<dyn crate::core::tools::adapters::DiffController>>,
}

impl ToolContext {
    pub fn new(workspace: PathBuf, user_id: String) -> Self {
        // Create RooIgnore for the workspace
        let rooignore = Arc::new(RooIgnore::new(workspace.clone()));
        
        // Get global config
        let config = Arc::new(crate::core::tools::config::get_config().clone());
        
        let session_id = uuid::Uuid::new_v4().to_string();
        let execution_id = uuid::Uuid::new_v4().to_string();
        
        // Create log context
        let log_context = LogContext::new(session_id.clone(), user_id.clone());
        
        Self {
            workspace: workspace.clone(),
            user_id,
            session_id,
            execution_id,
            require_approval: true,
            dry_run: false,
            metadata: HashMap::new(),
            permissions: ToolPermissions::default(),
            rooignore: Some(rooignore),
            permission_manager: None,
            config,
            log_context: Some(log_context),
            adapters: HashMap::new(),
            event_emitters: Vec::new(),
            diff_controllers: Vec::new(),
        }
    }
    
    /// Get an adapter by name (legacy, use specific methods)
    pub fn get_adapter(&self, name: &str) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.adapters.get(name).cloned()
    }
    
    /// Add an adapter
    pub fn add_adapter(&mut self, name: String, adapter: Arc<dyn std::any::Any + Send + Sync>) {
        self.adapters.insert(name, adapter);
    }
    
    /// Add event emitter adapter
    pub fn add_event_emitter(&mut self, emitter: Arc<dyn crate::core::tools::adapters::EventEmitter>) {
        self.event_emitters.push(emitter);
    }
    
    /// Get first available event emitter
    pub fn get_event_emitter(&self) -> Option<Arc<dyn crate::core::tools::adapters::EventEmitter>> {
        self.event_emitters.first().cloned()
    }
    
    /// Add diff controller adapter
    pub fn add_diff_controller(&mut self, controller: Arc<dyn crate::core::tools::adapters::DiffController>) {
        self.diff_controllers.push(controller);
    }
    
    /// Get first available diff controller
    pub fn get_diff_controller(&self) -> Option<Arc<dyn crate::core::tools::adapters::DiffController>> {
        self.diff_controllers.first().cloned()
    }
    
    /// Create context with custom config
    pub fn with_config(workspace: PathBuf, user_id: String, config: ToolConfig) -> Self {
        let rooignore = Arc::new(RooIgnore::new(workspace.clone()));
        let config = Arc::new(config);
        
        let session_id = uuid::Uuid::new_v4().to_string();
        let execution_id = uuid::Uuid::new_v4().to_string();
        let log_context = LogContext::new(session_id.clone(), user_id.clone());
        
        Self {
            workspace: workspace.clone(),
            user_id,
            session_id,
            execution_id,
            require_approval: true,
            dry_run: false,
            metadata: HashMap::new(),
            permissions: ToolPermissions::default(),
            rooignore: Some(rooignore),
            permission_manager: None,
            config,
            log_context: Some(log_context),
            adapters: HashMap::new(),
            event_emitters: Vec::new(),
            diff_controllers: Vec::new(),
        }
    }
    
    /// Check if a path is allowed by .rooignore
    pub fn is_path_allowed(&self, path: &PathBuf) -> bool {
        if let Some(ref rooignore) = self.rooignore {
            rooignore.is_allowed(path.as_path())
        } else {
            true
        }
    }
    
    /// Get absolute path relative to workspace
    pub fn resolve_path(&self, relative: &str) -> PathBuf {
        self.workspace.join(relative)
    }
    
    /// Check if file read is allowed
    pub async fn can_read_file(&self, path: &Path) -> bool {
        // First check basic permission
        if !self.permissions.file_read {
            return false;
        }
        
        // Then check with permission manager if available
        if let Some(ref pm) = self.permission_manager {
            pm.check_permission(path, &Permission::Read)
        } else {
            true
        }
    }
    
    /// Check if file write is allowed
    pub async fn can_write_file(&self, path: &Path) -> bool {
        // First check basic permission
        if !self.permissions.file_write {
            return false;
        }
        
        // Then check with permission manager if available
        if let Some(ref pm) = self.permission_manager {
            pm.check_permission(path, &Permission::Write)
        } else {
            true
        }
    }
    
    /// Check if command execution is allowed
    pub fn can_execute_command(&self) -> bool {
        self.permissions.execute
    }
    
    /// Check if network access is allowed
    pub fn can_access_network(&self) -> bool {
        self.permissions.network
    }
    
    /// Check if a specific command is blocked by config
    pub fn is_command_blocked(&self, command: &str) -> bool {
        self.config.is_command_blocked(command)
    }
    
    /// Get timeout for a specific tool from config
    pub fn get_tool_timeout(&self, tool_name: &str) -> std::time::Duration {
        self.config.get_timeout(tool_name)
    }
    
    /// Check if approval is required for a tool operation based on config
    pub fn requires_approval_for(&self, tool_name: &str, operation: &str) -> bool {
        // Override with context's require_approval if it's false
        if !self.require_approval {
            return false;
        }
        self.config.requires_approval(tool_name, operation)
    }
    
    /// Get maximum file size from config
    pub fn max_file_size(&self) -> usize {
        self.config.max_file_size()
    }
    
    /// Get maximum output size from config
    pub fn max_output_size(&self) -> usize {
        self.config.max_output_size()
    }
    
    /// Log start of tool execution
    pub fn log_start(&self, tool_name: &str) {
        log_tool_start(tool_name, &self.execution_id, &self.session_id);
    }
    
    /// Log completion of tool execution
    pub fn log_complete(&self, tool_name: &str, success: bool, error: Option<&str>) {
        if let Some(ref log_ctx) = self.log_context {
            log_tool_complete(
                tool_name,
                &self.execution_id,
                log_ctx.elapsed(),
                success,
                error
            );
        }
    }
    
    /// Log file operation for audit
    pub fn log_file_op(&self, operation: &str, path: &str, success: bool, error: Option<&str>) {
        log_file_operation(operation, path, success, error);
    }
    
    /// Log approval request and return approval ID
    pub fn log_approval(&self, tool_name: &str, operation: &str, target: &str) -> String {
        log_approval_request(tool_name, operation, target, &self.user_id)
    }
}

impl Default for ToolContext {
    fn default() -> Self {
        Self::new(PathBuf::from("."), "default_user".to_string())
    }
}

/// Result type for tool execution
pub type ToolResult = Result<ToolOutput, ToolError>;

/// Convenience struct for tool results (for tests)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultStruct {
    pub success: bool,
    pub data: Value,
    pub metadata: Value,
}

/// Tool output structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Success status
    pub success: bool,
    
    /// Primary result data
    pub result: Value,
    
    /// Optional error message
    pub error: Option<String>,
    
    /// Execution metadata (timing, counts, etc.)
    pub metadata: HashMap<String, Value>,
}

impl ToolOutput {
    pub fn success(result: Value) -> Self {
        Self {
            success: true,
            result,
            error: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            result: Value::Null,
            error: Some(msg.into()),
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Custom error type for tools
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Path blocked by .rooignore: {0}")]
    RooIgnoreBlocked(String),
    
    #[error("Approval required for operation: {0}")]
    ApprovalRequired(String),
    
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Operation timeout: {0}")]
    Timeout(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Approval request for destructive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequired {
    pub tool_name: String,
    pub operation: String,
    pub target: String,
    pub approval_id: String,
}

/// Core trait for tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Name of the tool
    fn name(&self) -> &'static str;
    
    /// Description of what the tool does
    fn description(&self) -> &'static str;
    
    /// Whether this tool requires approval for execution
    fn requires_approval(&self) -> bool {
        false
    }
    
    /// Execute the tool with given arguments
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult;
    
    /// Execute with automatic logging
    async fn execute_with_logging(&self, args: Value, context: ToolContext) -> ToolResult {
        let tool_name = self.name();
        
        // Log start
        context.log_start(tool_name);
        
        // Execute tool
        let start = Instant::now();
        let result = self.execute(args, context.clone()).await;
        
        // Log completion
        match &result {
            Ok(_) => {
                context.log_complete(tool_name, true, None);
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                context.log_complete(tool_name, false, Some(&error_msg));
            }
        }
        
        result
    }
    
    /// Validate arguments before execution
    fn validate_args(&self, args: &Value) -> anyhow::Result<()> {
        // Default implementation - tools can override for specific validation
        Ok(())
    }
    
    /// Get tool metadata (parameter schema, examples, etc.)
    fn metadata(&self) -> Value {
        serde_json::json!({
            "name": self.name(),
            "description": self.description(),
            "requires_approval": self.requires_approval(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    struct MockTool;
    
    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            "mock_tool"
        }
        
        fn description(&self) -> &'static str {
            "A mock tool for testing"
        }
        
        async fn execute(&self, _args: Value, _context: ToolContext) -> ToolResult {
            Ok(ToolOutput::success(serde_json::json!({
                "message": "Mock execution successful"
            })))
        }
    }
    
    #[tokio::test]
    async fn test_tool_context_creation() {
        let temp_dir = TempDir::new().unwrap();
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_user".to_string()
        );
        
        assert_eq!(context.user_id, "test_user");
        assert!(context.require_approval);
        assert!(!context.dry_run);
    }
    
    #[tokio::test]
    async fn test_tool_execution() {
        let tool = MockTool;
        let temp_dir = TempDir::new().unwrap();
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_user".to_string()
        );
        
        let result = tool.execute(Value::Null, context).await.unwrap();
        assert!(result.success);
        assert_eq!(
            result.result.get("message").unwrap(),
            "Mock execution successful"
        );
    }
    
    #[test]
    fn test_tool_output_builders() {
        let success = ToolOutput::success(serde_json::json!({"key": "value"}))
            .with_metadata("time_ms", serde_json::json!(42));
        assert!(success.success);
        assert_eq!(success.metadata.get("time_ms").unwrap(), 42);
        
        let error = ToolOutput::error("Something went wrong");
        assert!(!error.success);
        assert_eq!(error.error.unwrap(), "Something went wrong");
    }
}
