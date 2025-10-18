/// Backpressure & Queueing (LSP-030)
/// Bounded channels, circuit breakers, graceful overload handling

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};
use parking_lot::Mutex;
use anyhow::{Result, anyhow};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, reject requests
    HalfOpen,    // Testing recovery
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Success threshold to close circuit (in half-open state)
    pub success_threshold: u32,
    /// Timeout before trying half-open
    pub timeout_secs: u64,
    /// Window size for tracking failures
    pub window_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_secs: 30,
            window_secs: 60,
        }
    }
}

/// Circuit breaker for request handling
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
    last_state_change: Arc<Mutex<Instant>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            last_failure_time: Arc::new(Mutex::new(None)),
            last_state_change: Arc::new(Mutex::new(Instant::now())),
        }
    }
    
    /// Check if request should be allowed
    pub fn allow_request(&self) -> Result<()> {
        let state = *self.state.lock();
        
        match state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // Check if timeout has passed
                let last_change = *self.last_state_change.lock();
                let timeout = Duration::from_secs(self.config.timeout_secs);
                
                if last_change.elapsed() >= timeout {
                    // Try half-open
                    self.transition_to_half_open();
                    Ok(())
                } else {
                    Err(anyhow!("Circuit breaker is open - service overloaded"))
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests in half-open state
                Ok(())
            }
        }
    }
    
    /// Record successful request
    pub fn record_success(&self) {
        let state = *self.state.lock();
        
        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if success_count >= self.config.success_threshold as u64 {
                    self.transition_to_closed();
                }
            }
            CircuitState::Open => {}
        }
        
        tracing::debug!(state = ?state, "Circuit breaker: success recorded");
    }
    
    /// Record failed request
    pub fn record_failure(&self) {
        let state = *self.state.lock();
        
        match state {
            CircuitState::Closed => {
                let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                *self.last_failure_time.lock() = Some(Instant::now());
                
                if failure_count >= self.config.failure_threshold as u64 {
                    self.transition_to_open();
                }
            }
            CircuitState::HalfOpen => {
                // Failure in half-open immediately opens circuit
                self.transition_to_open();
            }
            CircuitState::Open => {}
        }
        
        tracing::warn!(state = ?state, "Circuit breaker: failure recorded");
    }
    
    fn transition_to_open(&self) {
        *self.state.lock() = CircuitState::Open;
        *self.last_state_change.lock() = Instant::now();
        self.success_count.store(0, Ordering::Relaxed);
        
        tracing::error!("Circuit breaker opened - rejecting requests");
        super::LspMetrics::inc_error_count("circuit_breaker", "opened");
    }
    
    fn transition_to_half_open(&self) {
        *self.state.lock() = CircuitState::HalfOpen;
        *self.last_state_change.lock() = Instant::now();
        self.success_count.store(0, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
        
        tracing::info!("Circuit breaker half-open - testing recovery");
    }
    
    fn transition_to_closed(&self) {
        *self.state.lock() = CircuitState::Closed;
        *self.last_state_change.lock() = Instant::now();
        self.success_count.store(0, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
        
        tracing::info!("Circuit breaker closed - normal operation resumed");
    }
    
    /// Get current state
    pub fn state(&self) -> CircuitState {
        *self.state.lock()
    }
}

/// Request queue configuration
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Maximum concurrent requests
    pub max_concurrent: usize,
    /// Request timeout
    pub request_timeout_secs: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_concurrent: 100,
            request_timeout_secs: 30,
        }
    }
}

/// Request with metadata
#[derive(Debug)]
pub struct QueuedRequest<T> {
    pub payload: T,
    pub enqueued_at: Instant,
    pub priority: RequestPriority,
}

/// Request priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

/// Bounded request queue with backpressure
pub struct RequestQueue<T> {
    config: QueueConfig,
    sender: mpsc::Sender<QueuedRequest<T>>,
    receiver: Arc<Mutex<mpsc::Receiver<QueuedRequest<T>>>>,
    semaphore: Arc<Semaphore>,
    queue_size: Arc<AtomicUsize>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl<T> RequestQueue<T> {
    pub fn new(config: QueueConfig, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        let (sender, receiver) = mpsc::channel(config.max_queue_size);
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));
        
