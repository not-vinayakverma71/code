// HOUR 1: Error Context System - 1:1 Translation from TypeScript
// Based on context collection patterns from codex-reference

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use super::errors::{LapceError, ErrorSeverity};

/// Error context for tracking additional information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Timestamp when error occurred
    pub timestamp: SystemTime,
    
    /// Request ID for tracing
    pub request_id: Option<String>,
    
    /// User ID if applicable
    pub user_id: Option<String>,
    
    /// Operation that was being performed
    pub operation: Option<String>,
    
    /// Component where error occurred
    pub component: Option<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Environment information
    pub environment: EnvironmentInfo,
    
    /// Stack trace if available
    pub stack_trace: Option<String>,
    
    /// Related errors (for error chains)
    pub related_errors: Vec<String>,
}

/// Environment information for error context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// Operating system
    pub os: String,
    
    /// Architecture
    pub arch: String,
    
    /// Rust version
    pub rust_version: String,
    
    /// Application version
    pub app_version: String,
    
    /// Memory usage
    pub memory_usage: Option<MemoryInfo>,
    
    /// CPU usage percentage
    pub cpu_usage: Option<f64>,
}

/// Memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total memory in bytes
    pub total: u64,
    
    /// Used memory in bytes
    pub used: u64,
    
    /// Free memory in bytes
    pub free: u64,
    
    /// Memory usage percentage
    pub percentage: f64,
}

impl ErrorContext {
    /// Create new error context
    pub fn new() -> Self {
        Self {
            timestamp: SystemTime::now(),
            request_id: None,
            user_id: None,
            operation: None,
            component: None,
            metadata: HashMap::new(),
            environment: EnvironmentInfo::current(),
            stack_trace: None,
            related_errors: Vec::new(),
        }
    }
    
    /// Create context with request ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
    
    /// Add operation context
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = Some(operation);
        self
    }
    
    /// Add component context
    pub fn with_component(mut self, component: String) -> Self {
        self.component = Some(component);
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Add stack trace
    pub fn with_stack_trace(mut self, trace: String) -> Self {
        self.stack_trace = Some(trace);
        self
    }
    
    /// Add related error
    pub fn add_related_error(&mut self, error: String) {
        self.related_errors.push(error);
    }
    
    /// Create context from error
    pub fn from_error(error: &LapceError) -> Self {
        let mut context = Self::new();
        
        // Extract component from error type
        context.component = Some(match error {
            LapceError::Ipc { .. } => "ipc".to_string(),
            LapceError::Provider { provider, .. } => format!("provider:{}", provider),
            LapceError::Parse { file, .. } => format!("parser:{}", file.display()),
            LapceError::ResourceExhausted { resource, .. } => format!("resource:{}", resource),
            LapceError::Timeout { operation, .. } => format!("timeout:{}", operation),
            LapceError::ContextWindowExceeded { provider, .. } => format!("context:{}", provider),
            LapceError::FileRestriction { .. } => "file_restriction".to_string(),
            LapceError::CircuitOpen { component } => format!("circuit:{}", component),
            LapceError::ComponentNotFound(name) => format!("component:{}", name),
            LapceError::InvalidRequest { .. } => "validation".to_string(),
            LapceError::RateLimit { provider, .. } => {
                provider.as_ref().map(|p| format!("rate_limit:{}", p))
                    .unwrap_or_else(|| "rate_limit".to_string())
            }
            LapceError::AuthenticationFailed { provider, .. } => {
                provider.as_ref().map(|p| format!("auth:{}", p))
                    .unwrap_or_else(|| "auth".to_string())
            }
            _ => "unknown".to_string(),
        });
        
        // Add error message as metadata
        context.metadata.insert(
            "error_message".to_string(),
            serde_json::Value::String(error.to_string()),
        );
        
        // Add severity
        context.metadata.insert(
            "severity".to_string(),
            serde_json::Value::String(format!("{:?}", error.severity())),
        );
        
        // Add backtrace if available
        // Note: backtrace field was removed from LapceError
        context.stack_trace = None;
        
        context
    }
}

impl EnvironmentInfo {
    /// Get current environment information
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            rust_version: option_env!("RUSTC_VERSION").unwrap_or("unknown").to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            memory_usage: Self::get_memory_info(),
            cpu_usage: Self::get_cpu_usage(),
        }
    }
    
    /// Get memory information
    fn get_memory_info() -> Option<MemoryInfo> {
        // This would use system-specific APIs in production
        // For now, return placeholder
        None
    }
    
    /// Get CPU usage
    fn get_cpu_usage() -> Option<f64> {
        // This would use system-specific APIs in production
        // For now, return placeholder
        None
    }
}

/// Error report for telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReport {
    /// Error message
    pub error: String,
    
    /// Backtrace if available
    pub backtrace: Option<String>,
    
    /// Timestamp
    pub timestamp: SystemTime,
    
    /// Context information
    pub context: ErrorContext,
    
    /// Error severity
    pub severity: ErrorSeverity,
    
    /// Error fingerprint for deduplication
    pub fingerprint: String,
}

