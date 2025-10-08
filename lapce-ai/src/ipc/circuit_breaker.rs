/// Circuit Breaker Implementation for IPC Server
/// Prevents cascading failures by monitoring error rates
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tracing::{info, warn, error};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    /// Normal operation
    Closed,
    /// Blocking all requests
    Open,
    /// Testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Error threshold to trip the circuit
    pub error_threshold: u32,
    /// Time window for counting errors
    pub window_duration: Duration,
    /// How long to stay open before trying half-open
    pub reset_timeout: Duration,
    /// Success count needed to close from half-open
    pub success_threshold: u32,
    /// Maximum consecutive errors before opening
    pub consecutive_error_limit: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            error_threshold: 50,  // 50% error rate
            window_duration: Duration::from_secs(60),
            reset_timeout: Duration::from_secs(30),
            success_threshold: 5,
            consecutive_error_limit: 10,
        }
    }
}

/// Circuit breaker for connection management
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    config: Arc<CircuitBreakerConfig>,
    
    // Metrics
    total_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    consecutive_errors: Arc<AtomicUsize>,
    consecutive_successes: Arc<AtomicUsize>,
    
    // Timing
    last_state_change: Arc<RwLock<Instant>>,
    window_start: Arc<RwLock<Instant>>,
    
    // Rolling window for error tracking
    error_window: Arc<RwLock<Vec<(Instant, bool)>>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            config: Arc::new(config),
            total_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            consecutive_errors: Arc::new(AtomicUsize::new(0)),
            consecutive_successes: Arc::new(AtomicUsize::new(0)),
            last_state_change: Arc::new(RwLock::new(Instant::now())),
            window_start: Arc::new(RwLock::new(Instant::now())),
            error_window: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Check if requests are allowed
    pub fn is_allowed(&self) -> bool {
        let state = *self.state.read();
        
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                let elapsed = self.last_state_change.read().elapsed();
                if elapsed >= self.config.reset_timeout {
                    self.transition_to_half_open();
                    true  // Allow one request to test
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,  // Allow limited requests
        }
    }
    
    /// Record a successful request
    pub fn record_success(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.consecutive_successes.fetch_add(1, Ordering::Relaxed);
        self.consecutive_errors.store(0, Ordering::Relaxed);
        
        // Update rolling window
        {
            let mut window = self.error_window.write();
            window.push((Instant::now(), false));
            self.clean_window(&mut window);
        }
        
        let state = *self.state.read();
        
        if state == CircuitState::HalfOpen {
            let successes = self.consecutive_successes.load(Ordering::Relaxed);
            if successes >= self.config.success_threshold as usize {
                self.transition_to_closed();
            }
        }
    }
    
    /// Record a failed request
    pub fn record_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.consecutive_errors.fetch_add(1, Ordering::Relaxed);
        self.consecutive_successes.store(0, Ordering::Relaxed);
        
        // Update rolling window
        {
            let mut window = self.error_window.write();
            window.push((Instant::now(), true));
            self.clean_window(&mut window);
        }
        
        let state = *self.state.read();
        let consecutive_errors = self.consecutive_errors.load(Ordering::Relaxed);
        
        match state {
            CircuitState::Closed => {
                // Check if we should open the circuit
                if consecutive_errors >= self.config.consecutive_error_limit as usize 
                    || self.error_rate() > self.config.error_threshold as f64 / 100.0 {
                    self.transition_to_open();
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open goes back to open
                self.transition_to_open();
            }
            _ => {}
        }
    }
    
    /// Calculate current error rate
    fn error_rate(&self) -> f64 {
        let window = self.error_window.read();
        if window.is_empty() {
            return 0.0;
        }
        
        let errors = window.iter().filter(|(_, is_error)| *is_error).count();
        errors as f64 / window.len() as f64
    }
    
    /// Clean old entries from the rolling window
    fn clean_window(&self, window: &mut Vec<(Instant, bool)>) {
        let cutoff = Instant::now() - self.config.window_duration;
        window.retain(|(timestamp, _)| *timestamp > cutoff);
    }
    
    /// Transition to open state
    fn transition_to_open(&self) {
        let mut state = self.state.write();
        *state = CircuitState::Open;
        *self.last_state_change.write() = Instant::now();
        warn!("Circuit breaker opened due to failures");
    }
    
    /// Transition to half-open state
    fn transition_to_half_open(&self) {
        let mut state = self.state.write();
        *state = CircuitState::HalfOpen;
        *self.last_state_change.write() = Instant::now();
        self.consecutive_successes.store(0, Ordering::Relaxed);
        info!("Circuit breaker transitioning to half-open");
    }
    
    /// Transition to closed state
    fn transition_to_closed(&self) {
        let mut state = self.state.write();
        *state = CircuitState::Closed;
        *self.last_state_change.write() = Instant::now();
        self.consecutive_errors.store(0, Ordering::Relaxed);
        info!("Circuit breaker closed, service recovered");
    }
    
    /// Get current state
    pub fn state(&self) -> CircuitState {
        *self.state.read()
    }
    
    /// Get statistics
    pub fn stats(&self) -> CircuitBreakerStats {
        let window = self.error_window.read();
        let errors = window.iter().filter(|(_, is_error)| *is_error).count();
        
        CircuitBreakerStats {
            state: *self.state.read(),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            consecutive_errors: self.consecutive_errors.load(Ordering::Relaxed),
            consecutive_successes: self.consecutive_successes.load(Ordering::Relaxed),
            error_rate: if window.is_empty() { 0.0 } else { errors as f64 / window.len() as f64 },
            window_size: window.len(),
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub consecutive_errors: usize,
    pub consecutive_successes: usize,
    pub error_rate: f64,
    pub window_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_breaker_opens_on_consecutive_errors() {
        let config = CircuitBreakerConfig {
            consecutive_error_limit: 3,
            ..Default::default()
        };
        
        let breaker = CircuitBreaker::new(config);
        
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.is_allowed());
        
        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();
        
        // Should be open now
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.is_allowed());
    }
    
    #[test]
    fn test_circuit_breaker_closes_after_successes() {
        let config = CircuitBreakerConfig {
            consecutive_error_limit: 2,
            success_threshold: 2,
            reset_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        
        let breaker = CircuitBreaker::new(config);
        
        // Open the circuit
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        
        // Wait for reset timeout
        std::thread::sleep(Duration::from_millis(150));
        
        // Should transition to half-open on next check
        assert!(breaker.is_allowed());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
        
        // Record successes
        breaker.record_success();
        breaker.record_success();
        
        // Should be closed now
        assert_eq!(breaker.state(), CircuitState::Closed);
    }
}