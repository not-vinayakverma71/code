// Terminal Pre-IPC: AI command injection APIs
// Part of HP1-3: Command injection and control

use std::path::PathBuf;
use anyhow::{Result, anyhow};

use super::types::{CommandRecord, CommandSource};

/// Command injection request
#[derive(Debug, Clone)]
pub struct InjectionRequest {
    /// The command to inject
    pub command: String,
    
    /// Working directory context
    pub cwd: PathBuf,
    
    /// Source identifier
    pub source: CommandSource,
}

impl InjectionRequest {
    /// Create a new injection request from AI
    pub fn from_ai(command: String, cwd: PathBuf) -> Self {
        Self {
            command,
            cwd,
            source: CommandSource::Cascade,
        }
    }
    
    /// Create a new injection request from user
    pub fn from_user(command: String, cwd: PathBuf) -> Self {
        Self {
            command,
            cwd,
            source: CommandSource::User,
        }
    }
    
    /// Validate the injection request
    pub fn validate(&self) -> Result<()> {
        // Check command is not empty
        if self.command.trim().is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }
        
        // Check for dangerous patterns (basic safety)
        let dangerous_patterns = [
            "rm -rf /",
            "mkfs",
            "dd if=/dev/zero",
            ":(){:|:&};:",  // Fork bomb
            "chmod -R 777 /",
        ];
        
        let cmd_lower = self.command.to_lowercase();
        for pattern in &dangerous_patterns {
            if cmd_lower.contains(pattern) {
                return Err(anyhow!("Command contains dangerous pattern: {}", pattern));
            }
        }
        
        Ok(())
    }
    
    /// Format command for injection with newline
    pub fn format_for_injection(&self) -> String {
        format!("{}\n", self.command.trim())
    }
}

/// Control signal for terminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlSignal {
    /// Ctrl+C (SIGINT)
    Interrupt,
    
    /// Ctrl+D (EOF)
    EndOfFile,
    
    /// Ctrl+Z (SIGTSTP)
    Suspend,
}

impl ControlSignal {
    /// Get the control character bytes
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            ControlSignal::Interrupt => b"\x03",      // Ctrl+C
            ControlSignal::EndOfFile => b"\x04",      // Ctrl+D
            ControlSignal::Suspend => b"\x1a",        // Ctrl+Z
        }
    }
    
    /// Get human-readable name
    pub fn name(&self) -> &str {
        match self {
            ControlSignal::Interrupt => "Ctrl+C (Interrupt)",
            ControlSignal::EndOfFile => "Ctrl+D (EOF)",
            ControlSignal::Suspend => "Ctrl+Z (Suspend)",
        }
    }
}

/// Safety checker for terminal commands
pub struct CommandSafety;

impl CommandSafety {
    /// Check if command is safe to execute automatically
    pub fn is_safe_to_auto_execute(command: &str) -> bool {
        let cmd = command.trim().to_lowercase();
        
        // Explicitly unsafe patterns
        let unsafe_patterns = [
            "rm ", "trash-put", "mv ", "cp ",
            "chmod", "chown",
            "mkfs", "fdisk", "parted",
            "kill", "killall", "pkill",
            "reboot", "shutdown", "halt",
            "dd ", "shred",
            "sudo ", "su ",
            ">", ">>",  // File redirection
            "|",        // Piping (could hide dangerous commands)
        ];
        
        for pattern in &unsafe_patterns {
            if cmd.contains(pattern) {
                return false;
            }
        }
        
        // Safe read-only commands
        let safe_commands = [
            "ls", "pwd", "echo", "cat", "less", "more",
            "head", "tail", "grep", "find", "wc",
            "date", "cal", "uptime", "whoami",
            "git status", "git log", "git diff",
        ];
        
        for safe_cmd in &safe_commands {
            if cmd.starts_with(safe_cmd) {
                return true;
            }
        }
        
        // Default to unsafe for unknown commands
        false
    }
    
