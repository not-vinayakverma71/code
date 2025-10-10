// Enhanced Error Model & Recovery System - Production-grade with normalized error codes
// Part of Error model TODO #7 - pre-IPC

use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};

// Normalized error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum ErrorCode {
    // 1xxx: File system errors
    FileNotFound = 1001,
    FileAccessDenied = 1002,
    FileAlreadyExists = 1003,
    FileTooLarge = 1004,
    InvalidPath = 1005,
    
    // 2xxx: Network errors
    NetworkTimeout = 2001,
    NetworkUnreachable = 2002,
    ConnectionRefused = 2003,
    
    // 3xxx: Tool errors
    ToolNotFound = 3001,
    ToolTimeout = 3002,
    ToolExecutionFailed = 3003,
    InvalidArguments = 3004,
    
    // 4xxx: Permission errors
    PermissionDenied = 4001,
    ApprovalRequired = 4002,
    RooIgnoreBlocked = 4003,
    
    // 5xxx: System errors
    OutOfMemory = 5001,
    DiskFull = 5002,
    SystemError = 5003,
    
    // 9xxx: Unknown
    Unknown = 9999,
}

impl ErrorCode {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::FileNotFound => ErrorSeverity::Warning,
            Self::FileAccessDenied | Self::PermissionDenied => ErrorSeverity::Error,
            Self::OutOfMemory | Self::DiskFull => ErrorSeverity::Critical,
            Self::NetworkTimeout => ErrorSeverity::Warning,
            _ => ErrorSeverity::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
}

// Normalized error shape
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: String,
    pub tool_name: String,
    pub severity: ErrorSeverity,
    pub escalation: Option<EscalationMetadata>,
    pub recovery_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationMetadata {
    pub consecutive_count: u32,
    pub total_count: u32,
    pub first_occurrence: DateTime<Utc>,
    pub escalation_level: u32,
    pub suggested_action: String,
}

// Per-tool error counter
#[derive(Debug, Clone, Default)]
pub struct ToolErrorCounter {
    consecutive_mistakes: u32,
    total_errors: u32,
    error_history: VecDeque<(ErrorCode, DateTime<Utc>)>,
    last_success: Option<DateTime<Utc>>,
}

impl ToolErrorCounter {
    pub fn record_error(&mut self, code: ErrorCode) {
        self.consecutive_mistakes += 1;
        self.total_errors += 1;
        self.error_history.push_back((code, Utc::now()));
        
        // Keep only last 100 errors
        while self.error_history.len() > 100 {
            self.error_history.pop_front();
        }
    }
    
    pub fn record_success(&mut self) {
        self.consecutive_mistakes = 0;
        self.last_success = Some(Utc::now());
    }
    
    pub fn should_escalate(&self) -> bool {
        self.consecutive_mistakes >= 3
    }
    
    pub fn get_escalation_level(&self) -> u32 {
        match self.consecutive_mistakes {
            0..=2 => 0,
            3..=5 => 1,
            6..=10 => 2,
            _ => 3,
        }
    }
}

// Enhanced error recovery manager
pub struct ErrorRecoveryV2 {
    // Normalized errors
    errors: Arc<RwLock<VecDeque<NormalizedError>>>,
    
    // Per-tool counters
    tool_counters: Arc<RwLock<HashMap<String, ToolErrorCounter>>>,
    
    // Global consecutive mistake count
    global_consecutive: Arc<RwLock<u32>>,
    
    // Error code mappings
    code_strategies: Arc<RwLock<HashMap<ErrorCode, RecoveryStrategy>>>,
    
    // Statistics
    stats: Arc<RwLock<ErrorStatistics>>,
    
    config: ErrorRecoveryConfig,
}

