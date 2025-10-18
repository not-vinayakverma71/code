// Exponential Backoff Utility - CHUNK-03: T08
// Generic retry logic with jitter for API and orchestrator retries

use std::time::Duration;
use rand::Rng;

/// Exponential backoff configuration
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,
    
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    
    /// Multiplier for each retry
    pub multiplier: f64,
    
    /// Maximum number of retries
    pub max_retries: u32,
    
    /// Enable jitter to prevent thundering herd
    pub enable_jitter: bool,
    
    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay_ms: 1000,      // 1 second
            max_delay_ms: 600_000,       // 10 minutes
            multiplier: 2.0,
            max_retries: 5,
            enable_jitter: true,
            jitter_factor: 0.3,          // ±30% jitter
        }
    }
}

/// Exponential backoff state tracker
pub struct BackoffState {
    config: BackoffConfig,
    current_attempt: u32,
    current_delay_ms: u64,
}

impl BackoffState {
    /// Create a new backoff state with given config
    pub fn new(config: BackoffConfig) -> Self {
        Self {
            current_delay_ms: config.initial_delay_ms,
            config,
            current_attempt: 0,
        }
    }
    
    /// Create with default config
    pub fn default() -> Self {
        Self::new(BackoffConfig::default())
    }
    
    /// Get the next delay duration and increment attempt counter
    /// Returns None if max retries exceeded
    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.current_attempt >= self.config.max_retries {
            return None;
        }
        
        let delay = self.calculate_delay();
        self.current_attempt += 1;
        
        // Calculate next delay for future use
        self.current_delay_ms = (self.current_delay_ms as f64 * self.config.multiplier) as u64;
        self.current_delay_ms = self.current_delay_ms.min(self.config.max_delay_ms);
        
        Some(Duration::from_millis(delay))
    }
    
    /// Calculate delay with optional jitter
    fn calculate_delay(&self) -> u64 {
        let base_delay = self.current_delay_ms;
        
        if !self.config.enable_jitter {
            return base_delay;
        }
        
        // Add jitter: delay * (1 ± jitter_factor)
        let mut rng = rand::thread_rng();
        let jitter_range = (base_delay as f64 * self.config.jitter_factor) as i64;
        let jitter = rng.gen_range(-jitter_range..=jitter_range);
        
        (base_delay as i64 + jitter).max(0) as u64
    }
    
    /// Reset the backoff state
    pub fn reset(&mut self) {
        self.current_attempt = 0;
        self.current_delay_ms = self.config.initial_delay_ms;
    }
    
    /// Get current attempt number
    pub fn attempt(&self) -> u32 {
        self.current_attempt
    }
    
    /// Check if retries exhausted
    pub fn is_exhausted(&self) -> bool {
        self.current_attempt >= self.config.max_retries
    }
    
    /// Get remaining retries
    pub fn remaining_retries(&self) -> u32 {
        self.config.max_retries.saturating_sub(self.current_attempt)
    }
}

/// Retry executor with exponential backoff
pub struct RetryExecutor {
    config: BackoffConfig,
}

impl RetryExecutor {
    /// Create a new retry executor
    pub fn new(config: BackoffConfig) -> Self {
        Self { config }
    }
    
    /// Execute a function with retry logic
    /// Retries on error until max retries or success
    pub async fn execute<F, Fut, T, E>(&self, mut operation: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let mut backoff = BackoffState::new(self.config.clone());
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if let Some(delay) = backoff.next_delay() {
                        tracing::warn!(
                            "Operation failed, retrying in {:?} (attempt {}/{})",
                            delay,
                            backoff.attempt(),
                            self.config.max_retries
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        tracing::error!(
                            "Operation failed after {} retries",
                            self.config.max_retries
                        );
                        return Err(err);
                    }
                }
            }
        }
    }
    
    /// Execute with a predicate to determine if retry should happen
    pub async fn execute_with_predicate<F, Fut, P, T, E>(
        &self,
        mut operation: F,
        should_retry: P,
    ) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        P: Fn(&E) -> bool,
    {
        let mut backoff = BackoffState::new(self.config.clone());
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if should_retry(&err) {
                        if let Some(delay) = backoff.next_delay() {
                            tracing::warn!(
                                "Operation failed with retryable error, retrying in {:?}",
                                delay
                            );
                            tokio::time::sleep(delay).await;
                        } else {
                            return Err(err);
                        }
                    } else {
                        tracing::info!("Operation failed with non-retryable error");
                        return Err(err);
                    }
                }
            }
        }
    }
}

