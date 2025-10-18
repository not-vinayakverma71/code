/// Structured error types for IPC system
/// Replaces unwrap/expect/panic with proper error handling

use thiserror::Error;
use std::io;
use anyhow;

/// Main IPC error type covering all failure modes
#[derive(Error, Debug)]
pub enum IpcError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Protocol error: {message}")]
    Protocol { message: String },
    
    #[error("Authentication failed: {reason}")]
    Authentication { reason: String },
    
    #[error("Connection error: {details}")]
    Connection { details: String },
    
    #[error("Timeout after {seconds}s: {operation}")]
    Timeout { seconds: u64, operation: String },
    
    #[error("Buffer full: {capacity} bytes, tried to write {attempted}")]
    BufferFull { capacity: usize, attempted: usize },
    
    #[error("Invalid message: {reason}")]
    InvalidMessage { reason: String },
    
    #[error("Codec error: {codec_type} - {details}")]
    Codec { codec_type: String, details: String },
    
    #[error("Shared memory error: {operation} failed - {reason}")]
    SharedMemory { operation: String, reason: String },
    
    #[error("Security violation: {violation}")]
    Security { violation: String },
    
    #[error("Rate limit exceeded: {limit} req/s")]
    RateLimit { limit: u32 },
    
    #[error("Configuration error: {field} - {issue}")]
    Configuration { field: String, issue: String },
    
    #[error("Internal error: {context}")]
    Internal { context: String },
    
    #[error("Handler error: {message}")]
    Handler { message: String },
    
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Result type alias for IPC operations
pub type IpcResult<T> = Result<T, IpcError>;

/// Extensions for converting common error patterns
impl IpcError {
    /// Create protocol error from context
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::Protocol { 
            message: message.into() 
        }
    }
    
    /// Create authentication error
    pub fn auth(reason: impl Into<String>) -> Self {
        Self::Authentication { 
            reason: reason.into() 
        }
    }
    
    /// Create connection error
    pub fn connection(details: impl Into<String>) -> Self {
        Self::Connection { 
            details: details.into() 
        }
    }
    
    /// Create timeout error
    pub fn timeout(seconds: u64, operation: impl Into<String>) -> Self {
        Self::Timeout { 
            seconds, 
            operation: operation.into() 
        }
    }
    
    /// Create buffer full error
    pub fn buffer_full(capacity: usize, attempted: usize) -> Self {
        Self::BufferFull { capacity, attempted }
    }
    
    /// Create invalid message error
    pub fn invalid_message(reason: impl Into<String>) -> Self {
        Self::InvalidMessage { 
            reason: reason.into() 
        }
    }
    
    /// Create codec error
    pub fn codec(codec_type: impl Into<String>, details: impl Into<String>) -> Self {
        Self::Codec { 
            codec_type: codec_type.into(), 
            details: details.into() 
        }
    }
    
    
    /// Create shared memory error
    pub fn shm(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::SharedMemory { 
            operation: operation.into(), 
            reason: reason.into() 
        }
    }
    
    /// Create security error
    pub fn security(violation: impl Into<String>) -> Self {
        Self::Security { 
            violation: violation.into() 
        }
    }
    
    /// Create rate limit error
    pub fn rate_limit(limit: u32) -> Self {
        Self::RateLimit { limit }
    }
    
    /// Create configuration error
    pub fn config(field: impl Into<String>, issue: impl Into<String>) -> Self {
        Self::Configuration { 
            field: field.into(), 
            issue: issue.into() 
        }
    }
    
    /// Create internal error
    pub fn internal(context: impl Into<String>) -> Self {
        Self::Internal { 
            context: context.into()
        }
    }
    
    /// Create handler error
    pub fn handler(message: impl Into<String>) -> Self {
        Self::Internal { 
            context: format!("Handler: {}", message.into())
        }
    }
}

/// Helper trait for safer time operations
pub trait SafeSystemTime {
    fn safe_elapsed_secs(&self) -> IpcResult<u64>;
    fn safe_duration_since_epoch(&self) -> IpcResult<u64>;
}

impl SafeSystemTime for std::time::SystemTime {
    fn safe_elapsed_secs(&self) -> IpcResult<u64> {
        self.duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| IpcError::internal(format!("System time error: {}", e)))
            .map(|d| d.as_secs())
    }
    
    fn safe_duration_since_epoch(&self) -> IpcResult<u64> {
        self.duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| IpcError::internal(format!("System time before epoch: {}", e)))
            .map(|d| d.as_secs())
    }
}

/// Helper trait for safer async operations
pub trait SafeAsyncOps<T> {
    async fn with_timeout_op(self, seconds: u64, operation: &str) -> IpcResult<T>;
}

impl<F> SafeAsyncOps<F::Output> for F 
where
    F: std::future::Future,
{
    async fn with_timeout_op(self, seconds: u64, operation: &str) -> IpcResult<F::Output> {
        tokio::time::timeout(tokio::time::Duration::from_secs(seconds), self)
            .await
            .map_err(|_| IpcError::timeout(seconds, operation))
    }
}

/// Logging integration for structured errors
impl IpcError {
    /// Log error with appropriate level
    pub fn log_error(&self) {
        use tracing::{error, warn, debug};
        
        match self {
            IpcError::Io(e) => error!("IO error: {}", e),
            IpcError::Protocol { message } => warn!("Protocol error: {}", message),
            IpcError::Authentication { reason } => warn!("Auth failed: {}", reason),
            IpcError::Connection { details } => debug!("Connection issue: {}", details),
            IpcError::Timeout { seconds, operation } => warn!("Timeout: {} after {}s", operation, seconds),
            IpcError::BufferFull { capacity, attempted } => debug!("Buffer full: {}/{}", attempted, capacity),
            IpcError::InvalidMessage { reason } => warn!("Invalid message: {}", reason),
            IpcError::Codec { codec_type, details } => error!("Codec error ({}): {}", codec_type, details),
            IpcError::SharedMemory { operation, reason } => error!("SHM error ({}): {}", operation, reason),
            IpcError::Security { violation } => error!("Security violation: {}", violation),
            IpcError::RateLimit { limit } => debug!("Rate limited: {} req/s", limit),
            IpcError::Configuration { field, issue } => error!("Config error ({}): {}", field, issue),
            IpcError::Internal { context } => error!("Internal error: {}", context),
            IpcError::Handler { message } => error!("Handler error: {}", message),
            IpcError::Anyhow(e) => error!("Anyhow error: {}", e),
        }
    }
    
    /// Log and return error
    pub fn log_and_return<T>(self) -> IpcResult<T> {
        self.log_error();
        Err(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_creation() {
        let err = IpcError::protocol("Invalid header");
        assert!(err.to_string().contains("Invalid header"));
        
        let err = IpcError::timeout(30, "handshake");
        assert!(err.to_string().contains("30s"));
        assert!(err.to_string().contains("handshake"));
    }
    
    #[test]
    fn test_safe_system_time() {
        let now = std::time::SystemTime::now();
        let result = now.safe_duration_since_epoch();
        assert!(result.is_ok());
        
        let secs = result.unwrap();
        assert!(secs > 1_600_000_000); // After 2020
    }
    
    #[test]
    fn test_error_logging() {
        // Should not panic
        let err = IpcError::internal("test context");
        err.log_error();
    }
}
