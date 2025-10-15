/// MCP Tool System Dispatcher
/// Central dispatcher for routing tool requests to appropriate implementations

use crate::mcp_tools::tools::CodebaseSearchTool;
use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult},
    config::McpServerConfig,
    rate_limiter::RateLimiter,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::error::Error as StdError;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use anyhow::{Result, bail};

pub struct McpToolSystem {
    /// Registry of available tools
    tools: HashMap<String, Box<dyn Tool + Send + Sync>>,
    
    /// Server configuration
    config: Arc<RwLock<McpServerConfig>>,
    
    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,
    
    /// Tool execution metrics
    metrics: Arc<RwLock<ToolMetrics>>,
    
    /// Current workspace path
    workspace: PathBuf,
}

#[derive(Default)]
pub struct ToolMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub tool_usage: HashMap<String, u64>,
    pub total_execution_time_ms: u64,
}

impl McpToolSystem {
    fn check_permissions(&self, tool_name: &str, config: &McpServerConfig) -> bool {
        // Check permissions based on tool name
        match tool_name {
            "executeCommand" => config.permissions.default.process_execute,
            "readFile" => config.permissions.default.file_read,
            "writeFile" => config.permissions.default.file_write,
            _ => true, // Allow other tools by default
        }
    }
    
    /// Create a new MCP tool system with configuration
    pub fn new(config: McpServerConfig, workspace: PathBuf) -> Self {
        let mut tools: HashMap<String, Box<dyn Tool + Send + Sync>> = HashMap::new();
        
        // Register all available tools
        tools.insert("codebaseSearch".to_string(), Box::new(CodebaseSearchTool::new()));
        tools.insert("readFile".to_string(), Box::new(crate::mcp_tools::tools::read_file::ReadFileTool::new()));
        tools.insert("writeFile".to_string(), Box::new(crate::mcp_tools::tools::write_file::WriteFileTool::new()));
        tools.insert("executeCommand".to_string(), Box::new(crate::mcp_tools::tools::execute_command::ExecuteCommandTool::new()));
        tools.insert("listFiles".to_string(), Box::new(crate::mcp_tools::tools::list_files::ListFilesTool::new()));
        tools.insert("searchFiles".to_string(), Box::new(crate::mcp_tools::tools::search_files::SearchFilesTool::new()));
        tools.insert("editFile".to_string(), Box::new(crate::mcp_tools::tools::edit_file::EditFileTool::new()));
        
        let rate_limiter = Arc::new(RateLimiter::with_rate(config.rate_limits.requests_per_minute as u32));
        
        Self {
            tools,
            config: Arc::new(RwLock::new(config)),
            rate_limiter,
            metrics: Arc::new(RwLock::new(ToolMetrics::default())),
            workspace,
        }
    }
    
    /// Execute a tool by name with the given arguments
    pub async fn execute_tool(&self, tool_name: &str, args: Value) -> Result<ToolResult> {
        let start_time = std::time::Instant::now();
        
        // Check if tool exists
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_name))?;
        
        // Create a temporary context for rate limiting
        let temp_user_id = "default_user".to_string();
        
        // Check rate limits
        if self.rate_limiter.check_rate_limit(&temp_user_id, tool_name).await.is_err() {
            bail!("Rate limit exceeded for tool '{}'", tool_name);
        }
        
        // Check permissions
        let config = self.config.read().await;
        if !self.check_permissions(tool_name, &config) {
            bail!("Permission denied for tool '{}'", tool_name);
        }
        
        // Create execution context
        let context = ToolContext {
            workspace: self.workspace.clone(),
            user: String::new(),
            user_id: String::new(),
            session_id: String::new(),
            request_id: String::new(),
            cancellation_token: CancellationToken::new(),
            metadata: None,
        };
        
        // Validate arguments
        tool.validate(&args).await?;
        
        // Execute tool
        let result = match tool.execute(args, context).await {
            Ok(result) => {
                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.total_requests += 1;
                metrics.successful_requests += 1;
                *metrics.tool_usage.entry(tool_name.to_string()).or_insert(0) += 1;
                metrics.total_execution_time_ms += start_time.elapsed().as_millis() as u64;
                
                result
            },
            Err(e) => {
                // Update failure metrics
                let mut metrics = self.metrics.write().await;
                metrics.total_requests += 1;
                metrics.failed_requests += 1;
                
                ToolResult {
                    success: false,
                    data: Some(json!({ "error": e.to_string() })),
                    error: Some(e.to_string()),
                    metadata: Some(json!({})),
                }
            }
        };
        
        Ok(result)
    }
    
    
    /// List all available tools
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        self.tools.iter().map(|(name, tool)| {
            ToolInfo {
                name: name.clone(),
                description: tool.description().to_string(),
                input_schema: tool.input_schema(),
            }
        }).collect()
    }
    
    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<&Box<dyn Tool + Send + Sync>> {
        self.tools.get(name)
    }
    
    /// Update configuration
    pub async fn update_config(&self, config: McpServerConfig) -> Result<()> {
        config.validate().map_err(|e| anyhow::anyhow!(e))?;
        let mut current = self.config.write().await;
        *current = config;
        Ok(())
    }
    
    /// Get current metrics
    pub async fn get_metrics(&self) -> ToolMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Reset metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = ToolMetrics::default();
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl Clone for ToolMetrics {
    fn clone(&self) -> Self {
        Self {
            total_requests: self.total_requests,
            successful_requests: self.successful_requests,
            failed_requests: self.failed_requests,
            tool_usage: self.tool_usage.clone(),
            total_execution_time_ms: self.total_execution_time_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_tool_system_initialization() {
        let config = McpServerConfig::default();
        let workspace = tempdir().unwrap().path().to_path_buf();
        let system = McpToolSystem::new(config, workspace);
        
        // Check tools are registered
        assert!(system.get_tool("readFile").is_some());
        assert!(system.get_tool("writeFile").is_some());
        assert!(system.get_tool("executeCommand").is_some());
    }
    
    #[tokio::test]
    async fn test_tool_execution() {
        let config = McpServerConfig::default();
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let system = McpToolSystem::new(config, workspace.clone());
        
        // Create a test file
        let test_file = workspace.join("test.txt");
        std::fs::write(&test_file, "Hello MCP").unwrap();
        
        // Execute readFile tool
        let args = json!({
            "path": "test.txt"
        });
        
        let result = system.execute_tool("readFile", args).await.unwrap();
        assert!(result.success);
        
        // Check metrics
        let metrics = system.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
    }
    
    #[tokio::test]
    async fn test_permission_checking() {
        let mut config = McpServerConfig::default();
        config.permissions.default.process_execute = false;
        
        let workspace = tempdir().unwrap().path().to_path_buf();
        let system = McpToolSystem::new(config, workspace);
        
        // Try to execute command (should fail due to permissions)
        let args = json!({
            "command": "echo test"
        });
        
        let result = system.execute_tool("executeCommand", args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Permission denied"));
    }
}