        Self {
            config,
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            semaphore,
            queue_size: Arc::new(AtomicUsize::new(0)),
            circuit_breaker,
        }
    }
    
    /// Enqueue request with backpressure
    pub async fn enqueue(&self, payload: T, priority: RequestPriority) -> Result<()> {
        // Check circuit breaker
        self.circuit_breaker.allow_request()?;
        
        let request = QueuedRequest {
            payload,
            enqueued_at: Instant::now(),
            priority,
        };
        
        // Try to send without blocking first
        match self.sender.try_send(request) {
            Ok(_) => {
                self.queue_size.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(request)) => {
                // Queue is full - apply backpressure
                tracing::warn!(
                    queue_size = self.queue_size.load(Ordering::Relaxed),
                    "Request queue full - applying backpressure"
                );
                
                super::LspMetrics::inc_error_count("queue", "full");
                
                // Return ServerBusy error
                Err(anyhow!("Server busy - request queue full"))
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err(anyhow!("Request queue closed"))
            }
        }
    }
    
    /// Dequeue request (for worker)
    pub async fn dequeue(&self) -> Option<QueuedRequest<T>> {
        let mut receiver = self.receiver.lock();
        receiver.recv().await.map(|request| {
            self.queue_size.fetch_sub(1, Ordering::Relaxed);
            request
        })
    }
    
    /// Acquire semaphore permit for concurrent execution
    pub async fn acquire_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        self.semaphore
            .acquire()
            .await
            .map_err(|e| anyhow!("Failed to acquire semaphore permit: {}", e))
    }
    
    /// Get current queue size
    pub fn queue_size(&self) -> usize {
        self.queue_size.load(Ordering::Relaxed)
    }
    
    /// Get queue utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        let size = self.queue_size();
        size as f64 / self.config.max_queue_size as f64
    }
}

impl<T> Clone for RequestQueue<T> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            semaphore: self.semaphore.clone(),
            queue_size: self.queue_size.clone(),
            circuit_breaker: self.circuit_breaker.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_breaker_closed_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        
        assert_eq!(breaker.state(), CircuitState::Closed);
        
        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
        
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        
        // Should reject requests
        assert!(breaker.allow_request().is_err());
    }
    
    #[test]
    fn test_circuit_breaker_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout_secs: 0, // Immediate transition to half-open
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);
        
        // Open circuit
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        
        // Wait for timeout (simulated by immediate timeout)
        std::thread::sleep(Duration::from_millis(10));
        
        // Should allow request (transitions to half-open)
        assert!(breaker.allow_request().is_ok());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
        
        // Record successes
        breaker.record_success();
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }
    
    #[tokio::test]
    async fn test_request_queue_enqueue_dequeue() {
        let config = QueueConfig {
            max_queue_size: 10,
            max_concurrent: 5,
            ..Default::default()
        };
        let breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
        let queue = RequestQueue::new(config, breaker);
        
        // Enqueue requests
        queue.enqueue(1, RequestPriority::Normal).await.unwrap();
        queue.enqueue(2, RequestPriority::High).await.unwrap();
        
        assert_eq!(queue.queue_size(), 2);
        
        // Dequeue requests
        let req1 = queue.dequeue().await.unwrap();
        assert_eq!(req1.payload, 1);
        
        let req2 = queue.dequeue().await.unwrap();
        assert_eq!(req2.payload, 2);
        
        assert_eq!(queue.queue_size(), 0);
    }
    
    #[tokio::test]
    async fn test_request_queue_full() {
        let config = QueueConfig {
            max_queue_size: 3,
            max_concurrent: 1,
            ..Default::default()
        };
        let breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
        let queue = RequestQueue::new(config, breaker);
        
        // Fill queue
        queue.enqueue(1, RequestPriority::Normal).await.unwrap();
        queue.enqueue(2, RequestPriority::Normal).await.unwrap();
        queue.enqueue(3, RequestPriority::Normal).await.unwrap();
        
        // Next enqueue should fail
        let result = queue.enqueue(4, RequestPriority::Normal).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("busy"));
    }
    
    #[tokio::test]
    async fn test_semaphore_concurrency() {
        let config = QueueConfig {
            max_queue_size: 10,
            max_concurrent: 2,
            ..Default::default()
        };
        let breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
        let queue = RequestQueue::new(config, breaker);
        
        // Acquire permits
        let permit1 = queue.acquire_permit().await.unwrap();
        let permit2 = queue.acquire_permit().await.unwrap();
        
        // Third should block (we'll test with try)
        let queue_clone = queue.clone();
        let handle = tokio::spawn(async move {
            queue_clone.acquire_permit().await
        });
        
        // Give it time to try
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Drop one permit
        drop(permit1);
        
        // Now third should succeed
        let permit3 = handle.await.unwrap();
        assert!(permit3.is_ok());
        
        drop(permit2);
        drop(permit3);
    }
}
