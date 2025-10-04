/// Lapce Plugin Protocol Implementation (Day 28)
/// LSP-like protocol for Lapce IDE integration

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;
use dashmap::DashMap;

/// Protocol version
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Message types following LSP pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    Request(RequestMessage),
    Response(ResponseMessage),
    Notification(NotificationMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMessage {
    pub id: u64,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub id: u64,
    pub result: Option<Value>,
    pub error: Option<ResponseError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMessage {
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// Plugin capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapabilities {
    pub code_completion: bool,
    pub hover: bool,
    pub goto_definition: bool,
    pub find_references: bool,
    pub diagnostics: bool,
    pub code_actions: bool,
    pub formatting: bool,
    pub semantic_tokens: bool,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub capabilities: PluginCapabilities,
}

/// Request handler trait
#[async_trait::async_trait]
pub trait RequestHandler: Send + Sync {
    async fn handle_request(&self, method: &str, params: Option<Value>) -> Result<Value>;
}

/// Notification handler trait
#[async_trait::async_trait]
pub trait NotificationHandler: Send + Sync {
    async fn handle_notification(&self, method: &str, params: Option<Value>);
}

/// Plugin protocol server
pub struct PluginProtocolServer {
    plugin_info: PluginInfo,
    request_handlers: Arc<DashMap<String, Arc<dyn RequestHandler>>>,
    notification_handlers: Arc<DashMap<String, Arc<dyn NotificationHandler>>>,
    message_tx: mpsc::UnboundedSender<Message>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<Message>>>,
}

impl PluginProtocolServer {
    /// Create new plugin protocol server
    pub fn new(plugin_info: PluginInfo) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            plugin_info,
            request_handlers: Arc::new(DashMap::new()),
            notification_handlers: Arc::new(DashMap::new()),
            message_tx: tx,
            message_rx: Arc::new(RwLock::new(rx)),
        }
    }

    /// Register request handler
    pub fn register_request_handler(
        &self,
        method: String,
        handler: Arc<dyn RequestHandler>,
    ) {
        self.request_handlers.insert(method, handler);
    }

    /// Register notification handler
    pub fn register_notification_handler(
        &self,
        method: String,
        handler: Arc<dyn NotificationHandler>,
    ) {
        self.notification_handlers.insert(method, handler);
    }

    /// Process incoming message
    pub async fn process_message(&self, msg: Message) -> Result<Option<Message>> {
        match msg {
            Message::Request(req) => {
                let response = self.handle_request(req).await?;
                Ok(Some(Message::Response(response)))
            }
            Message::Notification(notif) => {
                self.handle_notification(notif).await;
                Ok(None)
            }
            Message::Response(_) => {
                // Responses are handled by client
                Ok(None)
            }
        }
    }

    /// Handle request
    async fn handle_request(&self, req: RequestMessage) -> Result<ResponseMessage> {
        // Handle built-in methods
        match req.method.as_str() {
            "initialize" => {
                let result = serde_json::to_value(&self.plugin_info)?;
                Ok(ResponseMessage {
                    id: req.id,
                    result: Some(result),
                    error: None,
                })
            }
            "shutdown" => {
                Ok(ResponseMessage {
                    id: req.id,
                    result: Some(Value::Null),
                    error: None,
                })
            }
            _ => {
                // Try custom handlers
                if let Some(handler) = self.request_handlers.get(&req.method) {
                    match handler.handle_request(&req.method, req.params).await {
                        Ok(result) => Ok(ResponseMessage {
                            id: req.id,
                            result: Some(result),
                            error: None,
                        }),
                        Err(e) => Ok(ResponseMessage {
                            id: req.id,
                            result: None,
                            error: Some(ResponseError {
                                code: -32603,
                                message: e.to_string(),
                                data: None,
                            }),
                        }),
                    }
                } else {
                    Ok(ResponseMessage {
                        id: req.id,
                        result: None,
                        error: Some(ResponseError {
                            code: -32601,
                            message: "Method not found".to_string(),
                            data: None,
                        }),
                    })
                }
            }
        }
    }

    /// Handle notification
    async fn handle_notification(&self, notif: NotificationMessage) {
        if let Some(handler) = self.notification_handlers.get(&notif.method) {
            handler.handle_notification(&notif.method, notif.params).await;
        }
    }

    /// Start message processing loop
    pub async fn start(&self) {
        let mut rx = self.message_rx.write().await;
        
        while let Some(msg) = rx.recv().await {
            if let Ok(Some(response)) = self.process_message(msg).await {
                let _ = self.message_tx.send(response);
            }
        }
    }
}