impl ErrorReport {
    /// Create report from error
    pub fn from_error(error: &LapceError) -> Self {
        let context = ErrorContext::from_error(error);
        let fingerprint = Self::generate_fingerprint(error);
        
        Self {
            error: error.to_string(),
            backtrace: None, // backtrace field was removed from LapceError
            timestamp: SystemTime::now(),
            context,
            severity: error.severity(),
            fingerprint,
        }
    }
    
    /// Generate fingerprint for error deduplication
    fn generate_fingerprint(error: &LapceError) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash error type and key fields
        match error {
            LapceError::Ipc { message, .. } => {
                "ipc".hash(&mut hasher);
                message.hash(&mut hasher);
            }
            LapceError::Provider { provider, message, .. } => {
                "provider".hash(&mut hasher);
                provider.hash(&mut hasher);
                message.hash(&mut hasher);
            }
            LapceError::Parse { file, line, column, .. } => {
                "parse".hash(&mut hasher);
                file.hash(&mut hasher);
                line.hash(&mut hasher);
                column.hash(&mut hasher);
            }
            LapceError::ResourceExhausted { resource, .. } => {
                "resource".hash(&mut hasher);
                format!("{:?}", resource).hash(&mut hasher);
            }
            LapceError::Timeout { operation, .. } => {
                "timeout".hash(&mut hasher);
                operation.hash(&mut hasher);
            }
            LapceError::ContextWindowExceeded { provider, .. } => {
                "context_window".hash(&mut hasher);
                provider.hash(&mut hasher);
            }
            LapceError::FileRestriction { pattern, .. } => {
                "file_restriction".hash(&mut hasher);
                pattern.hash(&mut hasher);
            }
            LapceError::CircuitOpen { component } => {
                "circuit".hash(&mut hasher);
                component.hash(&mut hasher);
            }
            LapceError::ComponentNotFound(name) => {
                "component_not_found".hash(&mut hasher);
                name.hash(&mut hasher);
            }
            LapceError::InvalidRequest { error_type, .. } => {
                "invalid_request".hash(&mut hasher);
                error_type.hash(&mut hasher);
            }
            LapceError::RateLimit { provider, .. } => {
                "rate_limit".hash(&mut hasher);
                provider.hash(&mut hasher);
            }
            LapceError::AuthenticationFailed { provider, .. } => {
                "auth".hash(&mut hasher);
                provider.hash(&mut hasher);
            }
            _ => {
                "generic".hash(&mut hasher);
                error.to_string().hash(&mut hasher);
            }
        }
        
        format!("{:x}", hasher.finish())
    }
}

/// Context builder for fluent API
pub struct ContextBuilder {
    context: ErrorContext,
}

impl ContextBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            context: ErrorContext::new(),
        }
    }
    
    /// Set request ID
    pub fn request_id(mut self, id: String) -> Self {
        self.context.request_id = Some(id);
        self
    }
    
    /// Set operation
    pub fn operation(mut self, op: String) -> Self {
        self.context.operation = Some(op);
        self
    }
    
    /// Set component
    pub fn component(mut self, comp: String) -> Self {
        self.context.component = Some(comp);
        self
    }
    
    /// Add metadata
    pub fn metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.context.metadata.insert(key, value);
        self
    }
    
    /// Build the context
    pub fn build(self) -> ErrorContext {
        self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let context = ContextBuilder::new()
            .request_id("req-123".to_string())
            .operation("api_call".to_string())
            .component("provider".to_string())
            .metadata("user".to_string(), serde_json::json!("user-456"))
            .build();
        
        assert_eq!(context.request_id, Some("req-123".to_string()));
        assert_eq!(context.operation, Some("api_call".to_string()));
        assert_eq!(context.component, Some("provider".to_string()));
        assert!(context.metadata.contains_key("user"));
    }

    #[test]
    fn test_error_report_fingerprint() {
        let error1 = LapceError::Timeout {
            operation: "test".to_string(),
            duration: std::time::Duration::from_secs(5),
        };
        let error2 = LapceError::Timeout {
            operation: "test".to_string(),
            duration: std::time::Duration::from_secs(10),
        };
        let error3 = LapceError::Timeout {
            operation: "different".to_string(),
            duration: std::time::Duration::from_secs(5),
        };
        
        let report1 = ErrorReport::from_error(&error1);
        let report2 = ErrorReport::from_error(&error2);
        let report3 = ErrorReport::from_error(&error3);
        
        // Same operation should have same fingerprint regardless of duration
        assert_eq!(report1.fingerprint, report2.fingerprint);
        // Different operation should have different fingerprint
        assert_ne!(report1.fingerprint, report3.fingerprint);
    }
}
