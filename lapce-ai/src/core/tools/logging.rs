// Structured logging for tool execution - P0-LOG

use tracing::{info, warn, error, debug, instrument, Span};
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Structured log event for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionLog {
    /// Unique execution ID
    pub execution_id: String,
    
    /// Session ID for correlation
    pub session_id: String,
    
    /// Tool name
    pub tool_name: String,
    
    /// Operation type
    pub operation: String,
    
    /// User ID (redacted in logs)
    pub user_id: String,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    
    /// Success status
    pub success: bool,
    
    /// Error message if failed
    pub error: Option<String>,
    
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Audit log for approval events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalAuditLog {
    /// Unique approval ID
    pub approval_id: String,
    
    /// Tool requesting approval
    pub tool_name: String,
    
    /// Operation requiring approval
    pub operation: String,
    
    /// Target of operation (file path, command, etc.)
    pub target: String,
    
    /// User who approved/denied
    pub user_id: String,
    
    /// Approval decision
    pub approved: bool,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Reason for denial (if applicable)
    pub denial_reason: Option<String>,
}

/// File operation audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationLog {
    /// Operation type (read, write, delete, etc.)
    pub operation: String,
    
    /// File path
    pub path: String,
    
    /// User ID
    pub user_id: String,
    
    /// Success status
    pub success: bool,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// File size in bytes
    pub size_bytes: Option<u64>,
    
    /// Error if failed
    pub error: Option<String>,
}

/// Initialize structured logging with JSON formatter
pub fn init_logging() {
    let fmt_layer = fmt::layer()
        .with_target(false);
    
    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

/// Create a span for tool execution with correlation IDs
#[instrument(skip_all, fields(
    execution_id = %execution_id,
    session_id = %session_id,
    tool_name = %tool_name
))]
pub fn tool_execution_span(
    execution_id: &str,
    session_id: &str,
    tool_name: &str,
) -> Span {
    tracing::info_span!(
        "tool_execution",
        execution_id = %execution_id,
        session_id = %session_id,
        tool_name = %tool_name
    )
}

/// Log tool execution start
pub fn log_tool_start(tool_name: &str, execution_id: &str, session_id: &str) {
    info!(
        tool_name = %tool_name,
        execution_id = %execution_id,
        session_id = %session_id,
        "Tool execution started"
    );
}

/// Log tool execution completion
pub fn log_tool_complete(
    tool_name: &str,
    execution_id: &str,
    duration: Duration,
    success: bool,
    error: Option<&str>,
) {
    let log = ToolExecutionLog {
        execution_id: execution_id.to_string(),
        session_id: String::new(), // Would be retrieved from context
        tool_name: tool_name.to_string(),
        operation: "execute".to_string(),
        user_id: redact_pii("user_123"),
        timestamp: chrono::Utc::now(),
        duration_ms: Some(duration.as_millis() as u64),
        success,
        error: error.map(String::from),
        metadata: serde_json::json!({}),
    };
    
    if success {
        info!(
            tool_name = %tool_name,
            execution_id = %execution_id,
            duration_ms = duration.as_millis(),
            "Tool execution completed successfully"
        );
    } else {
        error!(
            tool_name = %tool_name,
            execution_id = %execution_id,
            duration_ms = duration.as_millis(),
            error = ?error,
            "Tool execution failed"
        );
    }
    
    // Emit structured log
    debug!("tool_execution_log: {}", serde_json::to_string(&log).unwrap());
}

/// Log approval request
pub fn log_approval_request(
    tool_name: &str,
    operation: &str,
    target: &str,
    user_id: &str,
) -> String {
    let approval_id = Uuid::new_v4().to_string();
    
    info!(
        approval_id = %approval_id,
        tool_name = %tool_name,
        operation = %operation,
        target = %redact_sensitive_path(target),
        user_id = %redact_pii(user_id),
        "Approval requested"
    );
    
    approval_id
}

