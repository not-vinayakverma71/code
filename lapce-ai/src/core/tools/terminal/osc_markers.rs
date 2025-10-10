// OSC 633/133 terminal markers for command tracking - P1-6
// Enables precise command output capture and status tracking

use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use anyhow::Result;

/// OSC escape sequences for command tracking
pub struct OscMarkers;

impl OscMarkers {
    /// OSC 633 - VS Code/Terminal.app command tracking
    pub const OSC_633_START: &'static str = "\x1b]633;A\x07";
    pub const OSC_633_END: &'static str = "\x1b]633;B\x07";
    pub const OSC_633_COMMAND: &'static str = "\x1b]633;C\x07";
    pub const OSC_633_OUTPUT: &'static str = "\x1b]633;D\x07";
    
    /// OSC 133 - iTerm2 shell integration
    pub const OSC_133_PROMPT_START: &'static str = "\x1b]133;A\x07";
    pub const OSC_133_PROMPT_END: &'static str = "\x1b]133;B\x07";
    pub const OSC_133_COMMAND_START: &'static str = "\x1b]133;C\x07";
    pub const OSC_133_COMMAND_END: &'static str = "\x1b]133;D\x07";
    
    /// Inject markers into command
    pub fn wrap_command(cmd: &str) -> String {
        format!(
            "{}{}{}{}{}",
            Self::OSC_633_START,
            Self::OSC_133_COMMAND_START,
            cmd,
            Self::OSC_133_COMMAND_END,
            Self::OSC_633_END
        )
    }
    
    /// Parse output with markers
    pub fn parse_output(output: &str) -> CommandOutput {
        let mut result = CommandOutput::default();
        let mut current_section = OutputSection::None;
        
        for line in output.lines() {
            if line.contains("\x1b]633;A") || line.contains("\x1b]133;C") {
                current_section = OutputSection::Command;
            } else if line.contains("\x1b]633;D") {
                current_section = OutputSection::Output;
            } else if line.contains("\x1b]633;B") || line.contains("\x1b]133;D") {
                current_section = OutputSection::Complete;
                // Extract exit code if present
                if let Some(code_str) = line.split(';').nth(1) {
                    if let Ok(code) = code_str.trim_end_matches('\x07').parse() {
                        result.exit_code = Some(code);
                    }
                }
            } else {
                // Regular output line
                match current_section {
                    OutputSection::Command => result.command_lines.push(line.to_string()),
                    OutputSection::Output => result.output_lines.push(line.to_string()),
                    _ => {}
                }
            }
        }
        
        result
    }
}

/// Parsed command output
#[derive(Debug, Default)]
pub struct CommandOutput {
    pub command_lines: Vec<String>,
    pub output_lines: Vec<String>,
    pub exit_code: Option<i32>,
}

/// Output section being parsed
#[derive(Debug)]
enum OutputSection {
    None,
    Command,
    Output,
    Complete,
}

/// Enhanced command executor with OSC marker support
pub struct EnhancedExecutor {
    shell: String,
    env_vars: Vec<(String, String)>,
}

impl EnhancedExecutor {
    pub fn new() -> Self {
        Self {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
            env_vars: vec![
                ("TERM".to_string(), "xterm-256color".to_string()),
                ("CLICOLOR_FORCE".to_string(), "1".to_string()),
            ],
        }
    }
    
    /// Execute command with marker tracking
    pub async fn execute_with_markers(
        &self,
        command: &str,
        cwd: Option<&str>,
    ) -> Result<CommandOutput> {
        let wrapped = OscMarkers::wrap_command(command);
        
        let mut cmd = Command::new(&self.shell);
        cmd.arg("-c")
           .arg(&wrapped)
           .stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        
        for (key, val) in &self.env_vars {
            cmd.env(key, val);
        }
        
        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        let mut result = OscMarkers::parse_output(&stdout);
        if result.exit_code.is_none() {
            result.exit_code = output.status.code();
        }
        
        Ok(result)
    }
    
    /// Stream command output with markers
    pub async fn stream_with_markers<F>(
        &self,
        command: &str,
        cwd: Option<&str>,
        mut on_line: F,
    ) -> Result<i32>
    where
        F: FnMut(String) + Send + 'static,
    {
        let wrapped = OscMarkers::wrap_command(command);
        
        let mut cmd = Command::new(&self.shell);
        cmd.arg("-c")
           .arg(&wrapped)
           .stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        
        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout).lines();
        
        while let Some(line) = reader.next_line().await? {
            on_line(line);
        }
        
        let status = child.wait().await?;
        Ok(status.code().unwrap_or(-1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wrap_command() {
        let cmd = "ls -la";
        let wrapped = OscMarkers::wrap_command(cmd);
        assert!(wrapped.contains(cmd));
        assert!(wrapped.contains(OscMarkers::OSC_633_START));
    }
    
    #[test]
    fn test_parse_output() {
        let output = format!(
            "{}ls -la\n{}file1.txt\nfile2.txt\n{}",
            OscMarkers::OSC_633_START,
            OscMarkers::OSC_633_OUTPUT,
            OscMarkers::OSC_633_END
        );
        
        let parsed = OscMarkers::parse_output(&output);
        assert_eq!(parsed.command_lines.len(), 1);
        assert_eq!(parsed.output_lines.len(), 2);
    }
}
