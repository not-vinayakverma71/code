// Terminal Pre-IPC: Observability and metrics
// Part of HP-OBS: Structured logging and metrics collection

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

use super::types::CommandSource;

/// Terminal command event for structured logging
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandEvent {
    /// Event type
    pub event_type: CommandEventType,
    
    /// Terminal ID
    pub terminal_id: String,
    
    /// Command source (User or Cascade)
    pub source: CommandSource,
    
    /// Command text (sanitized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    
    /// Exit code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    
    /// Duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    
    /// Whether command was force-completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forced_exit: Option<bool>,
    
    /// Timestamp (Unix epoch seconds)
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandEventType {
    /// Command started executing
    CommandStart,
    
    /// Command completed (success or failure)
    CommandEnd,
    
    /// Command was force-completed due to timeout
    ForceExit,
    
    /// Command was injected by AI
    InjectionSuccess,
    
    /// Command injection failed validation
    InjectionFailed,
}

impl CommandEvent {
    /// Create a command start event
    pub fn start(terminal_id: String, source: CommandSource, command: String) -> Self {
        Self {
            event_type: CommandEventType::CommandStart,
            terminal_id,
            source,
            command: Some(Self::sanitize_command(&command)),
            exit_code: None,
            duration_ms: None,
            forced_exit: None,
            timestamp: Self::current_timestamp(),
        }
    }
    
    /// Create a command end event
    pub fn end(
        terminal_id: String,
        source: CommandSource,
        command: String,
        exit_code: i32,
        duration: Duration,
        forced: bool,
    ) -> Self {
        let event_type = if forced {
            CommandEventType::ForceExit
        } else {
            CommandEventType::CommandEnd
        };
        
        Self {
            event_type,
            terminal_id,
            source,
            command: Some(Self::sanitize_command(&command)),
            exit_code: Some(exit_code),
            duration_ms: Some(duration.as_millis() as u64),
            forced_exit: Some(forced),
            timestamp: Self::current_timestamp(),
        }
    }
    
    /// Create an injection success event
    pub fn injection_success(terminal_id: String, command: String) -> Self {
        Self {
            event_type: CommandEventType::InjectionSuccess,
            terminal_id,
            source: CommandSource::Cascade,
            command: Some(Self::sanitize_command(&command)),
            exit_code: None,
            duration_ms: None,
            forced_exit: None,
            timestamp: Self::current_timestamp(),
        }
    }
    
    /// Create an injection failed event
    pub fn injection_failed(terminal_id: String, command: String) -> Self {
        Self {
            event_type: CommandEventType::InjectionFailed,
            terminal_id,
            source: CommandSource::Cascade,
            command: Some(Self::sanitize_command(&command)),
            exit_code: None,
            duration_ms: None,
            forced_exit: None,
            timestamp: Self::current_timestamp(),
        }
    }
    
    /// Sanitize command for logging (remove sensitive data)
    fn sanitize_command(cmd: &str) -> String {
        // Truncate long commands
        const MAX_CMD_LEN: usize = 200;
        
        let sanitized = if cmd.len() > MAX_CMD_LEN {
            format!("{}...", &cmd[..MAX_CMD_LEN])
        } else {
            cmd.to_string()
        };
        
        // TODO: Could add more sanitization (remove passwords, tokens, etc.)
        sanitized
    }
    
    /// Get current timestamp
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
    
    /// Log this event as structured JSON
    pub fn log(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            match self.event_type {
                CommandEventType::CommandStart => {
                    tracing::info!(target: "terminal::command", "{}", json);
                }
                CommandEventType::CommandEnd => {
                    tracing::info!(target: "terminal::command", "{}", json);
                }
                CommandEventType::ForceExit => {
                    tracing::warn!(target: "terminal::command", "{}", json);
                }
                CommandEventType::InjectionSuccess => {
                    tracing::info!(target: "terminal::injection", "{}", json);
                }
                CommandEventType::InjectionFailed => {
                    tracing::warn!(target: "terminal::injection", "{}", json);
                }
            }
        }
    }
}

