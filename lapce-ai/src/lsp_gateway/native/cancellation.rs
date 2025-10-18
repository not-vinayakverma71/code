/// Request Cancellation (LSP-020)
/// Cancellation tokens for graceful request termination

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use parking_lot::Mutex;

/// Cancellation token for a request
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    request_id: String,
}

impl CancellationToken {
    pub fn new(request_id: String) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            request_id,
        }
    }
    
    /// Cancel the token
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        tracing::debug!(request_id = %self.request_id, "Request cancelled");
    }
    
    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
    
    /// Check and throw error if cancelled
    pub fn check_cancelled(&self) -> Result<(), CancellationError> {
        if self.is_cancelled() {
            Err(CancellationError {
                request_id: self.request_id.clone(),
            })
        } else {
            Ok(())
        }
    }
    
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
}

/// Cancellation error
#[derive(Debug, Clone)]
pub struct CancellationError {
    pub request_id: String,
}

impl std::fmt::Display for CancellationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Request {} was cancelled", self.request_id)
    }
}

impl std::error::Error for CancellationError {}

/// Registry for managing cancellation tokens
pub struct CancellationRegistry {
    tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl CancellationRegistry {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Register a new request
    pub fn register(&self, request_id: String) -> CancellationToken {
        let token = CancellationToken::new(request_id.clone());
        self.tokens.lock().insert(request_id, token.clone());
        token
    }
    
    /// Cancel a request
    pub fn cancel(&self, request_id: &str) -> bool {
        if let Some(token) = self.tokens.lock().get(request_id) {
            token.cancel();
            true
        } else {
            tracing::warn!(request_id = %request_id, "Attempted to cancel unknown request");
            false
        }
    }
    
    /// Remove a completed request
    pub fn remove(&self, request_id: &str) {
        self.tokens.lock().remove(request_id);
    }
    
    /// Get token for request
    pub fn get(&self, request_id: &str) -> Option<CancellationToken> {
        self.tokens.lock().get(request_id).cloned()
    }
    
    /// Get count of active requests
    pub fn active_count(&self) -> usize {
        self.tokens.lock().len()
    }
    
    /// Cancel all requests
    pub fn cancel_all(&self) {
        let tokens = self.tokens.lock();
        for token in tokens.values() {
            token.cancel();
        }
        tracing::info!("Cancelled {} active requests", tokens.len());
    }
}

impl Default for CancellationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for checking cancellation in loops
#[macro_export]
macro_rules! check_cancelled {
    ($token:expr) => {
        if $token.is_cancelled() {
            return Err($crate::lsp_gateway::native::CancellationError {
                request_id: $token.request_id().to_string(),
            }.into());
        }
    };
}

/// Helper for cancellable operations
pub struct CancellableOperation<T> {
    token: CancellationToken,
    operation: Box<dyn FnOnce() -> anyhow::Result<T> + Send>,
}

impl<T> CancellableOperation<T> {
    pub fn new<F>(token: CancellationToken, operation: F) -> Self
    where
        F: FnOnce() -> anyhow::Result<T> + Send + 'static,
    {
        Self {
            token,
            operation: Box::new(operation),
        }
    }
    
    /// Execute with periodic cancellation checks
    pub fn execute(self) -> anyhow::Result<T> {
        self.token.check_cancelled()?;
        (self.operation)()
    }
}

/// Timeout wrapper for operations
pub struct TimeoutConfig {
    /// Default timeout for requests (30 seconds)
    pub default_timeout_secs: u64,
    /// Timeout for parse operations (10 seconds)
    pub parse_timeout_secs: u64,
    /// Timeout for index operations (60 seconds)
    pub index_timeout_secs: u64,
    /// Timeout for symbol search (5 seconds)
    pub symbol_search_timeout_secs: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout_secs: 30,
            parse_timeout_secs: 10,
            index_timeout_secs: 60,
            symbol_search_timeout_secs: 5,
        }
    }
}

impl TimeoutConfig {
    /// Get timeout for LSP method
    pub fn timeout_for_method(&self, method: &str) -> std::time::Duration {
        let secs = match method {
            "textDocument/didOpen" | "textDocument/didChange" => self.parse_timeout_secs,
            "textDocument/definition" | "textDocument/references" => self.symbol_search_timeout_secs,
            "workspace/symbol" => self.index_timeout_secs,
            _ => self.default_timeout_secs,
        };
        std::time::Duration::from_secs(secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new("test-123".to_string());
        
        assert!(!token.is_cancelled());
        assert!(token.check_cancelled().is_ok());
        
        token.cancel();
        
        assert!(token.is_cancelled());
        assert!(token.check_cancelled().is_err());
    }
    
    #[test]
    fn test_cancellation_registry() {
        let registry = CancellationRegistry::new();
        
        let token1 = registry.register("req-1".to_string());
        let token2 = registry.register("req-2".to_string());
        
        assert_eq!(registry.active_count(), 2);
        
        assert!(registry.cancel("req-1"));
        assert!(token1.is_cancelled());
        assert!(!token2.is_cancelled());
        
        registry.remove("req-1");
        assert_eq!(registry.active_count(), 1);
        
        registry.cancel_all();
        assert!(token2.is_cancelled());
    }
    
    #[test]
    fn test_token_clone() {
        let token1 = CancellationToken::new("test".to_string());
        let token2 = token1.clone();
        
        token1.cancel();
        
        // Both should see cancellation
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }
    
    #[test]
    fn test_timeout_config() {
        let config = TimeoutConfig::default();
        
        assert_eq!(
            config.timeout_for_method("textDocument/didOpen"),
            std::time::Duration::from_secs(10)
        );
        
        assert_eq!(
            config.timeout_for_method("textDocument/definition"),
            std::time::Duration::from_secs(5)
        );
        
        assert_eq!(
            config.timeout_for_method("workspace/symbol"),
            std::time::Duration::from_secs(60)
        );
        
        assert_eq!(
            config.timeout_for_method("unknown/method"),
            std::time::Duration::from_secs(30)
        );
    }
    
    #[tokio::test]
    async fn test_cancellable_operation() {
        let token = CancellationToken::new("test".to_string());
        
        let op = CancellableOperation::new(token.clone(), || {
            Ok(42)
        });
        
        let result = op.execute();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        
        // Test with cancelled token
        token.cancel();
        let op2 = CancellableOperation::new(token.clone(), || {
            Ok(100)
        });
        
        let result2 = op2.execute();
        assert!(result2.is_err());
    }
}
