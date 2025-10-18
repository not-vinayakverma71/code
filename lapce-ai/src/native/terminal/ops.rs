// Native terminal operations - Direct system execution without MCP overhead
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::{Result, bail};
use std::path::PathBuf;
use dashmap::DashMap;
use uuid::Uuid;

// Define types locally to avoid MCP dependencies
pub type SessionId = String;
// TerminalTool from doc lines 357-404
#[derive(Clone)]
pub struct TerminalSession {
    id: SessionId,
    env_vars: HashMap<String, String>,
    working_dir: std::path::PathBuf,
}

pub struct TerminalTool {
    sessions: DashMap<SessionId, TerminalSession>,
}

impl TerminalTool {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }
    
    async fn create_session(&self, args: Value, workspace: PathBuf) -> Result<Value> {
        let session_id = Uuid::new_v4().to_string();
        
        let env_vars = if let Some(env) = args["env"].as_object() {
            env.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect()
        } else {
            HashMap::new()
        };
        let session = TerminalSession {
            id: session_id.clone(),
            env_vars,
            working_dir: workspace.clone(),
        };
        
        self.sessions.insert(session_id.clone(), session);
        
        Ok(json!({
            "session_id": session_id,
            "status": "created"
        }))
    }
    
    async fn execute_command(&self, args: Value) -> Result<Value> {
        let session_id = args["session_id"].as_str()
            .ok_or_else(|| anyhow::anyhow!("session_id required"))?;
        let command = args["command"].as_str()
            .ok_or_else(|| anyhow::anyhow!("command required"))?;
        let session = self.sessions.get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
            
        // Execute command directly using tokio
        use tokio::process::Command;
        
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.current_dir(&session.working_dir);
        
        for (key, value) in &session.env_vars {
            cmd.env(key, value);
        }
        
        let output = cmd.output().await?;
        
        Ok(json!({
            "session_id": session_id,
            "output": String::from_utf8_lossy(&output.stdout).to_string(),
            "error": String::from_utf8_lossy(&output.stderr).to_string(),
            "exit_code": output.status.code().unwrap_or(-1)
        }))
    }
    
    async fn read_output(&self, args: Value) -> Result<Value> {
        let session_id = args["session_id"].as_str()
            .ok_or_else(|| anyhow::anyhow!("session_id required"))?;
        if !self.sessions.contains_key(session_id) {
            return Err(anyhow::anyhow!("Session not found"));
        }
        
        // In production, this would read from a buffer of terminal output
        Ok(json!({
            "session_id": session_id,
            "output": "",
            "has_more": false
        }))
    }
    
    async fn close_session(&self, args: Value) -> Result<Value> {
        let session_id = args["session_id"].as_str()
            .ok_or_else(|| anyhow::anyhow!("session_id required"))?;
        if self.sessions.remove(session_id).is_some() {
            Ok(json!({
                "session_id": session_id,
                "status": "closed"
            }))
        } else {
            Err(anyhow::anyhow!("Session not found"))
        }
    }
}

// TerminalOperations struct
pub struct TerminalOperations;

// Native implementation - no MCP Tool trait needed
impl TerminalTool {
    pub async fn execute(&self, args: Value, workspace: PathBuf) -> Result<Value> {
        let operation = args["operation"].as_str()
            .ok_or_else(|| anyhow::anyhow!("operation required"))?;
        match operation {
            "create" => self.create_session(args, workspace).await,
            "execute" => self.execute_command(args).await,
            "read" => self.read_output(args).await,
            "close" => self.close_session(args).await,
            _ => Err(anyhow::anyhow!("Unknown operation: {}", operation)),
        }
    }
}
