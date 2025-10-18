// Terminal Pre-IPC: Command source tagging and state types
// Part of HP1: Command Source Tagging feature

use std::collections::VecDeque;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Source of a terminal command (user-typed vs AI-generated)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandSource {
    /// Command typed by user in terminal
    User,
    /// Command generated/injected by AI (future: via IPC)
    Cascade,
}

impl std::fmt::Display for CommandSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandSource::User => write!(f, "USER"),
            CommandSource::Cascade => write!(f, "CASCADE"),
        }
    }
}

/// A single command execution record with full context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRecord {
    /// The actual command string executed
    pub command: String,
    
    /// Source of the command (User or Cascade)
    pub source: CommandSource,
    
    /// When the command was started (as Unix timestamp seconds)
    pub timestamp: i64,
    
    /// Exit code (None if still running or forced exit)
    pub exit_code: Option<i32>,
    
    /// Captured output (trimmed for memory)
    pub output: String,
    
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    
    /// Working directory when command executed
    pub cwd: PathBuf,
    
    /// Whether this command was force-completed due to timeout
    pub forced_exit: bool,
}

impl CommandRecord {
    /// Create a new command record at start time
    pub fn new(command: String, source: CommandSource, cwd: PathBuf) -> Self {
        Self {
            command,
            source,
            timestamp: Utc::now().timestamp(),
            exit_code: None,
            output: String::new(),
            duration_ms: 0,
            cwd,
            forced_exit: false,
        }
    }
    
    /// Mark command as completed with exit code
    pub fn complete(&mut self, exit_code: i32, output: String, duration_ms: u64) {
        self.exit_code = Some(exit_code);
        self.output = Self::truncate_output(output);
        self.duration_ms = duration_ms;
    }
    
    /// Mark command as force-completed (shell integration timeout)
    pub fn force_complete(&mut self, output: String, duration_ms: u64) {
        self.exit_code = Some(0); // Assume success
        self.output = Self::truncate_output(output);
        self.duration_ms = duration_ms;
        self.forced_exit = true;
    }
    
    /// Truncate output to prevent memory bloat (keep last 10KB)
    fn truncate_output(output: String) -> String {
        const MAX_OUTPUT_SIZE: usize = 10 * 1024; // 10KB
        if output.len() > MAX_OUTPUT_SIZE {
            let start = output.len() - MAX_OUTPUT_SIZE;
            format!("...[truncated]...\n{}", &output[start..])
        } else {
            output
        }
    }
    
    /// Check if command is still running
    pub fn is_running(&self) -> bool {
        self.exit_code.is_none()
    }
    
    /// Get success status (exit code 0)
    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

/// Circular buffer for command history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistory {
    records: VecDeque<CommandRecord>,
    max_size: usize,
}

impl CommandHistory {
    /// Create a new command history with max size
    pub fn new(max_size: usize) -> Self {
        Self {
            records: VecDeque::with_capacity(max_size),
            max_size,
        }
    }
    
    /// Add a new command record
    pub fn push(&mut self, record: CommandRecord) {
        if self.records.len() >= self.max_size {
            self.records.pop_front(); // Remove oldest
        }
        self.records.push_back(record);
    }
    
    /// Get most recent command
    pub fn last(&self) -> Option<&CommandRecord> {
        self.records.back()
    }
    
    /// Get mutable reference to most recent command
    pub fn last_mut(&mut self) -> Option<&mut CommandRecord> {
        self.records.back_mut()
    }
    
    /// Get all commands
    pub fn iter(&self) -> impl Iterator<Item = &CommandRecord> {
        self.records.iter()
    }
    
    /// Get command count by source
    pub fn count_by_source(&self, source: CommandSource) -> usize {
        self.records.iter().filter(|r| r.source == source).count()
    }
    
    /// Get recent commands (last N)
    pub fn recent(&self, count: usize) -> Vec<&CommandRecord> {
        self.records.iter().rev().take(count).collect()
    }
    
    /// Clear all history
    pub fn clear(&mut self) {
        self.records.clear();
    }
    
