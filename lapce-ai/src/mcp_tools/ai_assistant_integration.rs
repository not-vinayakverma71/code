/// AI Assistant Integration for MCP Tools
/// Connects MCP tools to Lapce's AI assistant for seamless tool execution

use crate::mcp_tools::{
    dispatcher::McpToolSystem,
    config::McpServerConfig,
    ipc_integration::{McpIpcHandler, McpRequest, McpResponse},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use anyhow::{Result, bail};

/// AI Assistant Tool Executor
/// Handles tool execution requests from the AI assistant
pub struct AiAssistantToolExecutor {
    mcp_handler: Arc<McpIpcHandler>,
    tool_queue: mpsc::Sender<ToolRequest>,
    active_sessions: Arc<RwLock<HashMap<String, SessionContext>>>,
}

/// Tool request from AI assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub session_id: String,
    pub tool_name: String,
    pub parameters: Value,
    pub context: Option<Value>,
    pub priority: ToolPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolPriority {
    High,
    Normal,
    Low,
}

/// Session context for tracking AI assistant interactions
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub session_id: String,
    pub user_id: Option<String>,
    pub workspace: PathBuf,
    pub conversation_history: Vec<ConversationTurn>,
    pub tool_execution_history: Vec<ToolExecution>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub tool_name: String,
    pub parameters: Value,
    pub result: Value,
    pub success: bool,
    pub execution_time_ms: u64,
    pub timestamp: u64,
}

/// Tool execution result for AI assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolResult {
    pub success: bool,
    pub output: Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, Value>,
}

