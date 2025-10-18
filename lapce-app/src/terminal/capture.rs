// Terminal Pre-IPC: Command capture and tracking
// Part of HP1-2: PTY User Input Capture

use std::path::PathBuf;
use std::time::Instant;

use super::types::{CommandRecord, CommandSource};

/// Tracks partial commands being typed by the user
#[derive(Debug, Clone)]
pub struct CommandCapture {
    /// Current partial command being built
    buffer: String,
    
    /// When the current command started
    started_at: Option<Instant>,
    
    /// Current working directory
    cwd: PathBuf,
}

impl CommandCapture {
    /// Create a new command capture
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            buffer: String::new(),
            started_at: None,
            cwd,
        }
    }
    
    /// Update the working directory
    pub fn set_cwd(&mut self, cwd: PathBuf) {
        self.cwd = cwd;
    }
    
    /// Process input bytes and detect completed commands
    /// Returns Some(CommandRecord) if a command was completed
    pub fn process_input(&mut self, bytes: &[u8]) -> Option<CommandRecord> {
        // Convert bytes to string (lossy to handle invalid UTF-8)
        let input = String::from_utf8_lossy(bytes);
        
        // Track start time on first input
        if self.started_at.is_none() && !input.trim().is_empty() {
            self.started_at = Some(Instant::now());
        }
        
        // Check for command submission triggers
        for ch in input.chars() {
            match ch {
                '\n' | '\r' => {
                    // Newline = command submitted
                    return self.complete_command();
                }
                '\x03' => {
                    // Ctrl+C = cancel current command
                    self.reset();
                    return None;
                }
                '\x04' => {
                    // Ctrl+D = EOF or exit (don't track)
                    self.reset();
                    return None;
                }
                '\x7f' | '\x08' => {
                    // Backspace/Delete
                    self.buffer.pop();
                }
                _ if ch.is_control() => {
                    // Ignore other control characters
                }
                _ => {
                    // Regular character
                    self.buffer.push(ch);
                }
            }
        }
        
        None
    }
    
    /// Complete the current command and return record
    fn complete_command(&mut self) -> Option<CommandRecord> {
        let command = self.buffer.trim().to_string();
        
        // Ignore empty commands
        if command.is_empty() {
            self.reset();
            return None;
        }
        
        // Ignore comment-only commands
        if command.starts_with('#') {
            self.reset();
            return None;
        }
        
        let record = CommandRecord::new(
            command,
            CommandSource::User,
            self.cwd.clone(),
        );
        
        self.reset();
        Some(record)
    }
    
    /// Reset the buffer for next command
    fn reset(&mut self) {
        self.buffer.clear();
        self.started_at = None;
    }
    
    /// Get the current partial command (for UI display)
    pub fn current_command(&self) -> &str {
        &self.buffer
    }
    
    /// Check if currently capturing a command
    pub fn is_capturing(&self) -> bool {
        !self.buffer.is_empty()
    }
}

/// Detect bracketed paste mode (for multi-line paste detection)
pub fn detect_bracketed_paste(bytes: &[u8]) -> bool {
    // Bracketed paste: ESC[200~ ... ESC[201~
    const PASTE_START: &[u8] = b"\x1b[200~";
    const PASTE_END: &[u8] = b"\x1b[201~";
    
    bytes.windows(PASTE_START.len()).any(|w| w == PASTE_START)
        || bytes.windows(PASTE_END.len()).any(|w| w == PASTE_END)
}

/// Extract command from bracketed paste
pub fn extract_paste_command(input: &str) -> Option<String> {
    // Find content between paste markers
    if let Some(start_idx) = input.find("\x1b[200~") {
        if let Some(end_idx) = input.find("\x1b[201~") {
            let start = start_idx + "\x1b[200~".len();
            if start < end_idx {
                let pasted = &input[start..end_idx];
                return Some(pasted.trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_command() {
        let mut capture = CommandCapture::new(PathBuf::from("/tmp"));
        
        // Type "ls"
        assert!(capture.process_input(b"l").is_none());
        assert!(capture.process_input(b"s").is_none());
        assert_eq!(capture.current_command(), "ls");
        
        // Press Enter
        let record = capture.process_input(b"\n").unwrap();
        assert_eq!(record.command, "ls");
        assert_eq!(record.source, CommandSource::User);
        assert_eq!(record.cwd, PathBuf::from("/tmp"));
        
        // Buffer should be reset
        assert_eq!(capture.current_command(), "");
    }
    
    #[test]
    fn test_command_with_backspace() {
        let mut capture = CommandCapture::new(PathBuf::from("/tmp"));
        
        // Type "lss" then backspace
        capture.process_input(b"l");
        capture.process_input(b"s");
        capture.process_input(b"s");
        assert_eq!(capture.current_command(), "lss");
        
        capture.process_input(b"\x7f"); // Backspace
        assert_eq!(capture.current_command(), "ls");
        
        let record = capture.process_input(b"\n").unwrap();
        assert_eq!(record.command, "ls");
    }
    
    #[test]
    fn test_ctrl_c_cancellation() {
        let mut capture = CommandCapture::new(PathBuf::from("/tmp"));
        
        capture.process_input(b"ls");
        assert!(capture.is_capturing());
        
        // Ctrl+C should cancel
        assert!(capture.process_input(b"\x03").is_none());
        assert!(!capture.is_capturing());
        assert_eq!(capture.current_command(), "");
    }
    
    #[test]
    fn test_empty_command_ignored() {
        let mut capture = CommandCapture::new(PathBuf::from("/tmp"));
        
        // Just press Enter
        assert!(capture.process_input(b"\n").is_none());
        
        // Type spaces then Enter
        capture.process_input(b"   ");
        assert!(capture.process_input(b"\n").is_none());
    }
    
    #[test]
    fn test_comment_ignored() {
        let mut capture = CommandCapture::new(PathBuf::from("/tmp"));
        
        capture.process_input(b"# this is a comment");
        assert!(capture.process_input(b"\n").is_none());
    }
    
    #[test]
    fn test_multi_line_command() {
        let mut capture = CommandCapture::new(PathBuf::from("/tmp"));
        
        // Type "git commit -m "hello\nworld""
        capture.process_input(b"git commit -m \"hello");
        // No newline yet, so no command
        assert!(capture.is_capturing());
        
        // Complete the command
        let record = capture.process_input(b"\n").unwrap();
        assert_eq!(record.command, "git commit -m \"hello");
    }
    
    #[test]
    fn test_bracketed_paste_detection() {
        let paste_data = b"\x1b[200~ls -la\n\x1b[201~";
        assert!(detect_bracketed_paste(paste_data));
        
        let normal_data = b"ls -la\n";
        assert!(!detect_bracketed_paste(normal_data));
    }
    
    #[test]
    fn test_extract_paste_command() {
        let input = "\x1b[200~git status\n\x1b[201~";
        let cmd = extract_paste_command(input).unwrap();
        assert_eq!(cmd, "git status");
    }
    
    #[test]
    fn test_cwd_tracking() {
        let mut capture = CommandCapture::new(PathBuf::from("/tmp"));
        
        capture.process_input(b"ls");
        let record = capture.process_input(b"\n").unwrap();
        assert_eq!(record.cwd, PathBuf::from("/tmp"));
        
        // Change directory
        capture.set_cwd(PathBuf::from("/home/user"));
        capture.process_input(b"pwd");
        let record2 = capture.process_input(b"\n").unwrap();
        assert_eq!(record2.cwd, PathBuf::from("/home/user"));
    }
}