    /// Get total count
    pub fn len(&self) -> usize {
        self.records.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new(1000) // Default: keep last 1000 commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    
    #[test]
    fn test_command_source_display() {
        assert_eq!(CommandSource::User.to_string(), "USER");
        assert_eq!(CommandSource::Cascade.to_string(), "CASCADE");
    }
    
    #[test]
    fn test_command_record_creation() {
        let cwd = PathBuf::from("/tmp");
        let record = CommandRecord::new(
            "ls -la".to_string(),
            CommandSource::User,
            cwd.clone(),
        );
        
        assert_eq!(record.command, "ls -la");
        assert_eq!(record.source, CommandSource::User);
        assert_eq!(record.cwd, cwd);
        assert!(record.is_running());
        assert!(!record.forced_exit);
    }
    
    #[test]
    fn test_command_completion() {
        let mut record = CommandRecord::new(
            "echo hello".to_string(),
            CommandSource::User,
            PathBuf::from("/tmp"),
        );
        
        record.complete(0, "hello\n".to_string(), 100);
        
        assert_eq!(record.exit_code, Some(0));
        assert_eq!(record.output, "hello\n");
        assert_eq!(record.duration_ms, 100);
        assert!(record.is_success());
        assert!(!record.is_running());
    }
    
    #[test]
    fn test_force_completion() {
        let mut record = CommandRecord::new(
            "sleep 10".to_string(),
            CommandSource::Cascade,
            PathBuf::from("/tmp"),
        );
        
        record.force_complete("partial output".to_string(), 3000);
        
        assert_eq!(record.exit_code, Some(0));
        assert_eq!(record.output, "partial output");
        assert_eq!(record.duration_ms, 3000);
        assert!(record.forced_exit);
    }
    
    #[test]
    fn test_output_truncation() {
        let large_output = "x".repeat(20 * 1024); // 20KB
        let mut record = CommandRecord::new(
            "cat large.txt".to_string(),
            CommandSource::User,
            PathBuf::from("/tmp"),
        );
        
        record.complete(0, large_output, 500);
        
        // Should be truncated to ~10KB
        assert!(record.output.len() < 11 * 1024);
        assert!(record.output.starts_with("...[truncated]..."));
    }
    
    #[test]
    fn test_command_history() {
        let mut history = CommandHistory::new(3);
        let cwd = PathBuf::from("/tmp");
        
        // Add 4 commands (should drop oldest)
        for i in 0..4 {
            let record = CommandRecord::new(
                format!("cmd{}", i),
                if i % 2 == 0 { CommandSource::User } else { CommandSource::Cascade },
                cwd.clone(),
            );
            history.push(record);
        }
        
        assert_eq!(history.len(), 3);
        assert_eq!(history.last().unwrap().command, "cmd3");
        
        // Count by source
        assert_eq!(history.count_by_source(CommandSource::Cascade), 2);
        assert_eq!(history.count_by_source(CommandSource::User), 1);
    }
    
    #[test]
    fn test_command_history_recent() {
        let mut history = CommandHistory::default();
        let cwd = PathBuf::from("/tmp");
        
        for i in 0..10 {
            history.push(CommandRecord::new(
                format!("cmd{}", i),
                CommandSource::User,
                cwd.clone(),
            ));
        }
        
        let recent = history.recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].command, "cmd9");
        assert_eq!(recent[1].command, "cmd8");
        assert_eq!(recent[2].command, "cmd7");
    }
    
    #[test]
    fn test_serialization() {
        let mut history = CommandHistory::new(10);
        let cwd = PathBuf::from("/home/user/project");
        
        let mut record = CommandRecord::new(
            "git status".to_string(),
            CommandSource::User,
            cwd,
        );
        record.complete(0, "On branch main".to_string(), 50);
        
        history.push(record);
        
        // Serialize to JSON
        let json = serde_json::to_string(&history).unwrap();
        
        // Deserialize
        let restored: CommandHistory = serde_json::from_str(&json).unwrap();
        
        assert_eq!(restored.len(), 1);
        assert_eq!(restored.last().unwrap().command, "git status");
        assert_eq!(restored.last().unwrap().source, CommandSource::User);
    }
}
