// Core Tool Adapter: Wraps core::tools::Tool to implement mcp_tools::core::Tool
// Enables production tools to be exposed via MCP with full safety/streaming/approvals

use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::sync::Arc;

use crate::core::tools::traits::Tool as CoreTool;
use crate::mcp_tools::core::{Tool as McpTool, ToolContext as McpToolContext, ToolResult, ToolParameter};
use crate::mcp_tools::permissions::Permission;
use crate::mcp_tools::config::McpServerConfig;
use crate::mcp_tools::core::ResourceLimits;

use super::result::core_output_to_mcp;
use super::context::to_core_context;

/// Wrapper that adapts a core tool to MCP tool interface
pub struct CoreToolAsMcp {
    inner: Arc<dyn CoreTool>,
    config: Arc<McpServerConfig>,
}

impl CoreToolAsMcp {
    pub fn new(tool: Arc<dyn CoreTool>, config: Arc<McpServerConfig>) -> Self {
        Self {
            inner: tool,
            config,
        }
    }
    
    /// Determine required permissions based on tool name and category
    fn derive_permissions(&self) -> Vec<Permission> {
        let name = self.inner.name();
        
        // Map tool names to permissions
        match name {
            // File read tools
            "readFile" | "listFiles" | "searchFiles" => {
                vec![Permission::FileRead("*".to_string())]
            }
            
            // File write tools
            "writeFile" | "editFile" | "insertContent" | "searchAndReplace" => {
                vec![Permission::FileWrite("*".to_string())]
            }
            
            // Diff tools (read + write)
            "applyDiff" | "apply_diff" => {
                vec![
                    Permission::FileRead("*".to_string()),
                    Permission::FileWrite("*".to_string()),
                ]
            }
            
            // Command execution
            "executeCommand" | "terminal" => {
                vec![Permission::CommandExecute("*".to_string())]
            }
            
            // Network tools
            "curl" => {
                vec![Permission::NetworkAccess("*".to_string())]
            }
            
            // Git tools (file read)
            "git_status" | "gitStatus" | "git_diff" | "gitDiff" => {
                vec![Permission::FileRead("*".to_string())]
            }
            
            // Default: file read permission
            _ => vec![Permission::FileRead("*".to_string())]
        }
    }
}

#[async_trait]
impl McpTool for CoreToolAsMcp {
    fn name(&self) -> &str {
        self.inner.name()
    }
    
    fn description(&self) -> &str {
        self.inner.description()
    }
    
    fn input_schema(&self) -> Value {
        // For XML-based tools (most core tools), expose a simple schema with "args" string
        // This matches how tests drive core tools (see read_file_v2.rs tests)
        json!({
            "type": "object",
            "properties": {
                "args": {
                    "type": "string",
                    "description": "XML-formatted arguments for the tool. See tool documentation for format."
                }
            },
            "required": ["args"]
        })
    }
    
    async fn validate(&self, _args: &Value) -> Result<()> {
        // Validation is handled by the core tool's execute method
        // which parses XML and validates arguments
        Ok(())
    }
    
    async fn execute(&self, args: Value, mcp_context: McpToolContext) -> Result<ToolResult> {
        // Convert MCP context to core context
        let core_context = to_core_context(mcp_context, &self.config);
        
        // Execute the core tool
        let core_output = self.inner.execute(args, core_context).await?;
        
        // Convert result
        Ok(core_output_to_mcp(core_output))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        self.derive_permissions()
    }
    
    fn parameters(&self) -> Vec<ToolParameter> {
        // Most core tools use XML args, so parameters are embedded in the XML
        // Return empty for now; can be enhanced per-tool later
        vec![]
    }
    
    fn resource_limits(&self) -> ResourceLimits {
        // Use reasonable defaults; can be customized per-tool later
        ResourceLimits {
            max_memory_mb: 100,
            max_cpu_seconds: 30,
            max_file_size_mb: 100,
            max_concurrent_ops: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tools::fs::read_file_v2::ReadFileToolV2;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;
    use std::fs;
    
    #[tokio::test]
    async fn test_adapter_basic() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Create test file
        let test_content = "Hello from core tool via MCP bridge!";
        fs::write(workspace.join("test.txt"), test_content).unwrap();
        
        // Create adapter
        let core_tool: Arc<dyn CoreTool> = Arc::new(ReadFileToolV2);
        let config = Arc::new(McpServerConfig::default());
        let adapter = CoreToolAsMcp::new(core_tool, config);
        
        // Verify MCP interface
        assert_eq!(adapter.name(), "readFile");
        assert!(!adapter.description().is_empty());
        
        // Check schema
        let schema = adapter.input_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["args"].is_object());
        
        // Execute via MCP
        let mcp_ctx = McpToolContext {
            workspace: workspace.clone(),
            user: Some("test_user".to_string()),
            user_id: "user123".to_string(),
            session_id: "session456".to_string(),
            request_id: "req789".to_string(),
            cancellation_token: CancellationToken::new(),
            metadata: None,
        };
        
        let args = json!(r#"
            <tool>
                <path>test.txt</path>
            </tool>
        "#);
        
        let result = adapter.execute(args, mcp_ctx).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.data.unwrap()["content"], test_content);
    }
    
    #[test]
    fn test_permission_derivation() {
        let config = Arc::new(McpServerConfig::default());
        
        // Test file read tool
        let read_tool: Arc<dyn CoreTool> = Arc::new(ReadFileToolV2);
        let adapter = CoreToolAsMcp::new(read_tool, config.clone());
        let perms = adapter.required_permissions();
        assert_eq!(perms.len(), 1);
        assert!(matches!(perms[0], Permission::FileRead(_)));
    }
    
    #[tokio::test]
    async fn test_rooignore_enforcement() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Create .rooignore blocking *.secret files
        fs::write(workspace.join(".rooignore"), "*.secret\n").unwrap();
        
        // Create a secret file
        fs::write(workspace.join("test.secret"), "secret data").unwrap();
        
        // Create adapter
        let core_tool: Arc<dyn CoreTool> = Arc::new(ReadFileToolV2);
        let config = Arc::new(McpServerConfig::default());
        let adapter = CoreToolAsMcp::new(core_tool, config);
        
        // Try to read blocked file via MCP
        let mcp_ctx = McpToolContext {
            workspace: workspace.clone(),
            user: Some("test_user".to_string()),
            user_id: "user123".to_string(),
            session_id: "session456".to_string(),
            request_id: "req789".to_string(),
            cancellation_token: CancellationToken::new(),
            metadata: None,
        };
        
        let args = json!(r#"
            <tool>
                <path>test.secret</path>
            </tool>
        "#);
        
        let result = adapter.execute(args, mcp_ctx).await;
        
        // Should be blocked by .rooignore
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("rooignore") || err_msg.contains("blocked"));
    }
}