/// Plugin lifecycle manager
pub struct PluginLifecycle {
    plugins: Arc<DashMap<String, PluginState>>,
}

#[derive(Debug, Clone)]
pub struct PluginState {
    pub info: PluginInfo,
    pub status: PluginStatus,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PluginStatus {
    Uninitialized,
    Initializing,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

impl PluginLifecycle {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(DashMap::new()),
        }
    }

    /// Discover available plugins
    pub async fn discover_plugins(&self, plugin_dir: &std::path::Path) -> Result<Vec<PluginInfo>> {
        let mut plugins = Vec::new();
        
        let mut entries = tokio::fs::read_dir(plugin_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(manifest) = self.read_plugin_manifest(&entry.path()).await {
                plugins.push(manifest);
            }
        }
        
        Ok(plugins)
    }

    /// Read plugin manifest
    async fn read_plugin_manifest(&self, path: &std::path::Path) -> Result<PluginInfo> {
        let manifest_path = path.join("plugin.json");
        let content = tokio::fs::read_to_string(manifest_path).await?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Initialize plugin
    pub async fn initialize_plugin(&self, name: String) -> Result<()> {
        if let Some(mut state) = self.plugins.get_mut(&name) {
            state.status = PluginStatus::Initializing;
            
            // Start plugin process
            let child = tokio::process::Command::new(&state.info.name)
                .spawn()?;
            
            state.pid = Some(child.id().unwrap_or(0));
            state.status = PluginStatus::Running;
        }
        
        Ok(())
    }

    /// Shutdown plugin
    pub async fn shutdown_plugin(&self, name: String) -> Result<()> {
        if let Some(mut state) = self.plugins.get_mut(&name) {
            state.status = PluginStatus::Stopping;
            
            // Send shutdown signal
            if let Some(pid) = state.pid {
                // In production, send proper shutdown signal
                // For now, just mark as stopped
                state.status = PluginStatus::Stopped;
            }
        }
        
        Ok(())
    }

    /// Get plugin status
    pub fn get_plugin_status(&self, name: &str) -> Option<PluginStatus> {
        self.plugins.get(name).map(|state| state.status.clone())
    }
}

/// Message router for dispatching to handlers
pub struct MessageRouter {
    routes: Arc<DashMap<String, mpsc::UnboundedSender<Message>>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(DashMap::new()),
        }
    }

    /// Register route
    pub fn register_route(&self, pattern: String, sender: mpsc::UnboundedSender<Message>) {
        self.routes.insert(pattern, sender);
    }

    /// Route message
    pub async fn route_message(&self, msg: Message) -> Result<()> {
        let method = match &msg {
            Message::Request(req) => &req.method,
            Message::Notification(notif) => &notif.method,
            Message::Response(_) => return Ok(()),
        };
        
        // Find matching route
        for route in self.routes.iter() {
            if method.starts_with(route.key()) {
                route.value().send(msg.clone())?;
                return Ok(());
            }
        }
        
        // No route found
        tracing::warn!("No route found for method: {}", method);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_protocol() {
        let info = PluginInfo {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: "Test plugin".to_string(),
            capabilities: PluginCapabilities {
                code_completion: true,
                hover: true,
                goto_definition: false,
                find_references: false,
                diagnostics: false,
                code_actions: false,
                formatting: false,
                semantic_tokens: false,
            },
        };
        
        let server = PluginProtocolServer::new(info);
        
        // Test initialize request
        let req = RequestMessage {
            id: 1,
            method: "initialize".to_string(),
            params: None,
        };
        
        let response = server.process_message(Message::Request(req)).await.unwrap();
        assert!(response.is_some());
    }
}
