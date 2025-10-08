// Terminal adapter for Lapce integration - P0-Adapters

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use tokio::sync::mpsc;

/// Terminal execution status for streaming output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandExecutionStatus {
    /// Command execution started
    Started {
        execution_id: String,
        command: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
    },
    
    /// Output line from command
    Output {
        execution_id: String,
        stream: StreamType,
        line: String,
        timestamp: u64,
    },
    
    /// Command execution completed
    Completed {
        execution_id: String,
        exit_code: i32,
        duration_ms: u64,
    },
    
    /// Command execution timeout
    Timeout {
        execution_id: String,
        duration_ms: u64,
    },
}

/// Stream type for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamType {
    Stdout,
    Stderr,
}

/// Terminal adapter for Lapce terminal integration
pub struct TerminalAdapter {
    /// Channel for sending terminal messages
    sender: mpsc::UnboundedSender<CommandExecutionStatus>,
    
    /// Whether to mirror commands to Lapce terminal
    mirror_to_terminal: bool,
}

impl TerminalAdapter {
    /// Create new terminal adapter
    pub fn new(sender: mpsc::UnboundedSender<CommandExecutionStatus>) -> Self {
        Self {
            sender,
            mirror_to_terminal: false,
        }
    }
    
    /// Enable/disable mirroring to Lapce terminal
    pub fn set_mirror(&mut self, mirror: bool) {
        self.mirror_to_terminal = mirror;
    }
    
    /// Emit command started event
    pub fn emit_started(
        &self,
        execution_id: &str,
        command: &str,
        args: &[String],
        cwd: Option<PathBuf>
    ) -> Result<()> {
        let message = CommandExecutionStatus::Started {
            execution_id: execution_id.to_string(),
            command: command.to_string(),
            args: args.to_vec(),
            cwd: cwd.clone(),
        };
        
        self.sender.send(message)?;
        
        if self.mirror_to_terminal {
            self.trigger_terminal_launch(command, args, cwd.as_deref())?;
        }
        
        Ok(())
    }
    
    /// Emit output line
    pub fn emit_output(
        &self,
        execution_id: &str,
        stream: StreamType,
        line: &str
    ) -> Result<()> {
        let message = CommandExecutionStatus::Output {
            execution_id: execution_id.to_string(),
            stream,
            line: line.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Emit command completed
    pub fn emit_completed(
        &self,
        execution_id: &str,
        exit_code: i32,
        duration_ms: u64
    ) -> Result<()> {
        let message = CommandExecutionStatus::Completed {
            execution_id: execution_id.to_string(),
            exit_code,
            duration_ms,
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Emit command timeout
    pub fn emit_timeout(&self, execution_id: &str, duration_ms: u64) -> Result<()> {
        let message = CommandExecutionStatus::Timeout {
            execution_id: execution_id.to_string(),
            duration_ms,
        };
        
        self.sender.send(message)?;
        Ok(())
    }
    
    /// Trigger terminal launch in Lapce (when mirroring is enabled)
    fn trigger_terminal_launch(
        &self,
        program: &str,
        args: &[String],
        cwd: Option<&std::path::Path>
    ) -> Result<()> {
        // This would send an InternalCommand::ExecuteProcess message
        // to Lapce to open a terminal with the given command
        
        // For now, we just prepare the message structure
        // The actual IPC integration will be wired in a later phase
        
        let _terminal_profile = TerminalProfile {
            program: program.to_string(),
            arguments: args.to_vec(),
            workdir: cwd.map(|p| p.to_owned()),
            environment: HashMap::new(),
        };
        
        // TODO: Send InternalCommand::ExecuteProcess via IPC
        // This will be implemented when we wire up the full IPC bridge
        
        Ok(())
    }
}

/// Terminal profile for launching processes
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TerminalProfile {
    program: String,
    arguments: Vec<String>,
    workdir: Option<PathBuf>,
    environment: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_terminal_lifecycle() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let adapter = TerminalAdapter::new(tx);
        
        let execution_id = "test_exec_123";
        
        // Test started event
        adapter.emit_started(
            execution_id,
            "echo",
            &["hello".to_string()],
            Some(PathBuf::from("/tmp"))
        ).unwrap();
        
        let msg = rx.recv().await.unwrap();
        match msg {
            CommandExecutionStatus::Started { command, args, cwd, .. } => {
                assert_eq!(command, "echo");
                assert_eq!(args, vec!["hello"]);
                assert_eq!(cwd, Some(PathBuf::from("/tmp")));
            }
            _ => panic!("Expected Started message"),
        }
        
        // Test output events
        adapter.emit_output(execution_id, StreamType::Stdout, "hello").unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            CommandExecutionStatus::Output { stream, line, .. } => {
                assert!(matches!(stream, StreamType::Stdout));
                assert_eq!(line, "hello");
            }
            _ => panic!("Expected Output message"),
        }
        
        adapter.emit_output(execution_id, StreamType::Stderr, "warning").unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            CommandExecutionStatus::Output { stream, line, .. } => {
                assert!(matches!(stream, StreamType::Stderr));
                assert_eq!(line, "warning");
            }
            _ => panic!("Expected Output message"),
        }
        
        // Test completed event
        adapter.emit_completed(execution_id, 0, 150).unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            CommandExecutionStatus::Completed { exit_code, duration_ms, .. } => {
                assert_eq!(exit_code, 0);
                assert_eq!(duration_ms, 150);
            }
            _ => panic!("Expected Completed message"),
        }
    }
    
    #[tokio::test]
    async fn test_terminal_timeout() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let adapter = TerminalAdapter::new(tx);
        
        let execution_id = "timeout_test";
        
        adapter.emit_timeout(execution_id, 5000).unwrap();
        let msg = rx.recv().await.unwrap();
        match msg {
            CommandExecutionStatus::Timeout { duration_ms, .. } => {
                assert_eq!(duration_ms, 5000);
            }
            _ => panic!("Expected Timeout message"),
        }
    }
    
    #[test]
    fn test_mirror_setting() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut adapter = TerminalAdapter::new(tx);
        
        assert!(!adapter.mirror_to_terminal);
        
        adapter.set_mirror(true);
        assert!(adapter.mirror_to_terminal);
        
        adapter.set_mirror(false);
        assert!(!adapter.mirror_to_terminal);
    }
}
