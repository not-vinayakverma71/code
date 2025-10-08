/// IPC Integration for MCP Tools with Lapce
/// Connects MCP tools to Lapce's IPC system for communication with the editor

use crate::mcp_tools::{
    dispatcher::{McpToolSystem, ToolInfo},
    config::McpServerConfig,
};
// TODO: Add lapce_rpc dependency

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

// Define missing types locally until lapce_rpc is available
pub type RpcId = u64;
pub type RequestId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RpcMessage {
    Request {
        id: RpcId,
        method: String,
        params: Value,
    },
    Response {
        id: RpcId,
        result: Option<Value>,
        error: Option<Value>,
    },
}

/// MCP IPC Handler for processing tool requests from Lapce
pub struct McpIpcHandler {
    tool_system: Arc<McpToolSystem>,
    active_requests: Arc<RwLock<Vec<RpcId>>>,
}

/// IPC Messages for MCP Tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum McpRequest {
    /// Execute a tool
    ExecuteTool {
        tool_name: String,
        args: Value,
    },
    
    /// List available tools
    ListTools,
    
    /// Get tool info
    GetToolInfo {
        tool_name: String,
    },
    
    /// Update configuration
    UpdateConfig {
        config: McpServerConfig,
    },
    
    /// Get metrics
    GetMetrics,
    
    /// Reset metrics
    ResetMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpResponse {
    /// Tool execution result
    ToolResult {
        success: bool,
        output: Value,
        error: Option<Value>,
    },
    
    /// List of available tools
    ToolList {
        tools: Vec<ToolInfo>,
    },
    
    /// Tool information
    ToolInfo {
        name: String,
        description: String,
        input_schema: Value,
    },
    
    /// Configuration update result
    ConfigUpdated {
        success: bool,
        error: Option<String>,
    },
    
    /// Metrics data
    Metrics {
        total_requests: u64,
        successful_requests: u64,
        failed_requests: u64,
        tool_usage: Vec<(String, u64)>,
        avg_execution_time_ms: u64,
    },
    
    /// Generic error
    Error {
        message: String,
    },
}

