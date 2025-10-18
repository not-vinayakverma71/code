// Terminal Pre-IPC: Shell integration marker parsing
// Part of HP3: Force Exit Timeout feature

use std::time::{Duration, Instant};

/// OSC 633/133 shell integration marker parser
/// 
/// Supports VS Code (OSC 633) and iTerm2 (OSC 133) shell integration protocols:
/// - OSC 633;A (Prompt Start)
/// - OSC 633;B (Prompt End)  
/// - OSC 633;C (Command Start)
/// - OSC 633;D;exit_code (Command End)
/// - OSC 133;A (Prompt Start)
/// - OSC 133;C (Command Start)
/// - OSC 133;D;exit_code (Command End)
#[derive(Debug, Clone)]
pub struct ShellIntegrationMonitor {
    /// Current command execution state
    state: ExecutionState,
    
    /// When the current command started
    command_start_time: Option<Instant>,
    
    /// Timeout duration for force-exit (default 3s)
    force_exit_timeout: Duration,
    
    /// Whether to enable debounce to avoid false positives
    debounce_enabled: bool,
    
    /// Debounce buffer time (default 100ms)
    debounce_duration: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionState {
    /// Waiting for command
    Idle,
    
    /// Command has started (saw C marker)
    CommandRunning,
    
    /// Command completed (saw D marker)
    CommandCompleted { exit_code: i32 },
    
    /// Force-completed due to timeout
    ForceCompleted,
}

/// Parsed shell integration marker
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellMarker {
    /// Prompt start (A marker)
    PromptStart,
    
    /// Prompt end (B marker)
    PromptEnd,
    
    /// Command execution start (C marker)
    CommandStart,
    
    /// Command execution end with exit code (D marker)
    CommandEnd { exit_code: i32 },
    
    /// Unknown/unsupported marker
    Unknown(String),
}

impl ShellIntegrationMonitor {
    /// Create a new monitor with default settings
    pub fn new() -> Self {
        Self {
            state: ExecutionState::Idle,
            command_start_time: None,
            force_exit_timeout: Duration::from_secs(3),
            debounce_enabled: true,
            debounce_duration: Duration::from_millis(100),
        }
    }
    
    /// Create with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        let mut monitor = Self::new();
        monitor.force_exit_timeout = timeout;
        monitor
    }
    
    /// Disable debounce (for testing)
    pub fn without_debounce(mut self) -> Self {
        self.debounce_enabled = false;
        self
    }
    
    /// Parse OSC sequence from terminal output
    /// Format: ESC ] 633 ; X [ ; args ] ESC \
    /// or: ESC ] 133 ; X [ ; args ] ESC \
    pub fn parse_marker(data: &str) -> Option<ShellMarker> {
        // OSC format: ESC ] Ps ; Pt BEL/ST
        // Where BEL = \x07, ST = ESC \
        
        // Check for OSC 633 or OSC 133
        if let Some(osc_data) = Self::extract_osc_sequence(data, "633") {
            return Self::parse_osc_633(&osc_data);
        }
        
        if let Some(osc_data) = Self::extract_osc_sequence(data, "133") {
            return Self::parse_osc_133(&osc_data);
        }
        
        None
    }
    
    /// Extract OSC sequence data
    fn extract_osc_sequence(data: &str, code: &str) -> Option<String> {
        // Look for: ESC ] code ; data
        let osc_start = format!("\x1b]{};", code);
        
        if let Some(start_idx) = data.find(&osc_start) {
            let data_start = start_idx + osc_start.len();
            let remaining = &data[data_start..];
            
            // Find terminator: BEL (\x07) or ST (ESC \)
            let end_idx = remaining
                .find('\x07')
                .or_else(|| remaining.find("\x1b\\"))
                .unwrap_or(remaining.len());
            
            return Some(remaining[..end_idx].to_string());
        }
        
        None
    }
    
