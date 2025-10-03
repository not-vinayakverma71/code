//! Timeout handling for parsing operations
//! Prevents hanging on large or malformed files

use std::future::Future;
use std::time::Duration;
use tokio::time::timeout;
use crate::error::{TreeSitterError, Result};

// PRODUCTION-GRADE TIMEOUTS FOR 30K+ FILES
/// Default timeout for parsing operations (30 seconds - handle large files)
pub const DEFAULT_PARSE_TIMEOUT_MS: u64 = 30000;

/// Maximum timeout for massive files (10 minutes - never give up)
pub const MAX_PARSE_TIMEOUT_MS: u64 = 600000;

/// Timeout for query execution (30 seconds)
pub const QUERY_TIMEOUT_MS: u64 = 30000;

/// Timeout for symbol extraction (5 minutes - complex extractions)
pub const SYMBOL_EXTRACTION_TIMEOUT_MS: u64 = 300000;

/// Timeout configuration
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub parse_timeout: Duration,
    pub query_timeout: Duration,
    pub symbol_extraction_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            parse_timeout: Duration::from_millis(DEFAULT_PARSE_TIMEOUT_MS),
            query_timeout: Duration::from_millis(QUERY_TIMEOUT_MS),
            symbol_extraction_timeout: Duration::from_millis(SYMBOL_EXTRACTION_TIMEOUT_MS),
        }
    }
}

impl TimeoutConfig {
    /// Adjust timeout based on file size - PRODUCTION GRADE
    pub fn for_file_size(file_size_bytes: usize) -> Self {
        let mut config = Self::default();
        
        // Scale timeout aggressively for large files
        // 1MB = 30s, 100MB = 2min, 500MB = 5min, 1GB+ = 10min
        let size_mb = file_size_bytes / (1024 * 1024);
        let parse_timeout_ms = if size_mb < 1 {
            DEFAULT_PARSE_TIMEOUT_MS
        } else if size_mb < 100 {
            DEFAULT_PARSE_TIMEOUT_MS + (size_mb as u64 * 1000)  // +1s per MB
        } else if size_mb < 500 {
            120000 + (size_mb as u64 * 500)  // 2min base + 500ms per MB
        } else {
            MAX_PARSE_TIMEOUT_MS  // 10 minutes for massive files
        };
        
        config.parse_timeout = Duration::from_millis(parse_timeout_ms);
        config
    }
}

/// Execute an operation with timeout
pub async fn with_timeout<F, T>(
    future: F,
    duration: Duration,
    operation: &str,
) -> Result<T>
where
    F: Future<Output = T>,
{
    match timeout(duration, future).await {
        Ok(result) => Ok(result),
        Err(_) => {
            tracing::warn!(
                operation = operation,
                timeout_ms = duration.as_millis(),
                "Operation timed out"
            );
            Err(TreeSitterError::Timeout {
                operation: operation.to_string(),
                timeout_ms: duration.as_millis() as u64,
            })
        }
    }
}

/// Execute parse operation with adaptive timeout
pub async fn with_parse_timeout<F, T>(
    future: F,
    file_size: usize,
    file_name: &str,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let config = TimeoutConfig::for_file_size(file_size);
    
    tracing::debug!(
        file = file_name,
        size_mb = file_size / (1024 * 1024),
        timeout_ms = config.parse_timeout.as_millis(),
        "Starting parse with timeout"
    );
    
    match timeout(config.parse_timeout, future).await {
        Ok(result) => result,
        Err(_) => {
            tracing::error!(
                file = file_name,
                size_mb = file_size / (1024 * 1024),
                timeout_ms = config.parse_timeout.as_millis(),
                "Parse operation timed out"
            );
            Err(TreeSitterError::Timeout {
                operation: format!("parse file: {}", file_name),
                timeout_ms: config.parse_timeout.as_millis() as u64,
            })
        }
    }
}

/// Circuit breaker for repeated timeouts
pub struct CircuitBreaker {
    failure_count: std::sync::atomic::AtomicUsize,
    threshold: usize,
    reset_after: Duration,
    last_failure: std::sync::RwLock<Option<std::time::Instant>>,
}

impl CircuitBreaker {
    pub fn new(threshold: usize, reset_after: Duration) -> Self {
        Self {
            failure_count: std::sync::atomic::AtomicUsize::new(0),
            threshold,
            reset_after,
            last_failure: std::sync::RwLock::new(None),
        }
    }

    pub fn is_open(&self) -> bool {
        let count = self.failure_count.load(std::sync::atomic::Ordering::Relaxed);
        if count >= self.threshold {
            // Check if we should reset
            if let Some(last) = *self.last_failure.read().unwrap() {
                if last.elapsed() > self.reset_after {
                    self.reset();
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn record_success(&self) {
        self.reset();
    }

    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self.last_failure.write().unwrap() = Some(std::time::Instant::now());
    }

    fn reset(&self) {
        self.failure_count.store(0, std::sync::atomic::Ordering::Relaxed);
        *self.last_failure.write().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timeout_triggers() {
        let result = with_timeout(
            async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                42
            },
            Duration::from_millis(50),
            "test_operation",
        )
        .await;

        assert!(result.is_err());
        if let Err(TreeSitterError::Timeout { operation, .. }) = result {
            assert_eq!(operation, "test_operation");
        }
    }

    #[tokio::test]
    async fn test_timeout_succeeds() {
        let result = with_timeout(
            async { 42 },
            Duration::from_millis(100),
            "test_operation",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(1));
        
        assert!(!breaker.is_open());
        
        breaker.record_failure();
        breaker.record_failure();
        assert!(!breaker.is_open());
        
        breaker.record_failure();
        assert!(breaker.is_open());
        
        breaker.record_success();
        assert!(!breaker.is_open());
    }
}
