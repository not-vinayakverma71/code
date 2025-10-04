/// Exact 1:1 Translation of TypeScript timeout and retry logic from codex-reference/api/providers/utils/timeout-config.ts
/// and retry logic from Task.ts
/// DAY 4 H5-6: Port timeout and retry logic

use std::time::Duration;
use tokio::time::{sleep, timeout};

/// Gets the API request timeout from configuration with validation
/// Exact translation lines 8-21
pub fn get_api_request_timeout() -> Duration {
    // Get timeout with validation
    let config_timeout = get_config_value("kilo-code.apiRequestTimeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(600); // Default to 600 seconds
    
    // Validate that it's a valid non-negative number
    let timeout_seconds = if config_timeout == 0 {
        0 // Allow 0 (no timeout)
    } else {
        config_timeout.max(1) // Ensure at least 1 second if not 0
    };
    
    Duration::from_secs(timeout_seconds)
}

/// Get configuration value (placeholder for VSCode config)
fn get_config_value(key: &str) -> Option<serde_json::Value> {
    // In production, this would read from actual configuration
    // For now, return default values
    match key {
        "kilo-code.apiRequestTimeout" => Some(serde_json::json!(600)),
        _ => None,
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 600_000, // 10 minutes
            exponential_base: 2.0,
        }
    }
}

/// Retry with exponential backoff
pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    config: RetryConfig,
) -> Result<T, E>
where
    F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>> + Clone,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    
    loop {
        match timeout(get_api_request_timeout(), operation()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) if attempt < config.max_retries => {
                attempt += 1;
                
                // Calculate exponential backoff delay
                let delay_ms = calculate_backoff_delay(
                    attempt,
                    config.initial_delay_ms,
                    config.max_delay_ms,
                    config.exponential_base,
                );
                
                println!("Retry attempt {}/{} after {}ms: {}", 
                    attempt, config.max_retries, delay_ms, e);
                
                sleep(Duration::from_millis(delay_ms)).await;
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                if attempt < config.max_retries {
                    attempt += 1;
                    let delay_ms = calculate_backoff_delay(
                        attempt,
                        config.initial_delay_ms,
                        config.max_delay_ms,
                        config.exponential_base,
                    );
                    
                    println!("Request timeout, retry {}/{} after {}ms", 
                        attempt, config.max_retries, delay_ms);
                    
                    sleep(Duration::from_millis(delay_ms)).await;
                } else {
                    panic!("Request timeout after {} retries", config.max_retries);
                }
            }
        }
    }
}

/// Calculate exponential backoff delay
fn calculate_backoff_delay(
    attempt: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    exponential_base: f64,
) -> u64 {
    let exponential_delay = (initial_delay_ms as f64) * exponential_base.powi(attempt as i32 - 1);
    exponential_delay.min(max_delay_ms as f64) as u64
}

/// Rate limiting with token bucket
pub struct RateLimiter {
    tokens: std::sync::Arc<tokio::sync::Mutex<f64>>,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: std::sync::Arc<tokio::sync::Mutex<std::time::Instant>>,
}

impl RateLimiter {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: std::sync::Arc::new(tokio::sync::Mutex::new(max_tokens)),
            max_tokens,
            refill_rate,
            last_refill: std::sync::Arc::new(tokio::sync::Mutex::new(std::time::Instant::now())),
        }
    }
    
    /// Wait for available tokens
    pub async fn acquire(&self, tokens_needed: f64) {
        loop {
            let mut tokens = self.tokens.lock().await;
            let mut last_refill = self.last_refill.lock().await;
            
            // Refill tokens based on time elapsed
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(*last_refill).as_secs_f64();
            let tokens_to_add = (elapsed * self.refill_rate).min(self.max_tokens - *tokens);
            
            *tokens += tokens_to_add;
            *last_refill = now;
            
            if *tokens >= tokens_needed {
                *tokens -= tokens_needed;
                break;
            }
            
            // Calculate wait time
            let tokens_deficit = tokens_needed - *tokens;
            let wait_seconds = tokens_deficit / self.refill_rate;
            
            drop(tokens);
            drop(last_refill);
            
            sleep(Duration::from_secs_f64(wait_seconds)).await;
        }
    }
}