#[derive(Debug, Clone)]
pub struct ErrorRecoveryConfig {
    pub max_errors: usize,
    pub max_consecutive_before_abort: u32,
    pub escalation_thresholds: Vec<u32>,
    pub auto_recovery_enabled: bool,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            max_errors: 10000,
            max_consecutive_before_abort: 10,
            escalation_thresholds: vec![3, 5, 10],
            auto_recovery_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    Retry { max_attempts: u32, backoff_ms: u64 },
    Fallback { alternative: String },
    Skip,
    Abort,
    Manual,
    AutoFix { fix_type: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorStatistics {
    pub total_errors: u64,
    pub by_code: HashMap<ErrorCode, u64>,
    pub by_tool: HashMap<String, u64>,
    pub by_severity: HashMap<ErrorSeverity, u64>,
    pub recovery_attempts: u64,
    pub recovery_successes: u64,
    pub escalations: u64,
}

impl ErrorRecoveryV2 {
    pub fn new(config: ErrorRecoveryConfig) -> Self {
        Self {
            errors: Arc::new(RwLock::new(VecDeque::with_capacity(config.max_errors))),
            tool_counters: Arc::new(RwLock::new(HashMap::new())),
            global_consecutive: Arc::new(RwLock::new(0)),
            code_strategies: Arc::new(RwLock::new(Self::default_strategies())),
            stats: Arc::new(RwLock::new(ErrorStatistics::default())),
            config,
        }
    }
    
    fn default_strategies() -> HashMap<ErrorCode, RecoveryStrategy> {
        let mut strategies = HashMap::new();
        
        // Network errors - retry with exponential backoff
        strategies.insert(ErrorCode::NetworkTimeout, RecoveryStrategy::Retry {
            max_attempts: 3,
            backoff_ms: 1000,
        });
        strategies.insert(ErrorCode::ConnectionRefused, RecoveryStrategy::Retry {
            max_attempts: 2,
            backoff_ms: 2000,
        });
        
        // File errors
        strategies.insert(ErrorCode::FileNotFound, RecoveryStrategy::Skip);
        strategies.insert(ErrorCode::FileTooLarge, RecoveryStrategy::Manual);
        
        // Permission errors
        strategies.insert(ErrorCode::PermissionDenied, RecoveryStrategy::Manual);
        strategies.insert(ErrorCode::ApprovalRequired, RecoveryStrategy::Manual);
        
        // System errors
        strategies.insert(ErrorCode::OutOfMemory, RecoveryStrategy::Abort);
        strategies.insert(ErrorCode::DiskFull, RecoveryStrategy::Abort);
        
        strategies
    }
    
    pub fn track_error(&self, tool_name: &str, error: anyhow::Error) -> NormalizedError {
        // Normalize the error
        let code = Self::error_to_code(&error);
        let correlation_id = uuid::Uuid::new_v4().to_string();
        
        // Update per-tool counter
        let mut counters = self.tool_counters.write();
        let counter = counters.entry(tool_name.to_string()).or_default();
        counter.record_error(code);
        
        let escalation = if counter.should_escalate() {
            Some(EscalationMetadata {
                consecutive_count: counter.consecutive_mistakes,
                total_count: counter.total_errors,
                first_occurrence: counter.error_history.front()
                    .map(|(_, t)| *t)
                    .unwrap_or_else(Utc::now),
                escalation_level: counter.get_escalation_level(),
                suggested_action: Self::get_escalation_action(counter.get_escalation_level()),
            })
        } else {
            None
        };
        
        // Update global consecutive
        *self.global_consecutive.write() += 1;
        
        // Create normalized error
        let normalized = NormalizedError {
            code,
            message: error.to_string(),
            details: error.source().map(|s| s.to_string()),
            timestamp: Utc::now(),
            correlation_id: correlation_id.clone(),
            tool_name: tool_name.to_string(),
            severity: code.severity(),
            escalation,
            recovery_hint: self.get_recovery_hint(code),
        };
        
        // Store error
        let mut errors = self.errors.write();
        if errors.len() >= self.config.max_errors {
            errors.pop_front();
        }
        errors.push_back(normalized.clone());
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_errors += 1;
        *stats.by_code.entry(code).or_insert(0) += 1;
        *stats.by_tool.entry(tool_name.to_string()).or_insert(0) += 1;
        *stats.by_severity.entry(code.severity()).or_insert(0) += 1;
        if normalized.escalation.is_some() {
            stats.escalations += 1;
        }
        
        normalized
    }
    
    pub fn track_success(&self, tool_name: &str) {
        // Reset consecutive counters
        *self.global_consecutive.write() = 0;
        
        let mut counters = self.tool_counters.write();
        if let Some(counter) = counters.get_mut(tool_name) {
            counter.record_success();
        }
    }
    
    pub fn get_consecutive_mistakes(&self, tool_name: &str) -> u32 {
        self.tool_counters.read()
            .get(tool_name)
            .map(|c| c.consecutive_mistakes)
            .unwrap_or(0)
    }
    
    pub fn should_abort(&self) -> bool {
        *self.global_consecutive.read() >= self.config.max_consecutive_before_abort
    }
    
    pub fn get_recovery_strategy(&self, code: ErrorCode) -> RecoveryStrategy {
        self.code_strategies.read()
            .get(&code)
            .cloned()
            .unwrap_or(RecoveryStrategy::Skip)
    }
    
    fn error_to_code(error: &anyhow::Error) -> ErrorCode {
        let error_str = error.to_string().to_lowercase();
        
        if error_str.contains("not found") || error_str.contains("no such file") {
            ErrorCode::FileNotFound
        } else if error_str.contains("permission denied") || error_str.contains("access denied") {
            ErrorCode::PermissionDenied
        } else if error_str.contains("already exists") {
            ErrorCode::FileAlreadyExists
        } else if error_str.contains("timeout") {
            ErrorCode::NetworkTimeout
        } else if error_str.contains("connection refused") {
            ErrorCode::ConnectionRefused
        } else if error_str.contains("out of memory") {
            ErrorCode::OutOfMemory
        } else if error_str.contains("disk full") || error_str.contains("no space") {
            ErrorCode::DiskFull
        } else if error_str.contains("rooignore") {
            ErrorCode::RooIgnoreBlocked
        } else if error_str.contains("approval") {
            ErrorCode::ApprovalRequired
        } else {
            ErrorCode::Unknown
        }
    }
    
    fn get_escalation_action(level: u32) -> String {
        match level {
            0 => "Continue normally".to_string(),
            1 => "Review error pattern and adjust strategy".to_string(),
            2 => "Consider manual intervention".to_string(),
            3 => "Abort operation and seek assistance".to_string(),
            _ => "Critical: Immediate action required".to_string(),
        }
    }
    
    fn get_recovery_hint(&self, code: ErrorCode) -> Option<String> {
        match code {
            ErrorCode::FileNotFound => Some("Check file path and existence".to_string()),
            ErrorCode::PermissionDenied => Some("Check file permissions or run with appropriate privileges".to_string()),
            ErrorCode::NetworkTimeout => Some("Check network connection and retry".to_string()),
            ErrorCode::FileTooLarge => Some("Consider processing file in chunks".to_string()),
            ErrorCode::RooIgnoreBlocked => Some("Path is blocked by .rooignore rules".to_string()),
            _ => None,
        }
    }
    
    pub fn get_statistics(&self) -> ErrorStatistics {
        self.stats.read().clone()
    }
    
    pub fn clear_old_errors(&self, older_than: DateTime<Utc>) {
        self.errors.write().retain(|e| e.timestamp > older_than);
    }
}

// Global instance
lazy_static::lazy_static! {
    pub static ref ERROR_RECOVERY: Arc<ErrorRecoveryV2> = 
        Arc::new(ErrorRecoveryV2::new(ErrorRecoveryConfig::default()));
}

// Add to Cargo.toml:
// uuid = { version = "1.6", features = ["v4"] }

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_consecutive_counting() {
        let recovery = ErrorRecoveryV2::new(ErrorRecoveryConfig::default());
        
        // Track multiple errors for same tool
        for _ in 0..3 {
            recovery.track_error("test_tool", anyhow::anyhow!("File not found"));
        }
        
        assert_eq!(recovery.get_consecutive_mistakes("test_tool"), 3);
        
        // Success should reset counter
        recovery.track_success("test_tool");
        assert_eq!(recovery.get_consecutive_mistakes("test_tool"), 0);
    }
    
    #[test]
    fn test_error_normalization() {
        let recovery = ErrorRecoveryV2::new(ErrorRecoveryConfig::default());
        
        let error1 = recovery.track_error("tool1", anyhow::anyhow!("File not found: test.txt"));
        assert_eq!(error1.code, ErrorCode::FileNotFound);
        
        let error2 = recovery.track_error("tool2", anyhow::anyhow!("Permission denied"));
        assert_eq!(error2.code, ErrorCode::PermissionDenied);
        
        let error3 = recovery.track_error("tool3", anyhow::anyhow!("Connection timeout"));
        assert_eq!(error3.code, ErrorCode::NetworkTimeout);
    }
    
    #[test]
    fn test_escalation() {
        let recovery = ErrorRecoveryV2::new(ErrorRecoveryConfig::default());
        
        // Track enough errors to trigger escalation
        for i in 0..5 {
            let error = recovery.track_error("escalate_tool", anyhow::anyhow!("Error {}", i));
            if i >= 2 {
                assert!(error.escalation.is_some());
                assert_eq!(error.escalation.as_ref().unwrap().consecutive_count, (i + 1) as u32);
            }
        }
        
        let stats = recovery.get_statistics();
        assert!(stats.escalations > 0);
    }
}
