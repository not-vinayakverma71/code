// Core traits for tool system - P0-0: Scaffold core layer

use crate::core::tools::permissions::rooignore::RooIgnore;
use crate::mcp_tools::permission_manager::PermissionManager;
use crate::mcp_tools::permissions::Permission;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use anyhow::Result;
use async_trait::async_trait;

/// Tool permissions
#[derive(Debug, Clone)]
pub struct ToolPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub file_read: bool,
    pub file_write: bool,
    pub network: bool,
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
}

impl ToolContext {
    pub fn new(workspace: PathBuf, user_id: String) -> Self {
        // Create RooIgnore for the workspace
        let rooignore = Arc::new(RooIgnore::new(workspace.clone()));
        
        Self {
            workspace: workspace.clone(),
            user_id,
            session_id: uuid::Uuid::new_v4().to_string(),
            execution_id: uuid::Uuid::new_v4().to_string(),
            require_approval: true,
            dry_run: false,
            metadata: HashMap::new(),
            permissions: ToolPermissions::default(),
            rooignore: Some(rooignore),
            permission_manager: None,
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
            match pm.check_permission("read_file", &Permission::FileRead(path.to_string_lossy().to_string()), Some(path)).await {
                Ok(allowed) => allowed,
                Err(_) => false,
            }
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
            match pm.check_permission("write_file", &Permission::FileWrite(path.to_string_lossy().to_string()), Some(path)).await {
                Ok(allowed) => allowed,
                Err(_) => false,
            }
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
    pub details: String,
}

/// Core tool trait - all tools must implement this
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique name for the tool
    fn name(&self) -> &'static str;
    
    /// Tool description
    fn description(&self) -> &'static str;
    
    /// Whether this tool requires approval for execution
    fn requires_approval(&self) -> bool {
        false
    }
    
    /// Validate arguments before execution
    fn validate_args(&self, args: &Value) -> Result<()> {
        // Default implementation - tools can override for specific validation
        Ok(())
    }
    
    /// Execute the tool with given arguments and context
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult;
    
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