/// Log approval decision
pub fn log_approval_decision(
    approval_id: &str,
    approved: bool,
    denial_reason: Option<&str>,
) {
    let log = ApprovalAuditLog {
        approval_id: approval_id.to_string(),
        tool_name: String::new(), // Would be retrieved from context
        operation: String::new(),
        target: String::new(),
        user_id: redact_pii("user_123"),
        approved,
        timestamp: chrono::Utc::now(),
        denial_reason: denial_reason.map(String::from),
    };
    
    if approved {
        info!(
            approval_id = %approval_id,
            "Approval granted"
        );
    } else {
        warn!(
            approval_id = %approval_id,
            denial_reason = ?denial_reason,
            "Approval denied"
        );
    }
    
    // Emit audit log
    info!("approval_audit: {}", serde_json::to_string(&log).unwrap());
}

/// Log file operation
pub fn log_file_operation(
    operation: &str,
    path: &str,
    success: bool,
    error: Option<&str>,
) {
    let log = FileOperationLog {
        operation: operation.to_string(),
        path: redact_sensitive_path(path),
        user_id: redact_pii("user_123"),
        success,
        timestamp: chrono::Utc::now(),
        size_bytes: None,
        error: error.map(String::from),
    };
    
    if success {
        info!(
            operation = %operation,
            path = %redact_sensitive_path(path),
            "File operation completed"
        );
    } else {
        error!(
            operation = %operation,
            path = %redact_sensitive_path(path),
            error = ?error,
            "File operation failed"
        );
    }
    
    // Emit audit log for write operations
    if operation == "write" || operation == "delete" || operation == "modify" {
        info!("file_audit: {}", serde_json::to_string(&log).unwrap());
    }
}

/// Redact PII from logs
fn redact_pii(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    
    // Redact email addresses
    if value.contains('@') {
        return "***@***.***".to_string();
    }
    
    // Redact user IDs (keep first 3 chars)
    if value.len() > 3 {
        format!("{}***", &value[..3])
    } else {
        "***".to_string()
    }
}

/// Redact sensitive paths
fn redact_sensitive_path(path: &str) -> String {
    // Redact home directory paths
    if path.contains("/home/") || path.contains("/Users/") {
        path.replace(|c: char| c.is_alphanumeric(), "*")
    } else if path.contains(".env") || path.contains("secret") || path.contains("key") {
        "***REDACTED***".to_string()
    } else {
        path.to_string()
    }
}

/// Structured logging context
#[derive(Clone)]
pub struct LogContext {
    pub execution_id: String,
    pub session_id: String,
    pub user_id: String,
    start_time: Instant,
}

impl LogContext {
    pub fn new(session_id: String, user_id: String) -> Self {
        Self {
            execution_id: Uuid::new_v4().to_string(),
            session_id,
            user_id,
            start_time: Instant::now(),
        }
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

#[cfg(test)]
#[path = "logging_tests.rs"]
mod logging_tests;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_redact_pii() {
        assert_eq!(redact_pii("user@example.com"), "***@***.***");
        assert_eq!(redact_pii("user123"), "use***");
        assert_eq!(redact_pii("ab"), "***");
    }
    
    #[test]
    fn test_redact_sensitive_path() {
        assert!(redact_sensitive_path("/home/user/file.txt").contains("*"));
        assert_eq!(redact_sensitive_path(".env"), "***REDACTED***");
        assert_eq!(redact_sensitive_path("secret.key"), "***REDACTED***");
        assert_eq!(redact_sensitive_path("/tmp/file.txt"), "/tmp/file.txt");
    }
    
    #[test]
    fn test_log_context() {
        let ctx = LogContext::new("session123".to_string(), "user456".to_string());
        assert!(!ctx.execution_id.is_empty());
        assert_eq!(ctx.session_id, "session123");
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(ctx.elapsed().as_millis() >= 10);
    }
}
