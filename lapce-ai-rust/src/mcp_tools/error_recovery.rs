/// Error Recovery Mechanism for MCP Tools
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use anyhow::Result;

pub struct ErrorRecoverySystem {
    retry_policies: HashMap<String, RetryPolicy>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
}

#[derive(Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
}

#[derive(Clone)]
struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

#[derive(Clone, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl ErrorRecoverySystem {
    pub fn new() -> Self {
        let mut retry_policies = HashMap::new();
        
        // Default retry policy
        let default_policy = RetryPolicy {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            exponential_base: 2.0,
        };
        
        // Tool-specific policies
        retry_policies.insert("readFile".to_string(), default_policy.clone());
        retry_policies.insert("writeFile".to_string(), RetryPolicy {
            max_retries: 5,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
        });
        
        Self {
            retry_policies,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn execute_with_recovery<F, T>(
        &self,
        tool_name: &str,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync,
    {
        // Check circuit breaker
        if !self.check_circuit_breaker(tool_name).await {
            anyhow::bail!("Circuit breaker open for tool '{}'", tool_name);
        }
        
        let policy = self.retry_policies
            .get(tool_name)
            .cloned()
            .unwrap_or_else(|| RetryPolicy::default());
        
        let mut attempt = 0;
        let mut delay = policy.initial_delay;
        
        loop {
            match operation().await {
                Ok(result) => {
                    self.record_success(tool_name).await;
                    return Ok(result);
                }
                Err(err) if attempt < policy.max_retries => {
                    self.record_failure(tool_name).await;
                    
                    attempt += 1;
                    tokio::time::sleep(delay).await;
                    
                    // Exponential backoff
                    delay = std::cmp::min(
                        Duration::from_secs_f64(
                            delay.as_secs_f64() * policy.exponential_base
                        ),
                        policy.max_delay
                    );
                }
                Err(err) => {
                    self.record_failure(tool_name).await;
                    return Err(err);
                }
            }
        }
    }
    
    async fn check_circuit_breaker(&self, tool_name: &str) -> bool {
        let mut breakers = self.circuit_breakers.write().await;
        let breaker = breakers.entry(tool_name.to_string())
            .or_insert_with(|| CircuitBreaker::new());
        
        match breaker.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = breaker.last_failure {
                    if last_failure.elapsed() > breaker.recovery_timeout {
                        breaker.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
    
    async fn record_success(&self, tool_name: &str) {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(tool_name) {
            breaker.success_count += 1;
            
            if breaker.state == CircuitState::HalfOpen {
                breaker.state = CircuitState::Closed;
                breaker.failure_count = 0;
            }
        }
    }
    
    async fn record_failure(&self, tool_name: &str) {
        let mut breakers = self.circuit_breakers.write().await;
        let breaker = breakers.entry(tool_name.to_string())
            .or_insert_with(|| CircuitBreaker::new());
        
        breaker.failure_count += 1;
        breaker.last_failure = Some(Instant::now());
        
        if breaker.failure_count >= breaker.failure_threshold {
            breaker.state = CircuitState::Open;
        }
    }
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            exponential_base: 2.0,
        }
    }
}