/// Metrics collector for terminal commands
#[derive(Debug, Clone)]
pub struct TerminalMetrics {
    /// Total commands executed
    pub total_commands: u64,
    
    /// Commands by source
    pub user_commands: u64,
    pub cascade_commands: u64,
    
    /// Forced exits count
    pub forced_exits: u64,
    
    /// Average command duration (milliseconds)
    pub avg_duration_ms: f64,
    
    /// Total duration sum (for calculating average)
    total_duration_ms: u64,
    
    /// Commands per minute (rolling window)
    pub commands_per_minute: f64,
    
    /// Timestamp of metrics start
    start_time: Instant,
}

impl TerminalMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            total_commands: 0,
            user_commands: 0,
            cascade_commands: 0,
            forced_exits: 0,
            avg_duration_ms: 0.0,
            total_duration_ms: 0,
            commands_per_minute: 0.0,
            start_time: Instant::now(),
        }
    }
    
    /// Record a command completion
    pub fn record_command(
        &mut self,
        source: CommandSource,
        duration: Duration,
        forced: bool,
    ) {
        self.total_commands += 1;
        
        match source {
            CommandSource::User => self.user_commands += 1,
            CommandSource::Cascade => self.cascade_commands += 1,
        }
        
        if forced {
            self.forced_exits += 1;
        }
        
        // Update duration statistics
        let duration_ms = duration.as_millis() as u64;
        self.total_duration_ms += duration_ms;
        self.avg_duration_ms = self.total_duration_ms as f64 / self.total_commands as f64;
        
        // Update commands per minute
        let elapsed_mins = self.start_time.elapsed().as_secs_f64() / 60.0;
        if elapsed_mins > 0.0 {
            self.commands_per_minute = self.total_commands as f64 / elapsed_mins;
        }
    }
    
    /// Reset all metrics
    pub fn reset(&mut self) {
        *self = Self::new();
    }
    
    /// Get snapshot of current metrics
    pub fn snapshot(&self) -> TerminalMetricsSnapshot {
        TerminalMetricsSnapshot {
            total_commands: self.total_commands,
            user_commands: self.user_commands,
            cascade_commands: self.cascade_commands,
            forced_exits: self.forced_exits,
            avg_duration_ms: self.avg_duration_ms,
            commands_per_minute: self.commands_per_minute,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }
}

impl Default for TerminalMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time (serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalMetricsSnapshot {
    pub total_commands: u64,
    pub user_commands: u64,
    pub cascade_commands: u64,
    pub forced_exits: u64,
    pub avg_duration_ms: f64,
    pub commands_per_minute: f64,
    pub uptime_seconds: u64,
}

/// Global metrics aggregator
pub struct MetricsAggregator {
    metrics: Arc<RwLock<TerminalMetrics>>,
}

impl MetricsAggregator {
    /// Create new aggregator
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(TerminalMetrics::new())),
        }
    }
    
    /// Record a command event
    pub fn record_command(
        &self,
        source: CommandSource,
        duration: Duration,
        forced: bool,
    ) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.record_command(source, duration, forced);
        }
    }
    
    /// Get current metrics snapshot
    pub fn snapshot(&self) -> Option<TerminalMetricsSnapshot> {
        self.metrics.read().ok().map(|m| m.snapshot())
    }
    
    /// Reset metrics
    pub fn reset(&self) {
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.reset();
        }
    }
}

impl Default for MetricsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MetricsAggregator {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_event_start() {
        let event = CommandEvent::start(
            "term-1".to_string(),
            CommandSource::User,
            "ls -la".to_string(),
        );
        
        assert_eq!(event.event_type, CommandEventType::CommandStart);
        assert_eq!(event.source, CommandSource::User);
        assert_eq!(event.command, Some("ls -la".to_string()));
        assert!(event.exit_code.is_none());
    }
    
