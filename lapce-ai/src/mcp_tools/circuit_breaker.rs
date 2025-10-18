use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use anyhow::{Result, bail};
use dashmap::DashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    // Per-service circuit breakers
    circuits: DashMap<String, Arc<Circuit>>,
    
    // Default configuration
    default_config: CircuitConfig,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            circuits: DashMap::new(),
            default_config: CircuitConfig::default(),
        }
    }
    
    pub async fn call<F, T>(&self, service: &str, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        let circuit = self.circuits
            .entry(service.to_string())
            .or_insert_with(|| Arc::new(Circuit::new(self.default_config.clone())))
            .clone();
        
        circuit.call(operation).await
    }
    
    pub fn get_state(&self, service: &str) -> CircuitState {
        self.circuits
            .get(service)
            .map(|c| c.get_state())
            .unwrap_or(CircuitState::Closed)
    }
    
    pub fn reset(&self, service: &str) {
        if let Some(circuit) = self.circuits.get(service) {
            circuit.reset();
        }
    }
    
    pub fn reset_all(&self) {
        for circuit in self.circuits.iter() {
            circuit.value().reset();
        }
    }
}

struct Circuit {
    state: Arc<RwLock<CircuitState>>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    config: CircuitConfig,
}

impl Circuit {
    fn new(config: CircuitConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: Arc::new(RwLock::new(None)),
            config,
        }
    }
    
    async fn call<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        let current_state = self.get_state();
        
        match current_state {
            CircuitState::Open => {
                // Check if we should transition to half-open
                if self.should_attempt_reset().await {
                    self.set_state(CircuitState::HalfOpen).await;
                } else {
                    bail!("Circuit breaker is open for this service");
                }
            }
            _ => {}
        }
        
        // Attempt the operation
        match operation() {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }
    
    async fn on_success(&self) {
        let current_state = self.get_state();
        
        match current_state {
            CircuitState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if success_count >= self.config.success_threshold {
                    self.set_state(CircuitState::Closed).await;
                    self.reset_counts();
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }
    
    async fn on_failure(&self) {
        let current_state = self.get_state();
        
        match current_state {
            CircuitState::Closed => {
                let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if failure_count >= self.config.failure_threshold {
                    self.set_state(CircuitState::Open).await;
                    *self.last_failure_time.write().await = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                // Single failure in half-open state reopens the circuit
                self.set_state(CircuitState::Open).await;
                *self.last_failure_time.write().await = Some(Instant::now());
                self.reset_counts();
            }
            _ => {}
        }
    }
    
    async fn should_attempt_reset(&self) -> bool {
        if let Some(last_failure) = *self.last_failure_time.read().await {
            last_failure.elapsed() >= self.config.timeout_duration
        } else {
            false
        }
    }
    
    fn get_state(&self) -> CircuitState {
        // Use try_read to avoid blocking
        self.state.try_read().map(|s| *s).unwrap_or(CircuitState::Closed)
    }
    
    async fn set_state(&self, new_state: CircuitState) {
        *self.state.write().await = new_state;
    }
    
    fn reset(&self) {
        self.reset_counts();
        // Reset state asynchronously would require async context
        // For now, we'll leave the state as-is
    }
    
    fn reset_counts(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
struct CircuitConfig {
    failure_threshold: u32,
    success_threshold: u32,
    timeout_duration: Duration,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_duration: Duration::from_secs(60),
        }
    }
}

// Advanced circuit breaker with exponential backoff
pub struct AdvancedCircuitBreaker {
    breaker: CircuitBreaker,
    backoff_multiplier: f64,
    max_backoff: Duration,
}

impl AdvancedCircuitBreaker {
    pub fn new() -> Self {
        Self {
            breaker: CircuitBreaker::new(),
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_secs(300),
        }
    }
    
    pub async fn call_with_backoff<F, T>(&self, service: &str, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        let mut backoff = Duration::from_millis(100);
        let mut attempts = 0;
        
        loop {
            match self.breaker.call(service, || operation()).await {
                Ok(result) => return Ok(result),
                Err(e) if attempts < 3 => {
                    tokio::time::sleep(backoff).await;
                    backoff = Duration::from_secs_f64(
                        (backoff.as_secs_f64() * self.backoff_multiplier).min(self.max_backoff.as_secs_f64())
                    );
                    attempts += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
