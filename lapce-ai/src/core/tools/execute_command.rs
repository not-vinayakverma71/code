// ExecuteCommand tool implementation - P0-6

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::xml_util::XmlParser;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct ExecuteCommandTool;

const MAX_OUTPUT_SIZE: usize = 1024 * 1024; // 1MB
const DEFAULT_TIMEOUT_SECS: u64 = 30;

// Dangerous commands that should be blocked
const DANGEROUS_COMMANDS: &[&str] = &[
    "rm", "rmdir", "del", "format", "fdisk", "dd", "mkfs",
    "sudo", "su", "chmod", "chown", "kill", "killall", "pkill",
    "shutdown", "reboot", "halt", "poweroff", "init",
];

#[async_trait]
impl Tool for ExecuteCommandTool {
    fn name(&self) -> &'static str {
        "executeCommand"
    }
    
    fn description(&self) -> &'static str {
        "Execute a shell command with safety checks and output streaming"
    }
    
    fn requires_approval(&self) -> bool {
        true
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        // Parse XML arguments
        let parser = XmlParser::new();
        let parsed = parser.parse(args.as_str().ok_or_else(|| {
            ToolError::InvalidArguments("Expected XML string".to_string())
        })?).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        
        // Extract arguments - handle both flat and nested structures
        let tool_data = if parsed.get("tool").is_some() {
            &parsed["tool"]
        } else {
            &parsed
        };
        
        let command = tool_data["command"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' argument".to_string()))?;
            
        let cwd = tool_data.get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| context.resolve_path(s));
            
        let timeout_secs = tool_data.get("timeout")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        
        // Safety check - block dangerous commands
        let command_parts: Vec<&str> = command.split_whitespace().collect();
        if let Some(cmd) = command_parts.first() {
            let base_cmd = cmd.split('/').last().unwrap_or(cmd);
            if DANGEROUS_COMMANDS.contains(&base_cmd) {
                // Suggest safer alternative for rm
                let suggestion = if base_cmd == "rm" {
                    "\nSuggestion: Use 'trash-put' instead of 'rm' for safer file deletion."
                } else {
                    ""
                };
                
                return Err(ToolError::PermissionDenied(format!(
                    "Command '{}' is potentially dangerous and has been blocked.{}",
                    base_cmd, suggestion
                )));
            }
        }
        
        // Check if cwd is within workspace
        if let Some(ref dir) = cwd {
            super::fs::ensure_workspace_path(&context.workspace, dir)
                .map_err(|e| ToolError::PermissionDenied(e))?;
        }
        
        // Request approval if required
        if context.require_approval && !context.dry_run {
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required to execute command: {}",
                command
            )));
        }
        
        // Dry run mode
        if context.dry_run {
            return Ok(ToolOutput::success(json!({
                "command": command,
                "cwd": cwd.as_ref().map(|p| p.display().to_string()),
                "dryRun": true,
                "wouldExecute": true,
            })));
        }
        
        // Execute the command
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        if let Some(ref dir) = cwd {
            cmd.current_dir(dir);
        }
        
        let mut child = cmd.spawn()
            .map_err(|e| ToolError::Other(format!("Failed to spawn command: {}", e)))?;
        
        // Set up output streaming
        let stdout = child.stdout.take()
            .ok_or_else(|| ToolError::Other("Failed to capture stdout".to_string()))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| ToolError::Other("Failed to capture stderr".to_string()))?;
        
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();
        
        let mut output = Vec::new();
        let mut error_output = Vec::new();
        let mut total_size = 0;
        
        // Stream output with timeout
        let result = timeout(Duration::from_secs(timeout_secs), async {
            loop {
                tokio::select! {
                    line = stdout_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                total_size += line.len();
                                if total_size < MAX_OUTPUT_SIZE {
                                    output.push(line);
                                }
                            }
                            Ok(None) => break,
                            Err(e) => return Err(ToolError::Other(format!("Error reading stdout: {}", e))),
                        }
                    }
                    line = stderr_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                total_size += line.len();
                                if total_size < MAX_OUTPUT_SIZE {
                                    error_output.push(line);
                                }
                            }
                            Ok(None) => {},
                            Err(e) => return Err(ToolError::Other(format!("Error reading stderr: {}", e))),
                        }
                    }
                }
                
                // Check if process has exited
                if let Ok(Some(status)) = child.try_wait() {
                    return Ok(status);
                }
            }
            
            // Wait for process to complete
            child.wait().await
                .map_err(|e| ToolError::Other(format!("Failed to wait for command: {}", e)))
        }).await;
        
        let (exit_status, truncated) = match result {
            Ok(Ok(status)) => (status, total_size >= MAX_OUTPUT_SIZE),
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // Timeout - kill the process
                let _ = child.kill().await;
                return Err(ToolError::Timeout(format!(
                    "Command timed out after {} seconds",
                    timeout_secs
                )));
            }
        };
        
        Ok(ToolOutput::success(json!({
            "command": command,
            "cwd": cwd.as_ref().map(|p| p.display().to_string()),
            "exitCode": exit_status.code(),
            "success": exit_status.success(),
            "stdout": output.join("\n"),
            "stderr": error_output.join("\n"),
            "truncated": truncated,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_execute_command_basic() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = ExecuteCommandTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false; // Disable approval for test
        
        let args = json!(r#"
            <tool>
                <command>echo "Hello, World!"</command>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["exitCode"], 0);
        assert!(result.result["stdout"].as_str().unwrap().contains("Hello, World!"));
    }
    
    #[tokio::test]
    async fn test_execute_command_with_cwd() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        
        let tool = ExecuteCommandTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <command>pwd</command>
                <cwd>subdir</cwd>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert!(result.result["stdout"].as_str().unwrap().contains("subdir"));
    }
    
    #[tokio::test]
    async fn test_execute_command_dangerous_blocked() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = ExecuteCommandTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false;
        
        let args = json!(r#"
            <tool>
                <command>rm -rf /</command>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        
        if let Err(ToolError::PermissionDenied(msg)) = result {
            assert!(msg.contains("dangerous"));
            assert!(msg.contains("trash-put")); // Should suggest safer alternative
        } else {
            panic!("Expected PermissionDenied error");
        }
    }
    
    #[tokio::test]
    async fn test_execute_command_timeout() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = ExecuteCommandTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false;
        
        let args = json!(r#"
            <tool>
                <command>sleep 10</command>
                <timeout>1</timeout>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        
        if let Err(ToolError::Timeout(msg)) = result {
            assert!(msg.contains("timed out"));
        } else {
            panic!("Expected Timeout error");
        }
    }
    
    #[tokio::test]
    async fn test_execute_command_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = ExecuteCommandTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.dry_run = true;
        context.require_approval = false;
        
        let args = json!(r#"
            <tool>
                <command>echo "Should not run"</command>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["dryRun"], true);
        assert_eq!(result.result["wouldExecute"], true);
        // stdout should not be present in dry run
        assert!(result.result.get("stdout").is_none());
    }
}