    #[test]
    fn test_command_event_end() {
        let event = CommandEvent::end(
            "term-1".to_string(),
            CommandSource::Cascade,
            "echo hello".to_string(),
            0,
            Duration::from_millis(150),
            false,
        );
        
        assert_eq!(event.event_type, CommandEventType::CommandEnd);
        assert_eq!(event.source, CommandSource::Cascade);
        assert_eq!(event.exit_code, Some(0));
        assert_eq!(event.duration_ms, Some(150));
        assert_eq!(event.forced_exit, Some(false));
    }
    
    #[test]
    fn test_command_event_force_exit() {
        let event = CommandEvent::end(
            "term-1".to_string(),
            CommandSource::User,
            "sleep 10".to_string(),
            0,
            Duration::from_secs(3),
            true,
        );
        
        assert_eq!(event.event_type, CommandEventType::ForceExit);
        assert_eq!(event.forced_exit, Some(true));
        assert_eq!(event.duration_ms, Some(3000));
    }
    
    #[test]
    fn test_command_sanitization() {
        let long_cmd = "a".repeat(300);
        let event = CommandEvent::start(
            "term-1".to_string(),
            CommandSource::User,
            long_cmd,
        );
        
        let sanitized = event.command.unwrap();
        assert!(sanitized.len() <= 203); // 200 chars + "..."
        assert!(sanitized.ends_with("..."));
    }
    
    #[test]
    fn test_command_event_serialization() {
        let event = CommandEvent::start(
            "term-1".to_string(),
            CommandSource::User,
            "ls".to_string(),
        );
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"eventType\":\"command_start\""));
        assert!(json.contains("\"source\":\"User\""));
    }
    
    #[test]
    fn test_metrics_recording() {
        let mut metrics = TerminalMetrics::new();
        
        // Record user command
        metrics.record_command(
            CommandSource::User,
            Duration::from_millis(100),
            false,
        );
        
        assert_eq!(metrics.total_commands, 1);
        assert_eq!(metrics.user_commands, 1);
        assert_eq!(metrics.cascade_commands, 0);
        assert_eq!(metrics.forced_exits, 0);
        assert_eq!(metrics.avg_duration_ms, 100.0);
        
        // Record cascade command with force exit
        metrics.record_command(
            CommandSource::Cascade,
            Duration::from_millis(200),
            true,
        );
        
        assert_eq!(metrics.total_commands, 2);
        assert_eq!(metrics.user_commands, 1);
        assert_eq!(metrics.cascade_commands, 1);
        assert_eq!(metrics.forced_exits, 1);
        assert_eq!(metrics.avg_duration_ms, 150.0);
    }
    
    #[test]
    fn test_metrics_snapshot() {
        let mut metrics = TerminalMetrics::new();
        metrics.record_command(CommandSource::User, Duration::from_millis(50), false);
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_commands, 1);
        assert_eq!(snapshot.user_commands, 1);
        assert_eq!(snapshot.avg_duration_ms, 50.0);
    }
    
    #[test]
    fn test_metrics_aggregator() {
        let aggregator = MetricsAggregator::new();
        
        aggregator.record_command(CommandSource::User, Duration::from_millis(100), false);
        aggregator.record_command(CommandSource::Cascade, Duration::from_millis(200), true);
        
        let snapshot = aggregator.snapshot().unwrap();
        assert_eq!(snapshot.total_commands, 2);
        assert_eq!(snapshot.forced_exits, 1);
        assert_eq!(snapshot.avg_duration_ms, 150.0);
    }
    
    #[test]
    fn test_metrics_reset() {
        let mut metrics = TerminalMetrics::new();
        metrics.record_command(CommandSource::User, Duration::from_millis(100), false);
        
        assert_eq!(metrics.total_commands, 1);
        
        metrics.reset();
        
        assert_eq!(metrics.total_commands, 0);
        assert_eq!(metrics.avg_duration_ms, 0.0);
    }
    
    #[test]
    fn test_commands_per_minute_calculation() {
        let mut metrics = TerminalMetrics::new();
        
        // Simulate some elapsed time by recording multiple commands
        for _ in 0..10 {
            metrics.record_command(CommandSource::User, Duration::from_millis(10), false);
        }
        
        // Commands per minute should be calculated
        assert!(metrics.commands_per_minute > 0.0);
    }
}
