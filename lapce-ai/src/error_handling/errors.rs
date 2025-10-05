// HOUR 1: Core Error Types - Exact 1:1 Translation from TypeScript
// Based on codex-reference error patterns

use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// Main error type for Lapce - CHARACTER-FOR-CHARACTER error messages from TypeScript
#[derive(Debug)]
pub enum LapceError {
    /// IPC errors - matches TypeScript IPC error handling
    Ipc {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Provider errors - matches provider error patterns from TypeScript
    Provider {
        provider: String,
        message: String,
        retry_after: Option<Duration>,
    },
    
    /// Parse errors - matches TypeScript parse error structure
    Parse {
        file: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },
    
    /// Resource exhausted - matches TypeScript resource limit handling
    ResourceExhausted {
        resource: ResourceType,
        limit: usize,
        current: usize,
    },
    
    /// Operation timeout - matches TypeScript timeout patterns
    Timeout {
        operation: String,
        duration: Duration,
    },

    /// Context window exceeded - matches checkContextWindowExceededError from TypeScript
    ContextWindowExceeded {
        provider: String,
        message: String,
        max_tokens: Option<usize>,
        used_tokens: Option<usize>,
    },

    /// File restriction error - matches FileRestrictionError from modes.ts
    FileRestriction {
        tool_info: String,
        pattern: String,
        description: String,
        file_path: String,
    },

    /// Circuit breaker open - for circuit breaker pattern
    CircuitOpen {
        component: String,
    },

    /// Component not found
    ComponentNotFound(String),

    /// Invalid request - matches Anthropic/OpenAI error patterns
    InvalidRequest {
        message: String,
        error_type: String,
        status_code: u16,
    },

    /// Rate limit error - matches provider rate limiting
    RateLimit {
        message: String,
        retry_after: Option<Duration>,
        provider: Option<String>,
    },

    /// Authentication error
    AuthenticationFailed {
        message: String,
        provider: Option<String>,
    },

    /// Serialization/Deserialization error
    Serialization(String),

    /// IO error wrapper
    Io(std::io::Error),

    /// Generic error with context
    Generic {
        context: String,
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl fmt::Display for LapceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LapceError::Ipc { message, .. } => write!(f, "IPC error: {}", message),
            LapceError::Provider { provider, message, .. } => write!(f, "Provider error: {} - {}", provider, message),
            LapceError::Parse { file, line, column, message } => write!(f, "Parse error at {:?}:{}:{}: {}", file, line, column, message),
            LapceError::ResourceExhausted { resource, .. } => write!(f, "Resource exhausted: {:?}", resource),
            LapceError::Timeout { operation, duration } => write!(f, "Operation {} timeout after {:?}", operation, duration),
            LapceError::ContextWindowExceeded { provider, message, .. } => write!(f, "Context window exceeded for {}: {}", provider, message),
            LapceError::FileRestriction { tool_info, pattern, description, file_path } => {
                write!(f, "{} can only edit files matching pattern: {}{}. Got: {}", tool_info, pattern, description, file_path)
            },
            LapceError::CircuitOpen { component } => write!(f, "Circuit breaker open for component: {}", component),
            LapceError::ComponentNotFound(name) => write!(f, "Component not found: {}", name),
            LapceError::InvalidRequest { message, .. } => write!(f, "Invalid request: {}", message),
            LapceError::RateLimit { message, .. } => write!(f, "Rate limit exceeded: {}", message),
            LapceError::AuthenticationFailed { message, .. } => write!(f, "Authentication failed: {}", message),
            LapceError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            LapceError::Io(err) => write!(f, "IO error: {}", err),
            LapceError::Generic { context, message, .. } => write!(f, "{}: {}", context, message),
        }
    }
}

impl StdError for LapceError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            LapceError::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// Resource types that can be exhausted
#[derive(Debug, Clone)]
pub enum ResourceType {
    Memory,
    Connections,
    FileHandles,
    Threads,
    Tokens,
    Cache,
    Custom(String),
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::Memory => write!(f, "memory"),
            ResourceType::Connections => write!(f, "connections"),
            ResourceType::FileHandles => write!(f, "file handles"),
            ResourceType::Threads => write!(f, "threads"),
            ResourceType::Tokens => write!(f, "tokens"),
            ResourceType::Cache => write!(f, "cache"),
            ResourceType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Result type alias for Lapce operations
pub type Result<T> = std::result::Result<T, LapceError>;

/// Error type classification for recovery strategies
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorType {
    Transient,
    Permanent,
    RateLimit,
    Network,
    Timeout,
    Authentication,
    Permission,
    ResourceNotFound,
    InvalidInput,
    InternalError,
    ContextWindow,
    Serialization,
    FileRestriction,
    ResourceExhausted,
    ResourceExhaustion, // Alias for compatibility
    Validation,
    CircuitBreaker,
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorType::Transient => write!(f, "Transient"),
            ErrorType::Permanent => write!(f, "Permanent"),
            ErrorType::RateLimit => write!(f, "RateLimit"),
            ErrorType::Network => write!(f, "Network"),
            ErrorType::Timeout => write!(f, "Timeout"),
            ErrorType::Authentication => write!(f, "Authentication"),
            ErrorType::Permission => write!(f, "Permission"),
            ErrorType::ResourceNotFound => write!(f, "ResourceNotFound"),
            ErrorType::InvalidInput => write!(f, "InvalidInput"),
            ErrorType::InternalError => write!(f, "InternalError"),
            ErrorType::ResourceExhausted => write!(f, "ResourceExhausted"),
            ErrorType::ResourceExhaustion => write!(f, "ResourceExhaustion"),
            ErrorType::Validation => write!(f, "Validation"),
            ErrorType::ContextWindow => write!(f, "ContextWindow"),
            ErrorType::Serialization => write!(f, "Serialization"),
            ErrorType::FileRestriction => write!(f, "FileRestriction"),
            ErrorType::CircuitBreaker => write!(f, "CircuitBreaker"),
        }
    }
}

