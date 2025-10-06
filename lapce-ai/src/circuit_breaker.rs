/// Circuit Breaker Pattern - Day 36 PM
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker configuration
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
        }
    }
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    config: CircuitBreakerConfig,
    // Configuration
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    half_open_max_calls: u32,
    half_open_calls: Arc<AtomicU32>,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            config: CircuitBreakerConfig::default(),
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
            half_open_calls: Arc::new(AtomicU32::new(0)),
        }
    }
    
    pub async fn call<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::fmt::Debug,
    {
        let current_state = self.state.read().await.clone();
        
        match current_state {
            CircuitState::Open => {
                if self.should_attempt_reset().await {
                    *self.state.write().await = CircuitState::HalfOpen;
                    self.half_open_calls.store(0, Ordering::SeqCst);
                } else {
                    return Err(self.create_circuit_open_error());
                }
            }
            CircuitState::HalfOpen => {
                let calls = self.half_open_calls.fetch_add(1, Ordering::SeqCst);
                if calls >= self.half_open_max_calls {
                    return Err(self.create_circuit_open_error());
                }
            }
            _ => {}
        }
        
        match f() {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(error)
            }
        }
    }
    
    async fn should_attempt_reset(&self) -> bool {
        if let Some(last_failure) = *self.last_failure_time.read().await {
            last_failure.elapsed() > self.timeout
        } else {
            false
        }
    }
    
    async fn on_success(&self) {
        let current_state = self.state.read().await.clone();
        
        match current_state {
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count >= self.success_threshold {
                    *self.state.write().await = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                }
            }
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::SeqCst);
            }
            _ => {}
        }
    }
    
    async fn on_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure_time.write().await = Some(Instant::now());
        
        if count >= self.failure_threshold {
            *self.state.write().await = CircuitState::Open;
            self.success_count.store(0, Ordering::SeqCst);
        }
    }
    
    fn create_circuit_open_error<E>(&self) -> E
    where
        E: std::fmt::Debug,
    {
        panic!("Circuit breaker is open")
    }
    
    pub async fn allow_request(&self) -> bool {
        let current_state = self.state.read().await.clone();
        
        match current_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if self.should_attempt_reset().await {
                    *self.state.write().await = CircuitState::HalfOpen;
                    self.half_open_calls.store(0, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                let calls = self.half_open_calls.load(Ordering::SeqCst);
                calls < self.half_open_max_calls
            }
        }
    }
    
    pub async fn record_success(&self) {
        self.on_success().await;
    }
    
    pub async fn record_failure(&self) {
        self.on_failure().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        let cb = CircuitBreaker::new();
        
        // Should start closed
        assert_eq!(*cb.state.read().await, CircuitState::Closed);
        
        // Simulate failures
        for _ in 0..5 {
            let _ = cb.call(|| Err::<(), &str>("error")).await;
        }
        
        // Should be open after threshold
        assert_eq!(*cb.state.read().await, CircuitState::Open);
    }
}
