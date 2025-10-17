// Context conversion: mcp_tools::core::ToolContext → core::tools::traits::ToolContext

use crate::mcp_tools::core::ToolContext as McpToolContext;
use crate::mcp_tools::config::McpServerConfig;
use crate::core::tools::traits::{ToolContext as CoreToolContext, ToolPermissions};
use crate::core::tools::RooIgnore;
use crate::core::tools::adapters::ipc::IpcAdapter;
use crate::core::tools::adapters::context_tracker_adapter::ContextTrackerAdapter;
use std::sync::Arc;
use serde_json::json;

/// Options for attaching adapters during context conversion
pub struct ContextConversionOptions {
    pub ipc_adapter: Option<Arc<IpcAdapter>>,
    pub context_tracker: Option<Arc<ContextTrackerAdapter>>,
}

impl Default for ContextConversionOptions {
    fn default() -> Self {
        Self {
            ipc_adapter: None,
            context_tracker: None,
        }
    }
}

/// Convert MCP tool context to core tool context (basic version)
/// Maps workspace, permissions, user info, and attaches .rooignore
pub fn to_core_context(
    mcp_ctx: McpToolContext,
    config: &McpServerConfig,
) -> CoreToolContext {
    to_core_context_with_adapters(mcp_ctx, config, ContextConversionOptions::default())
}

/// Convert MCP tool context to core tool context with optional adapters
/// This allows attaching IPC adapter and context tracker for full integration
pub fn to_core_context_with_adapters(
    mcp_ctx: McpToolContext,
    config: &McpServerConfig,
    options: ContextConversionOptions,
) -> CoreToolContext {
    // Create permissions from MCP config
    let permissions = ToolPermissions {
        read: config.permissions.default.file_read,
        write: config.permissions.default.file_write,
        execute: config.permissions.default.process_execute,
        file_read: config.permissions.default.file_read,
        file_write: config.permissions.default.file_write,
        network: config.permissions.default.network_access,
        command_execute: config.permissions.default.process_execute,
    };
    
    // Create core context starting from default
    let mut ctx = CoreToolContext::new(
        mcp_ctx.workspace.clone(),
        mcp_ctx.user_id.clone(),
    );
    
    // Override fields from MCP context
    ctx.session_id = mcp_ctx.session_id.clone();
    ctx.permissions = permissions;
    ctx.require_approval = true;
    
    // Attach RooIgnore
    ctx.rooignore = Some(Arc::new(RooIgnore::new(mcp_ctx.workspace.clone())));
    
    // Store MCP metadata in context metadata field
    if let Some(mcp_metadata) = mcp_ctx.metadata {
        ctx.metadata.insert("mcp_metadata".to_string(), mcp_metadata);
    }
    ctx.metadata.insert("mcp_request_id".to_string(), serde_json::json!(mcp_ctx.request_id));
    
    // Attach IPC adapter if provided
    if let Some(ipc_adapter) = options.ipc_adapter {
        ctx.add_adapter("ipc".to_string(), ipc_adapter.clone());
        ctx.add_event_emitter(ipc_adapter);
    }
    
    // Attach context tracker if provided
    if let Some(context_tracker) = options.context_tracker {
        ctx.add_adapter("context_tracker".to_string(), context_tracker);
    }
    
    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;
    
    #[test]
    fn test_basic_conversion() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let mcp_ctx = McpToolContext {
            workspace: workspace.clone(),
            user: Some("test_user".to_string()),
            user_id: "user123".to_string(),
            session_id: "session456".to_string(),
            request_id: "req789".to_string(),
            cancellation_token: CancellationToken::new(),
            metadata: None,
        };
        
        let config = McpServerConfig::default();
        let core_ctx = to_core_context(mcp_ctx, &config);
        
        assert_eq!(core_ctx.workspace, workspace);
        assert_eq!(core_ctx.user_id, "user123");
        assert_eq!(core_ctx.session_id, "session456");
        assert!(core_ctx.require_approval);
        assert!(!core_ctx.dry_run);
        
        // Check MCP metadata was stored
        assert!(core_ctx.metadata.contains_key("mcp_request_id"));
    }
    
    #[test]
    fn test_permissions_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let mcp_ctx = McpToolContext {
            workspace: workspace.clone(),
            user: None,
            user_id: "user123".to_string(),
            session_id: "session456".to_string(),
            request_id: "req789".to_string(),
            cancellation_token: CancellationToken::new(),
            metadata: None,
        };
        
        let mut config = McpServerConfig::default();
        config.permissions.default.file_read = true;
        config.permissions.default.file_write = false;
        config.permissions.default.network_access = true;
        config.permissions.default.process_execute = false;
        
        let core_ctx = to_core_context(mcp_ctx, &config);
        
        assert!(core_ctx.permissions.file_read);
        assert!(!core_ctx.permissions.file_write);
        assert!(core_ctx.permissions.network);
        assert!(!core_ctx.permissions.command_execute);
    }
    
    #[test]
    fn test_user_id_propagation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let mcp_ctx = McpToolContext {
            workspace: workspace.clone(),
            user: None,
            user_id: "user123".to_string(),
            session_id: "session456".to_string(),
            request_id: "req789".to_string(),
            cancellation_token: CancellationToken::new(),
            metadata: None,
        };
        
        let config = McpServerConfig::default();
        let core_ctx = to_core_context(mcp_ctx, &config);
        
        // user_id should be propagated
        assert_eq!(core_ctx.user_id, "user123");
    }
    
    #[test]
    fn test_rooignore_attached() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Create a .rooignore file
        std::fs::write(workspace.join(".rooignore"), "*.secret\n").unwrap();
        
        let mcp_ctx = McpToolContext {
            workspace: workspace.clone(),
            user: Some("test_user".to_string()),
            user_id: "user123".to_string(),
            session_id: "session456".to_string(),
            request_id: "req789".to_string(),
            cancellation_token: CancellationToken::new(),
            metadata: None,
        };
        
        let config = McpServerConfig::default();
        let core_ctx = to_core_context(mcp_ctx, &config);
        
        // Test that rooignore is working
        let secret_path = workspace.join("test.secret");
        assert!(!core_ctx.is_path_allowed(&secret_path));
        
        let normal_path = workspace.join("test.txt");
        assert!(core_ctx.is_path_allowed(&normal_path));
    }
}