    /// Suggest safer alternative for dangerous commands
    pub fn suggest_safer_alternative(command: &str) -> Option<String> {
        let cmd = command.trim().to_lowercase();
        
        // Check more specific patterns first
        if cmd.contains("rm -rf") {
            return Some("DANGER: 'rm -rf' can permanently delete files. Use 'trash-put' instead".to_string());
        }
        
        if cmd.starts_with("rm ") {
            return Some("Use 'trash-put' instead of 'rm' for safer deletion".to_string());
        }
        
        if cmd.starts_with("sudo ") {
            return Some("Command requires sudo - review carefully before executing".to_string());
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_injection_request_from_ai() {
        let req = InjectionRequest::from_ai(
            "ls -la".to_string(),
            PathBuf::from("/tmp"),
        );
        
        assert_eq!(req.command, "ls -la");
        assert_eq!(req.source, CommandSource::Cascade);
        assert!(req.validate().is_ok());
    }
    
    #[test]
    fn test_injection_request_validation_empty() {
        let req = InjectionRequest::from_ai(
            "   ".to_string(),
            PathBuf::from("/tmp"),
        );
        
        assert!(req.validate().is_err());
    }
    
    #[test]
    fn test_injection_request_validation_dangerous() {
        let dangerous_cmds = vec![
            "rm -rf /",
            "mkfs.ext4 /dev/sda",
            "dd if=/dev/zero of=/dev/sda",
            ":(){:|:&};:",
        ];
        
        for cmd in dangerous_cmds {
            let req = InjectionRequest::from_ai(
                cmd.to_string(),
                PathBuf::from("/tmp"),
            );
            assert!(req.validate().is_err(), "Should reject: {}", cmd);
        }
    }
    
    #[test]
    fn test_injection_request_format() {
        let req = InjectionRequest::from_ai(
            "echo hello".to_string(),
            PathBuf::from("/tmp"),
        );
        
        assert_eq!(req.format_for_injection(), "echo hello\n");
        
        // Test with trailing whitespace
        let req2 = InjectionRequest::from_ai(
            "ls -la  \n\n".to_string(),
            PathBuf::from("/tmp"),
        );
        assert_eq!(req2.format_for_injection(), "ls -la\n");
    }
    
    #[test]
    fn test_control_signal_bytes() {
        assert_eq!(ControlSignal::Interrupt.as_bytes(), b"\x03");
        assert_eq!(ControlSignal::EndOfFile.as_bytes(), b"\x04");
        assert_eq!(ControlSignal::Suspend.as_bytes(), b"\x1a");
    }
    
    #[test]
    fn test_control_signal_name() {
        assert_eq!(ControlSignal::Interrupt.name(), "Ctrl+C (Interrupt)");
        assert_eq!(ControlSignal::EndOfFile.name(), "Ctrl+D (EOF)");
        assert_eq!(ControlSignal::Suspend.name(), "Ctrl+Z (Suspend)");
    }
    
    #[test]
    fn test_command_safety_safe_commands() {
        let safe_commands = vec![
            "ls -la",
            "pwd",
            "echo hello",
            "cat file.txt",
            "git status",
            "grep pattern file.txt",
        ];
        
        for cmd in safe_commands {
            assert!(
                CommandSafety::is_safe_to_auto_execute(cmd),
                "Should be safe: {}",
                cmd
            );
        }
    }
    
    #[test]
    fn test_command_safety_unsafe_commands() {
        let unsafe_commands = vec![
            "rm file.txt",
            "sudo apt update",
            "chmod 777 file.txt",
            "echo hello > file.txt",
            "ls | grep foo",
            "kill -9 1234",
        ];
        
        for cmd in unsafe_commands {
            assert!(
                !CommandSafety::is_safe_to_auto_execute(cmd),
                "Should be unsafe: {}",
                cmd
            );
        }
    }
    
    #[test]
    fn test_command_safety_suggestions() {
        let suggestion = CommandSafety::suggest_safer_alternative("rm file.txt");
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("trash-put"));
        
        let suggestion = CommandSafety::suggest_safer_alternative("rm -rf /tmp/old");
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("DANGER"));
        
        let suggestion = CommandSafety::suggest_safer_alternative("sudo apt install foo");
        assert!(suggestion.is_some());
        assert!(suggestion.unwrap().contains("sudo"));
        
        let suggestion = CommandSafety::suggest_safer_alternative("ls -la");
        assert!(suggestion.is_none());
    }
}
