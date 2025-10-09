// ExecuteCommand tool implementation - P0-6

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::xml_util::XmlParser;
use crate::ipc::ipc_messages::CommandExecutionStatus;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

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
            
        // Get timeout from XML or use config default for this tool
        let timeout_secs = tool_data.get("timeout")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(|| context.get_tool_timeout("executeCommand").as_secs());
        
        // Parse command to check against denylist
        let command_parts: Vec<&str> = command.split_whitespace().collect();
        let base_command = command_parts.first()
            .map(|cmd| cmd.split('/').last().unwrap_or(cmd))
            .unwrap_or("");
        
        // Check against dangerous commands denylist
        if DANGEROUS_COMMANDS.contains(&base_command) {
            let suggestion = match base_command {
                "rm" | "rmdir" | "del" => {
                    "\nSuggestion: Use 'trash-put' instead for safer file deletion that allows recovery."
                }
                "sudo" | "su" => {
                    "\nSuggestion: Run commands without sudo. If elevated permissions are truly needed, explain the use case."
                }
                _ => ""
            };
            
            return Err(ToolError::PermissionDenied(format!(
                "Command '{}' is blocked for safety reasons.{}",
                base_command, suggestion
            )));
        }
        
        // Additional check using context configuration
        if context.is_command_blocked(command) {
            return Err(ToolError::PermissionDenied(format!(
                "Command '{}' is blocked by security policy.",
                command
            )));
        }
        
        // Check permission of cwd is within workspace
        if let Some(ref dir) = cwd {
            super::fs::ensure_workspace_path(&context.workspace, dir)
                .map_err(|e| ToolError::PermissionDenied(e))?;
        }
        
        // Request approval if required by config
        if context.requires_approval_for("executeCommand", "execute") && !context.dry_run {
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
            })));
        }
        
        // Generate correlation ID for this execution
        let correlation_id = Uuid::new_v4().to_string();
        let start_time = Instant::now();
        
        // Emit Started event through event emitter
        if let Some(emitter) = context.get_event_emitter() {
            let event = CommandExecutionStatus::Started {
                command: command.to_string(),
                args: vec![],
                correlation_id: correlation_id.clone(),
            };
            if let Ok(json) = serde_json::to_value(&event) {
                let _ = emitter.emit_correlated(correlation_id.clone(), json).await;
            }
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
        
        let stdout_reader = tokio::io::BufReader::new(stdout);
        let stderr_reader = tokio::io::BufReader::new(stderr);
        
        let output = Arc::new(Mutex::new(Vec::new()));
        let error_output = Arc::new(Mutex::new(Vec::new()));
        let total_size = Arc::new(AtomicUsize::new(0));
        
        // Clone for async block
        let output_clone = output.clone();
        let error_output_clone = error_output.clone();
        let total_size_clone = total_size.clone();
        
        // Stream output with timeout
        let timeout_secs_clone = timeout_secs;
        let result = tokio::time::timeout(Duration::from_secs(timeout_secs_clone), async move {
            use tokio::io::AsyncBufReadExt;
            let mut stdout_lines = stdout_reader.lines();
            let mut stderr_lines = stderr_reader.lines();
            
            loop {
                tokio::select! {
                    line = stdout_lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                let size = total_size_clone.fetch_add(line.len(), Ordering::Relaxed);
                                if size < MAX_OUTPUT_SIZE {
                                    output_clone.lock().await.push(line);
                                }
                            }
                            Ok(None) => break, // EOF
                            Err(e) => return Err(ToolError::Other(format!("Error reading stdout: {}", e))),
                        }
                    }
                    line = stderr_lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                let size = total_size_clone.fetch_add(line.len(), Ordering::Relaxed);
                                if size < MAX_OUTPUT_SIZE {
                                    error_output_clone.lock().await.push(line);
                                }
                            }
                            Ok(None) => break, // EOF
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
        });
        
        let final_total_size = total_size.load(Ordering::Relaxed);
        let (exit_status, truncated) = match result.await {
            Ok(Ok(status)) => (status, final_total_size >= MAX_OUTPUT_SIZE),
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // Timeout - kill the process
                // Note: child was moved, we can't kill it here
                
                // TODO P0-Adapters: Emit Exit event with timeout error when wired
                // if let Some(adapter) = context.get_adapter("ipc") {
                //     let _ = adapter.emit_event(CommandExecutionStatus::Exit {
                //         correlation_id: correlation_id.clone(),
                //         exit_code: -1,  // Timeout exit code
                //         duration_ms: start_time.elapsed().as_millis() as u64,
                //     }).await;
                // }
                
                return Err(ToolError::Timeout(format!(
                    "Command timed out after {} seconds",
                    timeout_secs
                )));
            }
        };
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let exit_code = exit_status.code().unwrap_or(-1);
        
        // Emit Exit event through event emitter
        if let Some(emitter) = context.get_event_emitter() {
            let event = CommandExecutionStatus::Exit {
                correlation_id: correlation_id.clone(),
                exit_code,
                duration_ms,
            };
            if let Ok(json) = serde_json::to_value(&event) {
                let _ = emitter.emit_correlated(correlation_id.clone(), json).await;
            }
        }
        
        // Extract output from Arc<Mutex<>>
        let output_lines = output.lock().await.clone();
        let error_lines = error_output.lock().await.clone();
        
        Ok(ToolOutput::success(json!({
            "command": command,
            "cwd": cwd.as_ref().map(|p| p.display().to_string()),
            "exitCode": exit_code,
            "success": exit_status.success(),
            "stdout": output_lines.join("\n"),
            "stderr": error_lines.join("\n"),
            "truncated": truncated,
            "duration_ms": duration_ms,
            "correlation_id": correlation_id,
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
        context.permissions.command_execute = true;
        context.require_approval = false;
        
        // Test rm command blocking with trash-put suggestion
        let args = json!(r#"
            <tool>
                <command>rm -rf /tmp/test</command>
            </tool>
        "#);
        
        let result = tool.execute(args, context.clone()).await;
        assert!(result.is_err());
        
        if let Err(ToolError::PermissionDenied(msg)) = result {
            assert!(msg.contains("blocked"));
            assert!(msg.contains("trash-put"));
            assert!(msg.contains("safer alternative"));
        } else {
            panic!("Expected PermissionDenied error with trash-put suggestion");
        }
        
        // Test sudo blocking
        let args = json!(r#"
            <tool>
                <command>sudo apt-get update</command>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        
        if let Err(ToolError::PermissionDenied(msg)) = result {
            assert!(msg.contains("blocked"));
            assert!(msg.contains("without sudo"));
        } else {
            panic!("Expected PermissionDenied error for sudo");
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