impl LapceError {
    /// Classify the error type for recovery strategies
    pub fn classify(&self) -> ErrorType {
        match self {
            LapceError::Timeout { .. } => ErrorType::Transient,
            LapceError::RateLimit { .. } => ErrorType::RateLimit,
            LapceError::ResourceExhausted { .. } => ErrorType::ResourceExhausted,
            LapceError::CircuitOpen { .. } => ErrorType::CircuitBreaker,
            LapceError::AuthenticationFailed { .. } => ErrorType::Authentication,
            LapceError::InvalidRequest { .. } => ErrorType::Permanent,
            LapceError::ContextWindowExceeded { .. } => ErrorType::ContextWindow,
            LapceError::FileRestriction { .. } => ErrorType::FileRestriction,
            LapceError::Serialization(_) => ErrorType::Serialization,
            LapceError::Provider { retry_after: Some(_), .. } => ErrorType::RateLimit,
            LapceError::Provider { .. } => ErrorType::Transient,
            LapceError::Ipc { .. } => ErrorType::Transient,
            LapceError::Io(_) => ErrorType::Transient,
            LapceError::Parse { .. } => ErrorType::Permanent,
            LapceError::ComponentNotFound(_) => ErrorType::Permanent,
            LapceError::Generic { .. } => ErrorType::Transient,
        }
    }

    /// Check if error is retryable - matches TypeScript retry logic
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.classify(),
            ErrorType::Transient | ErrorType::RateLimit | ErrorType::ResourceExhausted
        )
    }

    /// Get retry delay if applicable - matches TypeScript backoff patterns
    pub fn retry_delay(&self) -> Option<Duration> {
        match self {
            LapceError::Provider { retry_after, .. } => *retry_after,
            LapceError::RateLimit { retry_after, .. } => *retry_after,
            LapceError::Timeout { .. } => Some(Duration::from_millis(1000)),
            LapceError::ResourceExhausted { .. } => Some(Duration::from_millis(5000)),
            _ => None,
        }
    }

    /// Check if this is a context window error - matches checkContextWindowExceededError
    pub fn is_context_window_error(&self) -> bool {
        match self {
            LapceError::ContextWindowExceeded { .. } => true,
            LapceError::InvalidRequest { message, .. } => {
                // Match TypeScript patterns for context window errors
                let patterns = [
                    "context length",
                    "maximum context",
                    "tokens exceed",
                    "too many tokens",
                    "prompt is too long",
                    "context too long",
                    "exceeds context",
                    "token limit",
                    "context_length_exceeded",
                    "max_tokens_to_sample",
                ];
                
                let msg_lower = message.to_lowercase();
                patterns.iter().any(|pattern| msg_lower.contains(pattern))
            }
            LapceError::Provider { message, .. } => {
                let msg_lower = message.to_lowercase();
                msg_lower.contains("context") || msg_lower.contains("token")
            }
            _ => false,
        }
    }
}

/// Error severity levels for reporting - matches TypeScript severity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize, Hash)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl LapceError {
    /// Get error severity for reporting
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            LapceError::ResourceExhausted { .. } => ErrorSeverity::Critical,
            LapceError::CircuitOpen { .. } => ErrorSeverity::Critical,
            LapceError::AuthenticationFailed { .. } => ErrorSeverity::Critical,
            LapceError::Provider { .. } => ErrorSeverity::Warning,
            LapceError::Parse { .. } => ErrorSeverity::Info,
            LapceError::Timeout { .. } => ErrorSeverity::Warning,
            LapceError::RateLimit { .. } => ErrorSeverity::Warning,
            _ => ErrorSeverity::Error,
        }
    }
}

/// Helper function to stringify errors - matches TypeScript stringifyError
pub fn stringify_error(error: &dyn std::error::Error) -> String {
    // Match TypeScript: return error.stack || error.message
    // Note: backtrace is not available on the standard Error trait
    // We'll just use the error string representation
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let timeout_err = LapceError::Timeout {
            operation: "test".to_string(),
            duration: Duration::from_secs(5),
        };
        assert_eq!(timeout_err.classify(), ErrorType::Timeout);
        assert!(timeout_err.is_retryable());
    }

    #[test]
    fn test_context_window_detection() {
        let ctx_err = LapceError::ContextWindowExceeded {
            provider: "openai".to_string(),
            message: "context length exceeded".to_string(),
            max_tokens: Some(4096),
            used_tokens: Some(5000),
        };
        assert!(ctx_err.is_context_window_error());

        let invalid_req = LapceError::InvalidRequest {
            message: "maximum context length exceeded".to_string(),
            error_type: "invalid_request_error".to_string(),
            status_code: 400,
        };
        assert!(invalid_req.is_context_window_error());
    }

    #[test]
    fn test_retry_delay() {
        let rate_limit = LapceError::RateLimit {
            message: "too many requests".to_string(),
            retry_after: Some(Duration::from_secs(60)),
            provider: Some("anthropic".to_string()),
        };
        assert_eq!(rate_limit.retry_delay(), Some(Duration::from_secs(60)));
    }
}