impl Default for RetryExecutor {
    fn default() -> Self {
        Self::new(BackoffConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[test]
    fn test_backoff_config_default() {
        let config = BackoffConfig::default();
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 600_000);
        assert_eq!(config.multiplier, 2.0);
        assert_eq!(config.max_retries, 5);
        assert!(config.enable_jitter);
    }
    
    #[test]
    fn test_backoff_state_progression() {
        let config = BackoffConfig {
            initial_delay_ms: 100,
            max_delay_ms: 10_000,
            multiplier: 2.0,
            max_retries: 3,
            enable_jitter: false,
            jitter_factor: 0.0,
        };
        
        let mut backoff = BackoffState::new(config);
        
        // First delay: 100ms
        let delay1 = backoff.next_delay().unwrap();
        assert_eq!(delay1.as_millis(), 100);
        assert_eq!(backoff.attempt(), 1);
        
        // Second delay: 200ms
        let delay2 = backoff.next_delay().unwrap();
        assert_eq!(delay2.as_millis(), 200);
        assert_eq!(backoff.attempt(), 2);
        
        // Third delay: 400ms
        let delay3 = backoff.next_delay().unwrap();
        assert_eq!(delay3.as_millis(), 400);
        assert_eq!(backoff.attempt(), 3);
        
        // No more retries
        let delay4 = backoff.next_delay();
        assert!(delay4.is_none());
        assert!(backoff.is_exhausted());
    }
    
    #[test]
    fn test_backoff_max_delay_cap() {
        let config = BackoffConfig {
            initial_delay_ms: 1000,
            max_delay_ms: 2000,
            multiplier: 10.0,
            max_retries: 5,
            enable_jitter: false,
            jitter_factor: 0.0,
        };
        
        let mut backoff = BackoffState::new(config);
        
        backoff.next_delay().unwrap(); // 1000ms
        let delay2 = backoff.next_delay().unwrap(); // Should be capped at 2000ms
        assert_eq!(delay2.as_millis(), 2000);
    }
    
    #[test]
    fn test_backoff_jitter() {
        let config = BackoffConfig {
            initial_delay_ms: 1000,
            max_delay_ms: 10_000,
            multiplier: 2.0,
            max_retries: 3,
            enable_jitter: true,
            jitter_factor: 0.3,
        };
        
        let mut backoff = BackoffState::new(config);
        
        // With jitter, delay should be within 30% of base
        let delay = backoff.next_delay().unwrap().as_millis();
        assert!(delay >= 700 && delay <= 1300, "Delay {} out of expected range", delay);
    }
    
    #[test]
    fn test_backoff_reset() {
        let mut backoff = BackoffState::default();
        
        backoff.next_delay();
        backoff.next_delay();
        assert_eq!(backoff.attempt(), 2);
        
        backoff.reset();
        assert_eq!(backoff.attempt(), 0);
        assert!(!backoff.is_exhausted());
    }
    
    #[test]
    fn test_remaining_retries() {
        let config = BackoffConfig {
            max_retries: 5,
            ..Default::default()
        };
        
        let mut backoff = BackoffState::new(config);
        assert_eq!(backoff.remaining_retries(), 5);
        
        backoff.next_delay();
        assert_eq!(backoff.remaining_retries(), 4);
        
        backoff.next_delay();
        assert_eq!(backoff.remaining_retries(), 3);
    }
    
    #[tokio::test]
    async fn test_retry_executor_success_first_try() {
        let executor = RetryExecutor::default();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = executor.execute(|| async {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<_, String>(42)
        }).await;
        
        assert_eq!(result, Ok(42));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
    
    #[tokio::test]
    async fn test_retry_executor_success_after_retries() {
        let config = BackoffConfig {
            initial_delay_ms: 10,
            max_retries: 3,
            ..Default::default()
        };
        let executor = RetryExecutor::new(config);
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = executor.execute(|| {
            let c = counter_clone.clone();
            async move {
                let attempt = c.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err("Temporary failure")
                } else {
                    Ok::<_, &str>(100)
                }
            }
        }).await;
        
        assert_eq!(result, Ok(100));
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
    
    #[tokio::test]
    async fn test_retry_executor_exhausted() {
        let config = BackoffConfig {
            initial_delay_ms: 10,
            max_retries: 2,
            ..Default::default()
        };
        let executor = RetryExecutor::new(config);
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = executor.execute(|| {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err::<i32, _>("Always fails")
            }
        }).await;
        
        assert_eq!(result, Err("Always fails"));
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Initial + 2 retries
    }
    
    #[tokio::test]
    async fn test_retry_with_predicate() {
        let config = BackoffConfig {
            initial_delay_ms: 10,
            max_retries: 3,
            ..Default::default()
        };
        let executor = RetryExecutor::new(config);
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        // Only retry on "retryable" errors
        let result = executor.execute_with_predicate(
            || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, _>("non-retryable")
                }
            },
            |err| *err == "retryable"
        ).await;
        
        // Should fail immediately without retries
        assert_eq!(result, Err("non-retryable"));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