impl AiAssistantToolExecutor {
    /// Create new AI assistant tool executor
    pub fn new(config: McpServerConfig, workspace: PathBuf) -> Self {
        let mcp_handler = Arc::new(McpIpcHandler::new(config, workspace));
        let (tx, mut rx) = mpsc::channel::<ToolRequest>(100);
        
        // Spawn queue processor
        let handler_clone = mcp_handler.clone();
        tokio::spawn(async move {
            while let Some(request) = rx.recv().await {
                let _ = Self::process_queued_request(handler_clone.clone(), request).await;
            }
        });
        
        Self {
            mcp_handler,
            tool_queue: tx,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Execute tool for AI assistant
    pub async fn execute_tool(
        &self,
        session_id: String,
        tool_name: String,
        parameters: Value,
    ) -> Result<AiToolResult> {
        let start_time = std::time::Instant::now();
        
        // Get or create session context
        let mut sessions = self.active_sessions.write().await;
        let session = sessions.entry(session_id.clone())
            .or_insert_with(|| SessionContext {
                session_id: session_id.clone(),
                user_id: None,
                workspace: PathBuf::from("."),
                conversation_history: Vec::new(),
                tool_execution_history: Vec::new(),
                metadata: HashMap::new(),
            });
        
        // Execute tool through MCP handler
        let request_id = 0u64;
        let response = self.mcp_handler.handle_request(
            request_id,
            McpRequest::ExecuteTool {
                tool_name: tool_name.clone(),
                args: parameters.clone(),
            }
        ).await;
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Process response
        let result = match response {
            McpResponse::ToolResult { success, output, error } => {
                // Record execution in history
                session.tool_execution_history.push(ToolExecution {
                    tool_name,
                    parameters,
                    result: output.clone(),
                    success,
                    execution_time_ms,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                });
                
                AiToolResult {
                    success,
                    output,
                    error: error.and_then(|e| e.as_str().map(String::from)),
                    execution_time_ms,
                    metadata: HashMap::new(),
                }
            },
            McpResponse::Error { message } => {
                AiToolResult {
                    success: false,
                    output: json!({ "error": message }),
                    error: Some(message),
                    execution_time_ms,
                    metadata: HashMap::new(),
                }
            },
            _ => {
                AiToolResult {
                    success: false,
                    output: json!({ "error": "Unexpected response type" }),
                    error: Some("Unexpected response type".to_string()),
                    execution_time_ms,
                    metadata: HashMap::new(),
                }
            }
        };
        
        Ok(result)
    }
    
    /// Queue tool execution (for background processing)
    pub async fn queue_tool_execution(&self, request: ToolRequest) -> Result<()> {
        self.tool_queue.send(request).await
            .map_err(|e| anyhow::anyhow!("Failed to queue tool request: {}", e))
    }
    
    /// Process queued request
    async fn process_queued_request(
        handler: Arc<McpIpcHandler>,
        request: ToolRequest,
    ) -> Result<()> {
        let request_id = 0u64;
        handler.handle_request(
            request_id,
            McpRequest::ExecuteTool {
                tool_name: request.tool_name,
                args: request.parameters,
            }
        ).await;
        Ok(())
    }
    
    /// Get session context
    pub async fn get_session(&self, session_id: &str) -> Option<SessionContext> {
        let sessions = self.active_sessions.read().await;
        sessions.get(session_id).cloned()
    }
    
    /// Update session context
    pub async fn update_session(&self, session: SessionContext) {
        let mut sessions = self.active_sessions.write().await;
        sessions.insert(session.session_id.clone(), session);
    }
    
    /// Clear session
    pub async fn clear_session(&self, session_id: &str) {
        let mut sessions = self.active_sessions.write().await;
        sessions.remove(session_id);
    }
    
    /// Get tool suggestions based on context
    pub async fn get_tool_suggestions(&self, context: &str) -> Vec<ToolSuggestion> {
        let mut suggestions = Vec::new();
        
        // Analyze context and suggest appropriate tools
        if context.contains("read") || context.contains("file") {
            suggestions.push(ToolSuggestion {
                tool_name: "readFile".to_string(),
                description: "Read contents of a file".to_string(),
                confidence: 0.9,
            });
        }
        
        if context.contains("write") || context.contains("save") {
            suggestions.push(ToolSuggestion {
                tool_name: "writeFile".to_string(),
                description: "Write content to a file".to_string(),
                confidence: 0.85,
            });
        }
        
        if context.contains("run") || context.contains("execute") || context.contains("command") {
            suggestions.push(ToolSuggestion {
                tool_name: "executeCommand".to_string(),
                description: "Execute a shell command".to_string(),
                confidence: 0.8,
            });
        }
        
        if context.contains("search") || context.contains("find") {
            suggestions.push(ToolSuggestion {
                tool_name: "searchFiles".to_string(),
                description: "Search for patterns in files".to_string(),
                confidence: 0.75,
            });
        }
        
        if context.contains("list") || context.contains("directory") {
            suggestions.push(ToolSuggestion {
                tool_name: "listFiles".to_string(),
                description: "List files in a directory".to_string(),
                confidence: 0.7,
            });
        }
        
        suggestions
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSuggestion {
    pub tool_name: String,
    pub description: String,
    pub confidence: f32,
}

/// AI Assistant Message Handler
/// Processes AI assistant messages and executes tools as needed
pub struct AiMessageHandler {
    executor: Arc<AiAssistantToolExecutor>,
}

impl AiMessageHandler {
    pub fn new(executor: Arc<AiAssistantToolExecutor>) -> Self {
        Self { executor }
    }
    
    /// Process AI assistant message
    pub async fn process_message(&self, message: AiMessage) -> Result<AiResponse> {
        match message {
            AiMessage::UserQuery { session_id, query } => {
                // Analyze query and determine if tools are needed
                let suggestions = self.executor.get_tool_suggestions(&query).await;
                
                if !suggestions.is_empty() {
                    // Return tool suggestions
                    Ok(AiResponse::ToolSuggestions {
                        suggestions,
                        original_query: query,
                    })
                } else {
                    // No tools needed
                    Ok(AiResponse::TextResponse {
                        content: "I can help with that. What would you like to do?".to_string(),
                        metadata: HashMap::new(),
                    })
                }
            },
            
            AiMessage::ToolExecution { session_id, tool_name, parameters } => {
                // Execute tool
                let result = self.executor.execute_tool(
                    session_id,
                    tool_name,
                    parameters,
                ).await?;
                
                Ok(AiResponse::ToolResult(result))
            },
            
            AiMessage::SystemCommand { command } => {
                // Handle system commands
                self.handle_system_command(command).await
            },
        }
    }
    
    async fn handle_system_command(&self, command: String) -> Result<AiResponse> {
        match command.as_str() {
            "list_tools" => {
                let request_id = 0u64;
                let response = self.executor.mcp_handler.handle_request(
                    request_id,
                    McpRequest::ListTools,
                ).await;
                
                if let McpResponse::ToolList { tools } = response {
                    Ok(AiResponse::TextResponse {
                        content: format!("Available tools: {:?}", tools),
                        metadata: HashMap::new(),
                    })
                } else {
                    Ok(AiResponse::Error {
                        message: "Failed to list tools".to_string(),
                    })
                }
            },
            _ => {
                Ok(AiResponse::Error {
                    message: format!("Unknown command: {}", command),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiMessage {
    UserQuery {
        session_id: String,
        query: String,
    },
    ToolExecution {
        session_id: String,
        tool_name: String,
        parameters: Value,
    },
    SystemCommand {
        command: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiResponse {
    TextResponse {
        content: String,
        metadata: HashMap<String, Value>,
    },
    ToolSuggestions {
        suggestions: Vec<ToolSuggestion>,
        original_query: String,
    },
    ToolResult(AiToolResult),
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_ai_assistant_executor() {
        let config = McpServerConfig::default();
        let workspace = tempdir().unwrap().path().to_path_buf();
        let executor = AiAssistantToolExecutor::new(config, workspace.clone());
        
        // Create test file
        std::fs::write(workspace.join("test.txt"), "AI Test").unwrap();
        
        // Execute tool through AI assistant
        let result = executor.execute_tool(
            "test_session".to_string(),
            "readFile".to_string(),
            json!({ "path": "test.txt" }),
        ).await.unwrap();
        
        assert!(result.success);
    }
    
    #[tokio::test]
    async fn test_tool_suggestions() {
        let config = McpServerConfig::default();
        let workspace = tempdir().unwrap().path().to_path_buf();
        let executor = AiAssistantToolExecutor::new(config, workspace);
        
        let suggestions = executor.get_tool_suggestions("I need to read a file").await;
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.tool_name == "readFile"));
    }
}