/// Timeout wrapper for async operations
pub async fn with_timeout<F, T>(
    duration: Duration,
    future: F,
) -> Result<T, TimeoutError>
where
    F: std::future::Future<Output = T>,
{
    match timeout(duration, future).await {
        Ok(result) => Ok(result),
        Err(_) => Err(TimeoutError::TimedOut),
    }
}

/// Timeout error type
#[derive(Debug, Clone)]
pub enum TimeoutError {
    TimedOut,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::TimedOut => write!(f, "Operation timed out"),
        }
    }
}

impl std::error::Error for TimeoutError {}

/// Circuit breaker for handling repeated failures
pub struct CircuitBreaker {
    failure_count: std::sync::Arc<std::sync::atomic::AtomicU32>,
    last_failure_time: std::sync::Arc<tokio::sync::Mutex<Option<std::time::Instant>>>,
    threshold: u32,
    timeout_duration: Duration,
    state: std::sync::Arc<tokio::sync::Mutex<CircuitState>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, timeout_duration: Duration) -> Self {
        Self {
            failure_count: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            last_failure_time: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            threshold,
            timeout_duration,
            state: std::sync::Arc::new(tokio::sync::Mutex::new(CircuitState::Closed)),
        }
    }
    
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: From<&'static str>,
    {
        let mut state = self.state.lock().await;
        
        // Check if circuit should transition from Open to HalfOpen
        if *state == CircuitState::Open {
            if let Some(last_failure) = *self.last_failure_time.lock().await {
                if last_failure.elapsed() >= self.timeout_duration {
                    *state = CircuitState::HalfOpen;
                }
            }
        }
        
        match *state {
            CircuitState::Open => {
                Err(E::from("Circuit breaker is open"))
            }
            CircuitState::Closed | CircuitState::HalfOpen => {
                drop(state);
                
                match operation.await {
                    Ok(result) => {
                        // Success - reset failure count
                        self.failure_count.store(0, std::sync::atomic::Ordering::SeqCst);
                        *self.state.lock().await = CircuitState::Closed;
                        Ok(result)
                    }
                    Err(e) => {
                        // Failure - increment count
                        let count = self.failure_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        *self.last_failure_time.lock().await = Some(std::time::Instant::now());
                        
                        if count >= self.threshold {
                            *self.state.lock().await = CircuitState::Open;
                        }
                        
                        Err(e)
                    }
                }
            }
        }
    }
    
    pub async fn reset(&self) {
        self.failure_count.store(0, std::sync::atomic::Ordering::SeqCst);
        *self.state.lock().await = CircuitState::Closed;
        *self.last_failure_time.lock().await = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate_backoff_delay() {
        assert_eq!(calculate_backoff_delay(1, 1000, 10000, 2.0), 1000);
        assert_eq!(calculate_backoff_delay(2, 1000, 10000, 2.0), 2000);
        assert_eq!(calculate_backoff_delay(3, 1000, 10000, 2.0), 4000);
        assert_eq!(calculate_backoff_delay(4, 1000, 10000, 2.0), 8000);
        assert_eq!(calculate_backoff_delay(5, 1000, 10000, 2.0), 10000); // capped at max
    }
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10.0, 5.0);
        
        // Should succeed immediately
        limiter.acquire(5.0).await;
        
        // Should succeed with remaining tokens
        limiter.acquire(5.0).await;
        
        // Should wait for refill
        let start = std::time::Instant::now();
        limiter.acquire(5.0).await;
        let elapsed = start.elapsed();
        
        // Should have waited approximately 1 second for refill
        assert!(elapsed >= Duration::from_millis(900));
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(2, Duration::from_secs(1));
        
        // First failure
        let _ = breaker.call(async { Err::<(), &str>("error") }).await;
        
        // Second failure - should open circuit
        let _ = breaker.call(async { Err::<(), &str>("error") }).await;
        
        // Circuit should be open
        let result = breaker.call(async { Ok::<(), &str>(()) }).await;
        assert!(result.is_err());
        
        // Wait for timeout
        sleep(Duration::from_secs(2)).await;
        
        // Circuit should be half-open, success should close it
        let result = breaker.call(async { Ok::<(), &str>(()) }).await;
        assert!(result.is_ok());
    }
}
