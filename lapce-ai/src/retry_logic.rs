/// Exponential Backoff Retry Logic for Provider Calls
use std::time::Duration;
use anyhow::Result;
use tokio::time::sleep;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            exponential_base: 2.0,
            jitter: true,
        }
    }
}

pub struct RetryManager {
    config: RetryConfig,
}

impl RetryManager {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }
    
    pub async fn execute_with_retry<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = self.config.initial_delay;
        
        loop {
            attempt += 1;
            debug!("Attempt {}/{} for {}", attempt, self.config.max_retries + 1, operation_name);
            
            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        debug!("Operation {} succeeded after {} attempts", operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(e) if attempt > self.config.max_retries => {
                    warn!("Operation {} failed after {} attempts: {}", 
                          operation_name, attempt, e);
                    return Err(e);
                }
                Err(e) => {
                    warn!("Operation {} failed on attempt {}: {}", 
                          operation_name, attempt, e);
                    
                    // Apply jitter if enabled
                    let mut actual_delay = delay;
                    if self.config.jitter {
                        use rand::Rng;
                        let jitter_amount = rand::thread_rng().gen_range(0..=delay.as_millis() as u64 / 4);
                        actual_delay += Duration::from_millis(jitter_amount);
                    }
                    
                    debug!("Waiting {:?} before retry", actual_delay);
                    sleep(actual_delay).await;
                    
                    // Calculate next delay with exponential backoff
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * self.config.exponential_base)
                            .min(self.config.max_delay.as_secs_f64())
                    );
                }
            }
        }
    }
    
    pub async fn execute_with_retry_and_fallback<F, Fut, T, G, Gut>(
        &self,
        primary: F,
        fallback: G,
        operation_name: &str,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
        G: Fn() -> Gut,
        Gut: std::future::Future<Output = Result<T>>,
    {
        match self.execute_with_retry(primary, operation_name).await {
            Ok(result) => Ok(result),
            Err(primary_err) => {
                warn!("Primary operation failed, trying fallback: {}", primary_err);
                self.execute_with_retry(fallback, &format!("{}_fallback", operation_name)).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_successful_first_attempt() {
        let manager = RetryManager::new(RetryConfig::default());
        
        let result = manager.execute_with_retry(
            || async { Ok::<_, anyhow::Error>(42) },
            "test_op"
        ).await;
        
        assert_eq!(result.unwrap(), 42);
    }
    
    #[tokio::test]
    async fn test_retry_then_success() {
        let manager = RetryManager::new(RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            exponential_base: 2.0,
            jitter: false,
        });
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = manager.execute_with_retry(
            || {
                let count = counter_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if count < 2 {
                        Err(anyhow::anyhow!("Temporary failure"))
                    } else {
                        Ok(42)
                    }
                }
            },
            "test_op"
        ).await;
        
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
    
    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let manager = RetryManager::new(RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            exponential_base: 2.0,
            jitter: false,
        });
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = manager.execute_with_retry(
            || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Err::<i32, _>(anyhow::anyhow!("Always fails")) }
            },
            "test_op"
        ).await;
        
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Initial + 2 retries
    }
    
    #[tokio::test]
    async fn test_fallback() {
        let manager = RetryManager::new(RetryConfig {
            max_retries: 1,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            exponential_base: 2.0,
            jitter: false,
        });
        
        let result = manager.execute_with_retry_and_fallback(
            || async { Err::<i32, _>(anyhow::anyhow!("Primary fails")) },
            || async { Ok(42) },
            "test_op"
        ).await;
        
        assert_eq!(result.unwrap(), 42);
    }
}