    /// Parse OSC 633 (VS Code shell integration)
    fn parse_osc_633(data: &str) -> Option<ShellMarker> {
        let parts: Vec<&str> = data.split(';').collect();
        
        match parts.first()? {
            &"A" => Some(ShellMarker::PromptStart),
            &"B" => Some(ShellMarker::PromptEnd),
            &"C" => Some(ShellMarker::CommandStart),
            &"D" => {
                // D marker can have exit code: D;exit_code
                let exit_code = parts.get(1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                Some(ShellMarker::CommandEnd { exit_code })
            }
            other => Some(ShellMarker::Unknown(other.to_string())),
        }
    }
    
    /// Parse OSC 133 (iTerm2 shell integration)
    fn parse_osc_133(data: &str) -> Option<ShellMarker> {
        let parts: Vec<&str> = data.split(';').collect();
        
        match parts.first()? {
            &"A" => Some(ShellMarker::PromptStart),
            &"C" => Some(ShellMarker::CommandStart),
            &"D" => {
                let exit_code = parts.get(1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                Some(ShellMarker::CommandEnd { exit_code })
            }
            other => Some(ShellMarker::Unknown(other.to_string())),
        }
    }
    
    /// Process a marker and update state
    pub fn process_marker(&mut self, marker: ShellMarker) -> MarkerEvent {
        match marker {
            ShellMarker::CommandStart => {
                self.state = ExecutionState::CommandRunning;
                self.command_start_time = Some(Instant::now());
                MarkerEvent::CommandStarted
            }
            
            ShellMarker::CommandEnd { exit_code } => {
                if matches!(self.state, ExecutionState::CommandRunning) {
                    self.state = ExecutionState::CommandCompleted { exit_code };
                    let duration = self.command_start_time
                        .map(|start| start.elapsed())
                        .unwrap_or_default();
                    
                    self.command_start_time = None;
                    MarkerEvent::CommandCompleted { exit_code, duration, forced: false }
                } else {
                    MarkerEvent::None
                }
            }
            
            ShellMarker::PromptStart | ShellMarker::PromptEnd => {
                // Reset to idle on new prompt
                self.reset();
                MarkerEvent::None
            }
            
            ShellMarker::Unknown(_) => MarkerEvent::None,
        }
    }
    
    /// Check if command should be force-completed due to timeout
    /// Call this periodically (e.g., every 100ms) when command is running
    pub fn check_timeout(&mut self) -> Option<MarkerEvent> {
        if !matches!(self.state, ExecutionState::CommandRunning) {
            return None;
        }
        
        let start_time = self.command_start_time?;
        let elapsed = start_time.elapsed();
        
        // Apply debounce if enabled
        if self.debounce_enabled && elapsed < self.debounce_duration {
            return None;
        }
        
        if elapsed >= self.force_exit_timeout {
            self.state = ExecutionState::ForceCompleted;
            let duration = elapsed;
            self.command_start_time = None;
            
            return Some(MarkerEvent::CommandCompleted {
                exit_code: 0,
                duration,
                forced: true,
            });
        }
        
        None
    }
    
    /// Reset the monitor state
    pub fn reset(&mut self) {
        self.state = ExecutionState::Idle;
        self.command_start_time = None;
    }
    
    /// Check if command is currently running
    pub fn is_command_running(&self) -> bool {
        matches!(self.state, ExecutionState::CommandRunning)
    }
    
    /// Get time since command started
    pub fn time_since_start(&self) -> Option<Duration> {
        self.command_start_time.map(|start| start.elapsed())
    }
}

/// Event emitted by the monitor
#[derive(Debug, Clone, PartialEq)]
pub enum MarkerEvent {
    /// No event
    None,
    
    /// Command has started
    CommandStarted,
    
    /// Command has completed
    CommandCompleted {
        exit_code: i32,
        duration: Duration,
        forced: bool,
    },
}

impl Default for ShellIntegrationMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_osc_633_command_start() {
        let data = "\x1b]633;C\x07";
        let marker = ShellIntegrationMonitor::parse_marker(data).unwrap();
        assert_eq!(marker, ShellMarker::CommandStart);
    }
    
    #[test]
    fn test_parse_osc_633_command_end() {
        let data = "\x1b]633;D;0\x07";
        let marker = ShellIntegrationMonitor::parse_marker(data).unwrap();
        assert_eq!(marker, ShellMarker::CommandEnd { exit_code: 0 });
        
        let data = "\x1b]633;D;127\x07";
        let marker = ShellIntegrationMonitor::parse_marker(data).unwrap();
        assert_eq!(marker, ShellMarker::CommandEnd { exit_code: 127 });
    }
    
    #[test]
    fn test_parse_osc_633_with_st_terminator() {
        // ST terminator (ESC \) instead of BEL
        let data = "\x1b]633;C\x1b\\";
        let marker = ShellIntegrationMonitor::parse_marker(data).unwrap();
        assert_eq!(marker, ShellMarker::CommandStart);
    }
    
    #[test]
    fn test_parse_osc_133_markers() {
        let data = "\x1b]133;A\x07";
        let marker = ShellIntegrationMonitor::parse_marker(data).unwrap();
        assert_eq!(marker, ShellMarker::PromptStart);
        
        let data = "\x1b]133;C\x07";
        let marker = ShellIntegrationMonitor::parse_marker(data).unwrap();
        assert_eq!(marker, ShellMarker::CommandStart);
        
        let data = "\x1b]133;D;42\x07";
        let marker = ShellIntegrationMonitor::parse_marker(data).unwrap();
        assert_eq!(marker, ShellMarker::CommandEnd { exit_code: 42 });
    }
    
    #[test]
    fn test_monitor_command_lifecycle() {
        let mut monitor = ShellIntegrationMonitor::new();
        
        // Start command
        let event = monitor.process_marker(ShellMarker::CommandStart);
        assert_eq!(event, MarkerEvent::CommandStarted);
        assert!(monitor.is_command_running());
        
        // Complete command
        let event = monitor.process_marker(ShellMarker::CommandEnd { exit_code: 0 });
        match event {
            MarkerEvent::CommandCompleted { exit_code, forced, .. } => {
                assert_eq!(exit_code, 0);
                assert!(!forced);
            }
            _ => panic!("Expected CommandCompleted event"),
        }
        
        assert!(!monitor.is_command_running());
    }
    
    #[test]
    fn test_monitor_force_exit_timeout() {
        let mut monitor = ShellIntegrationMonitor::with_timeout(Duration::from_millis(100))
            .without_debounce();
        
        // Start command
        monitor.process_marker(ShellMarker::CommandStart);
        assert!(monitor.is_command_running());
        
        // Check timeout immediately - should not trigger
        assert!(monitor.check_timeout().is_none());
        
        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));
        
