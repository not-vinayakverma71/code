use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use anyhow::{Result, bail};
use tokio::process::Command;

use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ToolParameter},
    permissions::Permission,
};
pub struct ExecuteCommandTool;
impl ExecuteCommandTool {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl Tool for ExecuteCommandTool {
    fn name(&self) -> &str {
        "executeCommand"
    
    fn description(&self) -> &str {
        "Execute shell command"
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 30)"
                }
            },
            "required": ["command"]
        })
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("command").is_none() {
            bail!("Missing required parameter: command");
        }
        Ok(())
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        // REAL IMPLEMENTATION - Actually execute commands
        let command_str = args.get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: command"))?;
        
        // Get optional timeout
        let timeout_secs = args.get("timeout")
            .and_then(|t| t.as_u64())
            .unwrap_or(30);
        // Security checks - prevent dangerous commands
        let dangerous_patterns = [
            "rm -rf /", "rm -rf /*", "dd if=/dev", "mkfs", "format /",
            "> /dev/sd", "chmod -R 777 /", ":(){ :|:& };:", // fork bomb
            "mv / ", "shred /", ">" // redirect to overwrite
        ];
        for pattern in &dangerous_patterns {
            if command_str.contains(pattern) {
                return Ok(ToolResult {
                    success: false,
                    data: Some(json!({ 
                        "error": format!("Command blocked: contains dangerous pattern '{}'", pattern)
                    })),
                    error: Some("Security violation".to_string()),
                });
            }
        // Check for sudo/privilege escalation
        if command_str.starts_with("sudo") || command_str.contains("| sudo") {
            return Ok(ToolResult {
                success: false,
                data: Some(json!({ "error": "Sudo commands are not allowed" })),
                error: Some("Permission denied".to_string()),
            });
        // ACTUALLY EXECUTE THE COMMAND
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
           .arg(command_str)
           .current_dir(&context.workspace)
           .stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        // Set environment variables for safety
        cmd.env("SAFE_MODE", "true");
        // Execute with timeout
        let timeout = std::time::Duration::from_secs(timeout_secs);
        match tokio::time::timeout(timeout, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);
                
                // Limit output size to prevent memory issues
                const MAX_OUTPUT_SIZE: usize = 1_000_000; // 1MB
                let stdout_truncated = if stdout.len() > MAX_OUTPUT_SIZE {
                    format!("{}... (truncated, {} bytes total)", 
                            &stdout[..MAX_OUTPUT_SIZE], 
                            stdout.len())
                } else {
                    stdout.to_string()
                };
                let stderr_truncated = if stderr.len() > MAX_OUTPUT_SIZE {
                            &stderr[..MAX_OUTPUT_SIZE], 
                            stderr.len())
                    stderr.to_string()
                Ok(ToolResult::success(json!({
                    "command": command_str,
                    "stdout": stdout_truncated,
                    "stderr": stderr_truncated,
                    "exit_code": exit_code,
                    "success": output.status.success(),
                    "executed_in": context.workspace.display().to_string()
                })))
            Ok(Err(e)) => {
                Ok(ToolResult {
                        "error": format!("Failed to execute command: {}", e),
                        "command": command_str
                    error: Some(e.to_string()),
                })
            Err(_) => {
                        "error": format!("Command timed out after {} seconds", timeout_secs),
                    error: Some("Timeout".to_string()),
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::Execute]
