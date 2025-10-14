// TerminalTool Backend - OSC 633/133 markers and command safety
// Part of TerminalTool backend TODO #11

use std::io::{BufRead, BufReader, Write};
use anyhow::{Result, bail, Context};
use serde::{Serialize, Deserialize};
use tokio::process::{Command as TokioCommand};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::expanded_tools_v2::ToolPermissions;
use crate::core::tools::security_hardening::{validate_command_security};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
// OSC sequences for command tracking
const OSC_633_A: &str = "\x1b]633;A\x07";  // Prompt start
const OSC_633_B: &str = "\x1b]633;B\x07";  // Prompt end  
const OSC_633_C: &str = "\x1b]633;C\x07";  // Command start
const OSC_633_D: &str = "\x1b]633;D\x07";  // Command end
const OSC_133_A: &str = "\x1b]133;A\x07";  // Alternative prompt start
const OSC_133_B: &str = "\x1b]133;B\x07";  // Alternative prompt end

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalCommand {
    pub command: String,
    pub cwd: Option<String>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub timeout_ms: Option<u64>,
    pub capture_output: bool,
    pub use_osc_markers: bool,
    #[serde(default)]
    pub allow_dangerous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutput {
    pub command: String,
    pub exit_code: i32,
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub segments: Vec<OutputSegment>,
    pub duration_ms: u64,
    pub was_sanitized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSegment {
    pub segment_type: SegmentType,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegmentType {
    Prompt,
    Command,
    Output,
    Error,
    Marker,
}

pub struct TerminalTool;

#[async_trait]
impl Tool for TerminalTool {
    async fn execute(&self, args: serde_json::Value, context: ToolContext) -> ToolResult {
        let cmd: TerminalCommand = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid terminal command: {}", e)))?;
        
        // Security validation
        let sanitized_command = if !cmd.allow_dangerous {
            validate_command_security(&cmd.command)
                .map_err(|e| ToolError::SecurityViolation(e.to_string()))?
        } else if context.require_approval {
            return Err(ToolError::ApprovalRequired("Dangerous command requires explicit approval".to_string()));
        } else {
            cmd.command.clone()
        };
        
        let was_sanitized = sanitized_command != cmd.command;
        
        // Execute command
        let output = if cmd.use_osc_markers {
            self.execute_with_markers(&sanitized_command, &cmd, &context).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        } else {
            self.execute_simple(&sanitized_command, &cmd, &context).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        };
        
        Ok(ToolOutput {
            success: output.exit_code == 0,
            result: serde_json::to_value(output).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
            error: None,
            metadata: Default::default(),
        })
    }
    
    fn name(&self) -> &'static str {
        "terminal"
    }
    
    fn description(&self) -> &'static str {
        "Execute terminal commands with OSC marker support and safety checks"
    }
    
    // Permissions are handled at the context level
}

impl TerminalTool {
    async fn execute_simple(
        &self,
        command: &str,
        config: &TerminalCommand,
        context: &ToolContext,
    ) -> Result<TerminalOutput> {
        let start_time = std::time::Instant::now();
        
        let mut cmd = TokioCommand::new("sh");
        cmd.arg("-c").arg(command);
        
        if let Some(cwd) = &config.cwd {
            cmd.current_dir(cwd);
        } else {
            cmd.current_dir(&context.workspace);
        }
        
        if let Some(env) = &config.env {
            cmd.envs(env);
        }
        
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .stdin(Stdio::null());
        
        let output = if let Some(timeout_ms) = config.timeout_ms {
            tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                cmd.output()
            ).await
            .map_err(|_| anyhow::anyhow!("Command timed out after {}ms", timeout_ms))?
        } else {
            cmd.output().await
        }?;
        
        let stdout = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|l| l.to_string())
            .collect();
        
        let stderr = String::from_utf8_lossy(&output.stderr)
            .lines()
            .map(|l| l.to_string())
            .collect();
        
        Ok(TerminalOutput {
            command: config.command.clone(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            segments: Vec::new(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            was_sanitized: false,
        })
    }
    
    async fn execute_with_markers(
        &self,
        command: &str,
        config: &TerminalCommand,
        context: &ToolContext,
    ) -> Result<TerminalOutput> {
        let start_time = std::time::Instant::now();
        
        // Wrap command with OSC markers
        let wrapped_command = format!(
            "{}{}{}{}{}",
            OSC_633_A,  // Prompt start
            OSC_633_C,  // Command start
            command,
            OSC_633_D,  // Command end
            OSC_633_B   // Prompt end
        );
        
        let mut cmd = TokioCommand::new("sh");
        cmd.arg("-c").arg(&wrapped_command);
        
        if let Some(cwd) = &config.cwd {
            cmd.current_dir(cwd);
        } else {
            cmd.current_dir(&context.workspace);
        }
        
        if let Some(env) = &config.env {
            cmd.envs(env);
        }
        
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .stdin(Stdio::null());
        
        let mut child = cmd.spawn()?;
        
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        
        let mut stdout_reader = TokioBufReader::new(stdout);
        let mut stderr_reader = TokioBufReader::new(stderr);
        
        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();
        let mut segments = Vec::new();
        
        // Parse output with OSC markers
        let mut current_segment = String::new();
        let mut current_type = SegmentType::Output;
        let mut line_number = 0;
        
        let mut stdout_line = String::new();
        while stdout_reader.read_line(&mut stdout_line).await? > 0 {
            let trimmed = stdout_line.trim_end();
            
            // Check for OSC markers
            if trimmed.contains(OSC_633_A) || trimmed.contains(OSC_133_A) {
                if !current_segment.is_empty() {
                    segments.push(OutputSegment {
                        segment_type: current_type.clone(),
                        content: current_segment.clone(),
                        timestamp: Utc::now(),
                        line_start: line_number,
                        line_end: line_number,
                    });
                }
                current_type = SegmentType::Prompt;
                current_segment.clear();
            } else if trimmed.contains(OSC_633_C) {
                current_type = SegmentType::Command;
            } else if trimmed.contains(OSC_633_D) {
                current_type = SegmentType::Output;
            } else {
                current_segment.push_str(trimmed);
                current_segment.push('\n');
                stdout_lines.push(trimmed.to_string());
            }
            
            line_number += 1;
            stdout_line.clear();
        }
        
        // Capture stderr
        let mut stderr_line = String::new();
        while stderr_reader.read_line(&mut stderr_line).await? > 0 {
            stderr_lines.push(stderr_line.trim_end().to_string());
            stderr_line.clear();
        }
        
        let status = child.wait().await?;
        
        // Add final segment
        if !current_segment.is_empty() {
            segments.push(OutputSegment {
                segment_type: current_type,
                content: current_segment,
                timestamp: Utc::now(),
                line_start: line_number,
                line_end: line_number,
            });
        }
        
        Ok(TerminalOutput {
            command: config.command.clone(),
            exit_code: status.code().unwrap_or(-1),
            stdout: stdout_lines,
            stderr: stderr_lines,
            segments,
            duration_ms: start_time.elapsed().as_millis() as u64,
            was_sanitized: command != config.command,
        })
    }
    
    pub fn segment_output(&self, output: &str) -> Vec<OutputSegment> {
        let mut segments = Vec::new();
        let osc_regex = Regex::new(r"\x1b\](?:633|133);[A-D]\x07").unwrap();
        
        let mut current_segment = String::new();
        let mut current_type = SegmentType::Output;
        let mut last_pos = 0;
        let line_count = output.lines().count();
        
        // Find all OSC markers and split by them
        for marker_match in osc_regex.find_iter(output) {
            // Add content before this marker to current segment
            let content_before = &output[last_pos..marker_match.start()];
            if !content_before.is_empty() {
                current_segment.push_str(content_before);
            }
            
            // Save current segment if not empty
            if !current_segment.is_empty() {
                segments.push(OutputSegment {
                    segment_type: current_type.clone(),
                    content: current_segment.clone(),
                    timestamp: Utc::now(),
                    line_start: 0,
                    line_end: line_count.saturating_sub(1),
                });
                current_segment.clear();
            }
            
            // Determine new segment type based on marker
            current_type = match marker_match.as_str() {
                s if s.contains("A") => SegmentType::Prompt,
                s if s.contains("C") => SegmentType::Command,
                s if s.contains("D") => SegmentType::Output,
                s if s.contains("B") => SegmentType::Output,
                _ => SegmentType::Output,
            };
            
            last_pos = marker_match.end();
        }
        
        // Add remaining content
        if last_pos < output.len() {
            let remaining = &output[last_pos..];
            if !remaining.is_empty() {
                current_segment.push_str(remaining);
            }
        }
        
        // Add final segment
        if !current_segment.is_empty() {
            segments.push(OutputSegment {
                segment_type: current_type,
                content: current_segment,
                timestamp: Utc::now(),
                line_start: 0,
                line_end: line_count.saturating_sub(1),
            });
        }
        
        segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_safe_command() {
        let tool = TerminalTool;
        let temp_dir = TempDir::new().unwrap();
        
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_user".to_string(),
        );
        
        let args = serde_json::json!({
            "command": "echo hello",
            "capture_output": true,
            "use_osc_markers": false,
        });
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let output: TerminalOutput = serde_json::from_value(result.result).unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.iter().any(|l| l.contains("hello")));
    }
    
    #[tokio::test]
    async fn test_dangerous_command_blocked() {
        let tool = TerminalTool;
        let temp_dir = TempDir::new().unwrap();
        
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_user".to_string(),
        );
        
        let args = serde_json::json!({
            "command": "rm -rf /",
            "capture_output": true,
            "use_osc_markers": false,
            "allow_dangerous": false,
        });
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("trash-put"));
    }
    
    #[test]
    fn test_osc_segmentation() {
        let tool = TerminalTool;
        
        let output = format!(
            "{}prompt$ {}ls -la{}\nfile1.txt\nfile2.txt{}",
            OSC_633_A, OSC_633_C, OSC_633_D, OSC_633_B
        );
        
        let segments = tool.segment_output(&output);
        assert!(!segments.is_empty());
        
        // Check for command segment
        assert!(segments.iter().any(|s| matches!(s.segment_type, SegmentType::Command)));
    }
    
    #[tokio::test]
    async fn test_timeout() {
        let tool = TerminalTool;
        let temp_dir = TempDir::new().unwrap();
        
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_user".to_string(),
        );
        
        let args = serde_json::json!({
            "command": "sleep 10",
            "timeout_ms": 100,
            "capture_output": true,
            "use_osc_markers": false,
        });
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }
}