        // Check timeout - should trigger force completion
        let event = monitor.check_timeout().unwrap();
        match event {
            MarkerEvent::CommandCompleted { exit_code, forced, duration } => {
                assert_eq!(exit_code, 0);
                assert!(forced);
                assert!(duration >= Duration::from_millis(100));
            }
            _ => panic!("Expected forced CommandCompleted event"),
        }
        
        assert!(!monitor.is_command_running());
    }
    
    #[test]
    fn test_monitor_no_timeout_if_completed() {
        let mut monitor = ShellIntegrationMonitor::with_timeout(Duration::from_millis(100))
            .without_debounce();
        
        // Start and complete command quickly
        monitor.process_marker(ShellMarker::CommandStart);
        monitor.process_marker(ShellMarker::CommandEnd { exit_code: 0 });
        
        // Wait past timeout
        std::thread::sleep(Duration::from_millis(150));
        
        // Timeout check should return None since already completed
        assert!(monitor.check_timeout().is_none());
    }
    
    #[test]
    fn test_monitor_reset_on_prompt() {
        let mut monitor = ShellIntegrationMonitor::new();
        
        monitor.process_marker(ShellMarker::CommandStart);
        assert!(monitor.is_command_running());
        
        // Prompt start should reset state
        monitor.process_marker(ShellMarker::PromptStart);
        assert!(!monitor.is_command_running());
    }
    
    #[test]
    fn test_parse_marker_in_mixed_output() {
        // Marker embedded in normal output
        let data = "some output \x1b]633;C\x07 more output";
        let marker = ShellIntegrationMonitor::parse_marker(data).unwrap();
        assert_eq!(marker, ShellMarker::CommandStart);
    }
    
    #[test]
    fn test_debounce_prevents_immediate_timeout() {
        // Set timeout to 200ms with default debounce (100ms)
        let mut monitor = ShellIntegrationMonitor::with_timeout(Duration::from_millis(200));
        
        monitor.process_marker(ShellMarker::CommandStart);
        
        // Check immediately - debounce should prevent any check
        assert!(monitor.check_timeout().is_none());
        
        // Wait 50ms (still within debounce window)
        std::thread::sleep(Duration::from_millis(50));
        assert!(monitor.check_timeout().is_none());
        
        // Wait past debounce (110ms total)
        std::thread::sleep(Duration::from_millis(60));
        // Should still not timeout (110ms < 200ms timeout)
        assert!(monitor.check_timeout().is_none());
        
        // Wait past timeout (250ms total)
        std::thread::sleep(Duration::from_millis(140));
        // Now should force-complete
        assert!(monitor.check_timeout().is_some());
    }
}