impl McpIpcHandler {
    /// Create a new IPC handler with MCP tool system
    pub fn new(config: McpServerConfig, workspace: PathBuf) -> Self {
        let tool_system = Arc::new(McpToolSystem::new(config, workspace));
        
        Self {
            tool_system,
            active_requests: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Handle incoming IPC request
    pub async fn handle_request(&self, request_id: RequestId, request: McpRequest) -> McpResponse {
        // Track active request
        {
            let mut active = self.active_requests.write().await;
            active.push(request_id.clone());
        }
        
        let response = match request {
            McpRequest::ExecuteTool { tool_name, args } => {
                self.execute_tool(&tool_name, args).await
            },
            
            McpRequest::ListTools => {
                self.list_tools().await
            },
            
            McpRequest::GetToolInfo { tool_name } => {
                self.get_tool_info(&tool_name).await
            },
            
            McpRequest::UpdateConfig { config } => {
                self.update_config(config).await
            },
            
            McpRequest::GetMetrics => {
                self.get_metrics().await
            },
            
            McpRequest::ResetMetrics => {
                self.reset_metrics().await
            },
        };
        
        // Remove from active requests
        {
            let mut active = self.active_requests.write().await;
            active.retain(|id| id != &request_id);
        }
        
        response
    }
    
    /// Execute a tool
    async fn execute_tool(&self, tool_name: &str, args: Value) -> McpResponse {
        match self.tool_system.execute_tool(tool_name, args).await {
            Ok(result) => McpResponse::ToolResult {
                success: result.success,
                output: result.data.unwrap_or(json!({})),
                error: result.error.map(|e| json!(e)),
            },
            Err(e) => McpResponse::Error {
                message: e.to_string(),
            },
        }
    }
    
    /// List available tools
    async fn list_tools(&self) -> McpResponse {
        let tools = self.tool_system.list_tools();
        McpResponse::ToolList { tools }
    }
    
    /// Get information about a specific tool
    async fn get_tool_info(&self, tool_name: &str) -> McpResponse {
        if let Some(tool) = self.tool_system.get_tool(tool_name) {
            McpResponse::ToolInfo {
                name: tool_name.to_string(),
                description: tool.description().to_string(),
                input_schema: tool.input_schema(),
            }
        } else {
            McpResponse::Error {
                message: format!("Tool '{}' not found", tool_name),
            }
        }
    }
    
    /// Update configuration
    async fn update_config(&self, config: McpServerConfig) -> McpResponse {
        match self.tool_system.update_config(config).await {
            Ok(_) => McpResponse::ConfigUpdated {
                success: true,
                error: None,
            },
            Err(e) => McpResponse::ConfigUpdated {
                success: false,
                error: Some(e.to_string()),
            },
        }
    }
    
    /// Get metrics
    async fn get_metrics(&self) -> McpResponse {
        let metrics = self.tool_system.get_metrics().await;
        
        let avg_time = if metrics.total_requests > 0 {
            metrics.total_execution_time_ms / metrics.total_requests
        } else {
            0
        };
        
        McpResponse::Metrics {
            total_requests: metrics.total_requests,
            successful_requests: metrics.successful_requests,
            failed_requests: metrics.failed_requests,
            tool_usage: metrics.tool_usage.into_iter().collect(),
            avg_execution_time_ms: avg_time,
        }
    }
    
    /// Reset metrics
    async fn reset_metrics(&self) -> McpResponse {
        self.tool_system.reset_metrics().await;
        McpResponse::ConfigUpdated {
            success: true,
            error: None,
        }
    }
    
    /// Cancel active request
    pub async fn cancel_request(&self, request_id: RequestId) -> bool {
        let mut active = self.active_requests.write().await;
        let index = active.iter().position(|id| id == &request_id);
        
        if let Some(idx) = index {
            active.remove(idx);
            true
        } else {
            false
        }
    }
}

/// Integrate MCP tools with Lapce's existing IPC system
pub struct LapceIpcIntegration {
    handler: Arc<McpIpcHandler>,
}

impl LapceIpcIntegration {
    pub fn new(handler: Arc<McpIpcHandler>) -> Self {
        Self { handler }
    }
    
    /// Process incoming RPC message
    pub async fn process_rpc(&self, msg: RpcMessage) -> Option<RpcMessage> {
        match msg {
            RpcMessage::Request { id, method, params } if method.starts_with("mcp.") => {
                // Parse MCP request
                if let Ok(request) = serde_json::from_value::<McpRequest>(params) {
                    let response = self.handler.handle_request(id.clone(), request).await;
                    
                    Some(RpcMessage::Response {
                        id,
                        result: Some(serde_json::to_value(response).ok()?),
                        error: None,
                    })
                } else {
                    Some(RpcMessage::Response {
                        id,
                        result: None,
                        error: Some(json!({
                            "code": -32602,
                            "message": "Invalid params"
                        })),
                    })
                }
            },
            _ => None, // Not an MCP request
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_ipc_handler_creation() {
        let config = McpServerConfig::default();
        let workspace = tempdir().unwrap().path().to_path_buf();
        let handler = McpIpcHandler::new(config, workspace);
        
        assert!(handler.active_requests.read().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_list_tools_request() {
        let config = McpServerConfig::default();
        let workspace = tempdir().unwrap().path().to_path_buf();
        let handler = McpIpcHandler::new(config, workspace);
        
        let request_id: RequestId = 1;
        let response = handler.handle_request(
            request_id,
            McpRequest::ListTools
        ).await;
        
        match response {
            McpResponse::ToolList { tools } => {
                assert!(!tools.is_empty());
                assert!(tools.iter().any(|t| t.name == "readFile"));
            },
            _ => panic!("Expected ToolList response"),
        }
    }
    
    #[tokio::test]
    async fn test_execute_tool_request() {
        let config = McpServerConfig::default();
        let workspace = tempdir().unwrap().path().to_path_buf();
        let handler = McpIpcHandler::new(config, workspace.clone());
        
        // Create test file
        std::fs::write(workspace.join("test.txt"), "Hello IPC").unwrap();
        
        let request_id: RequestId = 1;
        let response = handler.handle_request(
            request_id,
            McpRequest::ExecuteTool {
                tool_name: "readFile".to_string(),
                args: json!({ "path": "test.txt" }),
            }
        ).await;
        
        match response {
            McpResponse::ToolResult { success, .. } => {
                assert!(success);
            },
            _ => panic!("Expected ToolResult response"),
        }
    }
}
