// EXACT 1:1 TRANSLATION FROM TYPESCRIPT McpHub
// Critical: NO CHANGES to message formats - must match TypeScript exactly

// use super::protocol::*;  // TODO: Create protocol module
// use super::server_manager::McpServerManager;  // TODO: Create server_manager module
// use jsonschema::{Draft, JSONSchema};  
use anyhow::{Result, bail};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

// EXACT TypeScript types translated to Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub name: String,
    pub config: String,
    pub status: McpServerStatus,
    pub disabled: bool,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub errors: Vec<McpErrorEntry>,
    pub capabilities: Option<McpCapabilities>,
    pub environment: Option<HashMap<String, String>>,
    pub args: Option<Vec<String>>,
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpServerStatus {
    Connected,
    Connecting,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpErrorEntry {
    pub message: String,
    pub timestamp: u64,
    pub level: McpErrorLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpErrorLevel {
    Error,
    Warn,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
    pub server_name: Option<String>,
    pub enabled_for_prompt: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceTemplate {
    pub uri_template: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceResponse {
    #[serde(rename = "_meta")]
    pub meta: Option<HashMap<String, serde_json::Value>>,
    pub contents: Vec<McpResourceContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallResponse {
    #[serde(rename = "_meta")]
    pub meta: Option<HashMap<String, serde_json::Value>>,
    pub content: Vec<McpToolContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpToolContent {
    Text {
        #[serde(rename = "type")]
        content_type: String, // "text"
        text: String,
    },
    Image {
        #[serde(rename = "type")]
        content_type: String, // "image"
        data: String,
        mime_type: String,
    },
    Resource {
        #[serde(rename = "type")]
        content_type: String, // "resource"
        resource: McpResource,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCapabilities {
    pub tools: bool,
    pub resources: bool,
    pub prompts: bool,
    pub sampling: bool,
}

/// McpHub - EXACT translation from TypeScript
/// Manages MCP servers, tools, resources, and lifecycle
pub struct McpHub {
    servers: Arc<RwLock<HashMap<String, McpServer>>>,
    tools: Arc<RwLock<HashMap<String, McpTool>>>,
    resources: Arc<RwLock<HashMap<String, McpResource>>>,
    resource_templates: Arc<RwLock<Vec<McpResourceTemplate>>>,
    mcp_servers_path: Option<String>,
    server_manager: Arc<RwLock<Option<McpServerManager>>>,
}

fn validate_json_schema(data: &serde_json::Value, schema: &serde_json::Value) -> Result<()> {
    use jsonschema::{JSONSchema, Draft};
    
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(schema)
        .map_err(|e| anyhow::anyhow!("Invalid schema: {}", e))?;
    
    if let Err(errors) = compiled.validate(data) {
        let error_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
        bail!("Schema validation failed: {}", error_msgs.join(", "));
    }
    
    Ok(())
}

impl McpHub {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
            resources: Arc::new(RwLock::new(HashMap::new())),
            resource_templates: Arc::new(RwLock::new(Vec::new())),
            mcp_servers_path: None,
            server_manager: Arc::new(RwLock::new(None)),
        }
    }

    /// Get all registered servers
    pub async fn get_servers(&self) -> Vec<McpServer> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }

    /// Get MCP servers path
    pub async fn get_mcp_servers_path(&self) -> Option<String> {
        self.mcp_servers_path.clone()
    }

    /// List all available tools
    pub async fn list_tools(&self) -> Vec<McpTool> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// Call a specific tool
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolCallResponse> {
        let tools = self.tools.read().await;
        
        let tool_key = format!("{}:{}", server_name, tool_name);
        let tool = tools.get(&tool_key)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_key))?;
        
        // Validate arguments against schema if present
        if let Some(schema) = &tool.input_schema {
            validate_json_schema(&arguments, schema)?;
        }
        
        // Execute tool through server manager
        let manager = self.server_manager.read().await;
        if let Some(ref mgr) = *manager {
            mgr.execute_tool(server_name, tool_name, arguments).await
        } else {
            bail!("Server manager not initialized")
        }
    }

    /// List all available resources
    pub async fn list_resources(&self) -> Vec<McpResource> {
        let resources = self.resources.read().await;
        resources.values().cloned().collect()
    }

    /// Get specific resource
    pub async fn get_resource(&self, uri: &str) -> Result<McpResourceResponse> {
        let resources = self.resources.read().await;
        
        let resource = resources.get(uri)
            .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", uri))?;
        
        // Fetch resource content through server manager
        let manager = self.server_manager.read().await;
        if let Some(ref mgr) = *manager {
            mgr.fetch_resource(uri).await
        } else {
            bail!("Server manager not initialized")
        }
    }

    /// Handle MCP enabled/disabled state change
    pub async fn handle_mcp_enabled_change(&self, enabled: bool) -> Result<()> {
        if enabled {
            self.start_all_servers().await?;
        } else {
            self.stop_all_servers().await?;
        }
        Ok(())
    }

    /// Start all registered servers
    pub async fn start_all_servers(&self) -> Result<()> {
        let servers = self.servers.read().await;
        for server in servers.values() {
            if !server.disabled {
                self.start_server(&server.name).await?;
            }
        }
        Ok(())
    }

    /// Stop all running servers
    pub async fn stop_all_servers(&self) -> Result<()> {
        let mut servers = self.servers.write().await;
        for server in servers.values_mut() {
            server.status = McpServerStatus::Disconnected;
        }
        
        // Stop through server manager
        let manager = self.server_manager.read().await;
        if let Some(ref mgr) = *manager {
            mgr.stop_all().await?;
        }
        
        Ok(())
    }

    /// Start a specific server
    pub async fn start_server(&self, server_name: &str) -> Result<()> {
        let mut servers = self.servers.write().await;
        
        if let Some(server) = servers.get_mut(server_name) {
            server.status = McpServerStatus::Connecting;
            
            // Start through server manager
            let manager = self.server_manager.read().await;
            if let Some(ref mgr) = *manager {
                match mgr.start_server(server_name).await {
                    Ok(_) => {
                        server.status = McpServerStatus::Connected;
                        self.discover_server_capabilities(server_name).await?;
                    }
                    Err(e) => {
                        server.status = McpServerStatus::Disconnected;
                        server.errors.push(McpErrorEntry {
                            message: e.to_string(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            level: McpErrorLevel::Error,
                        });
                        return Err(e);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Stop a specific server
    pub async fn stop_server(&self, server_name: &str) -> Result<()> {
        let mut servers = self.servers.write().await;
        
        if let Some(server) = servers.get_mut(server_name) {
            server.status = McpServerStatus::Disconnected;
            
            // Stop through server manager
            let manager = self.server_manager.read().await;
            if let Some(ref mgr) = *manager {
                mgr.stop_server(server_name).await?;
            }
        }
        
        Ok(())
    }

    /// Discover tools and resources for a server
    async fn discover_server_capabilities(&self, server_name: &str) -> Result<()> {
        let manager = self.server_manager.read().await;
        
        if let Some(ref mgr) = *manager {
            // Discover tools
            let tools = mgr.discover_tools(server_name).await?;
            let mut tools_map = self.tools.write().await;
            for mut tool in tools {
                tool.server_name = Some(server_name.to_string());
                let key = format!("{}:{}", server_name, tool.name);
                tools_map.insert(key, tool);
            }
            
            // Discover resources
            let resources = mgr.discover_resources(server_name).await?;
            let mut resources_map = self.resources.write().await;
            for resource in resources {
                resources_map.insert(resource.uri.clone(), resource);
            }
            
            // Update server capabilities
            let mut servers = self.servers.write().await;
            if let Some(server) = servers.get_mut(server_name) {
                server.capabilities = Some(McpCapabilities {
                    tools: !tools_map.is_empty(),
                    resources: !resources_map.is_empty(),
                    prompts: false, // TODO: Implement prompt discovery
                    sampling: false, // TODO: Implement sampling discovery
                });
            }
        }
        
        Ok(())
    }

    /// Add a new server
    pub async fn add_server(&self, server: McpServer) -> Result<()> {
        let mut servers = self.servers.write().await;
        servers.insert(server.name.clone(), server);
        Ok(())
    }

    /// Remove a server
    pub async fn remove_server(&self, server_name: &str) -> Result<()> {
        self.stop_server(server_name).await?;
        
        let mut servers = self.servers.write().await;
        servers.remove(server_name);
        
        // Remove associated tools and resources
        let mut tools = self.tools.write().await;
        tools.retain(|k, _| !k.starts_with(&format!("{}:", server_name)));
        
        Ok(())
    }

    /// Set the server manager
    pub async fn set_server_manager(&self, manager: McpServerManager) {
        let mut mgr = self.server_manager.write().await;
        *mgr = Some(manager);
    }
}

/// McpServerManager - Manages MCP server processes and communication
pub struct McpServerManager {
    // Implementation would handle actual server process management
    // This is a placeholder for the actual implementation
}

impl McpServerManager {
    pub async fn start_server(&self, _server_name: &str) -> Result<()> {
        // TODO: Implement actual server start logic
        Ok(())
    }

    pub async fn stop_server(&self, _server_name: &str) -> Result<()> {
        // TODO: Implement actual server stop logic
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        // TODO: Implement stop all servers logic
        Ok(())
    }

    pub async fn execute_tool(
        &self,
        _server_name: &str,
        _tool_name: &str,
        _arguments: serde_json::Value,
    ) -> Result<McpToolCallResponse> {
        // TODO: Implement actual tool execution
        Ok(McpToolCallResponse {
            meta: None,
            content: vec![],
        })
    }

    pub async fn fetch_resource(&self, _uri: &str) -> Result<McpResourceResponse> {
        // TODO: Implement actual resource fetching
        Ok(McpResourceResponse {
            meta: None,
            contents: vec![],
        })
    }

    pub async fn discover_tools(&self, _server_name: &str) -> Result<Vec<McpTool>> {
        // TODO: Implement tool discovery
        Ok(vec![])
    }

    pub async fn discover_resources(&self, _server_name: &str) -> Result<Vec<McpResource>> {
        // TODO: Implement resource discovery
        Ok(vec![])
    }
}
