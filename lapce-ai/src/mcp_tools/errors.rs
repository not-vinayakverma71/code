// MCP Tools Error Types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Timeout: operation took longer than {0} seconds")]
    Timeout(u64),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(String),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    
    #[error("Sandboxing error: {0}")]
    SandboxingError(String),
    
    #[error("Cache error: {0}")]
    CacheError(String),
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            exponential_base: 2.0,
        }
    }
}
