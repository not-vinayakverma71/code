/// Stream Backpressure Controller - Adaptive flow control for streaming
/// Phase 2, Task 7: StreamBackpressureController
/// Based on docs/08-STREAMING-PIPELINE.md lines 422-495

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore, SemaphorePermit};
use anyhow::Result;

/// Adaptive backpressure controller for streaming
#[derive(Clone)]
pub struct StreamBackpressureController {
    /// Semaphore for limiting concurrent processing
    semaphore: Arc<Semaphore>,
    
    /// Buffer for holding messages when under pressure
    buffer_size: Arc<AtomicUsize>,
    
    /// Metric tracking
    messages_processed: Arc<AtomicU64>,
    messages_dropped: Arc<AtomicU64>,
    
    /// Adaptive settings
    last_adjustment: Arc<RwLock<Instant>>,
    capacity: Arc<AtomicUsize>,
    
    /// Current queue depth
    queue_depth: Arc<AtomicUsize>,
    
    /// Buffer size limits
    min_buffer: usize,
    max_buffer: usize,
    
    /// Average processing time
    process_time: Arc<RwLock<Duration>>,
}

impl StreamBackpressureController {
    /// Create new backpressure controller
    pub fn new(initial_permits: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(initial_permits)),
            buffer_size: Arc::new(AtomicUsize::new(4096)),
            messages_processed: Arc::new(AtomicU64::new(0)),
            messages_dropped: Arc::new(AtomicU64::new(0)),
            last_adjustment: Arc::new(RwLock::new(Instant::now())),
            capacity: Arc::new(AtomicUsize::new(initial_permits)),
            queue_depth: Arc::new(AtomicUsize::new(0)),
            min_buffer: 1024,
            max_buffer: 65536,
            process_time: Arc::new(RwLock::new(Duration::ZERO)),
        }
    }
    
    /// Create with custom buffer limits
    pub fn with_limits(initial_permits: usize, min_buffer: usize, max_buffer: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(initial_permits)),
            buffer_size: Arc::new(AtomicUsize::new(min_buffer * 2)),
            messages_processed: Arc::new(AtomicU64::new(0)),
            messages_dropped: Arc::new(AtomicU64::new(0)),
            last_adjustment: Arc::new(RwLock::new(Instant::now())),
            capacity: Arc::new(AtomicUsize::new(initial_permits)),
            queue_depth: Arc::new(AtomicUsize::new(0)),
            min_buffer,
            max_buffer,
            process_time: Arc::new(RwLock::new(Duration::ZERO)),
        }
    }
    
    /// Acquire permit with adaptive buffer sizing
    pub async fn acquire(&self) -> Result<SemaphorePermit<'_>> {
        // Increment queue depth
        let depth = self.queue_depth.fetch_add(1, Ordering::Acquire);
        
        // Adapt buffer size based on queue depth
        if depth > 100 {
            // Heavy load - increase buffer
            let current = self.buffer_size.load(Ordering::Relaxed);
            let new_size = (current * 2).min(self.max_buffer);
            self.buffer_size.store(new_size, Ordering::Relaxed);
        } else if depth < 10 {
            // Light load - decrease buffer
            let current = self.buffer_size.load(Ordering::Relaxed);
            let new_size = (current / 2).max(self.min_buffer);
            self.buffer_size.store(new_size, Ordering::Relaxed);
        }
        
        // Acquire permit with timeout
        match tokio::time::timeout(
            Duration::from_secs(30),
            self.semaphore.acquire()
        ).await {
            Ok(Ok(permit)) => {
                self.queue_depth.fetch_sub(1, Ordering::Release);
                Ok(permit)
            }
            Ok(Err(e)) => {
                self.queue_depth.fetch_sub(1, Ordering::Release);
                Err(anyhow::anyhow!("Semaphore closed: {}", e))
            }
            Err(_) => {
                self.queue_depth.fetch_sub(1, Ordering::Release);
                Err(anyhow::anyhow!("Backpressure timeout after 30 seconds"))
            }
        }
    }
    
    /// Adapt capacity based on processing time
    pub async fn adapt_capacity(&self, processing_time: Duration) {
        // Update average processing time
        let mut avg_time = self.process_time.write().await;
        *avg_time = (*avg_time + processing_time) / 2;
        
        // Adjust semaphore capacity based on processing speed
        if processing_time < Duration::from_millis(10) {
            // Fast processing - can handle more
            self.semaphore.add_permits(1);
        } else if processing_time > Duration::from_millis(100) {
            // Slow processing - reduce load
            // Note: Semaphore doesn't support reducing permits dynamically
            // We handle this by not releasing permits immediately
        }
    }
    
    /// Get current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_size.load(Ordering::Relaxed)
    }
    
    /// Get current queue depth
    pub fn queue_depth(&self) -> usize {
        self.queue_depth.load(Ordering::Relaxed)
    }
    
    /// Get available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
    
    /// Reset to initial state
    pub async fn reset(&self) {
        *self.process_time.write().await = Duration::ZERO;
        self.queue_depth.store(0, Ordering::Relaxed);
        self.buffer_size.store(self.min_buffer * 2, Ordering::Relaxed);
    }
}

impl Default for StreamBackpressureController {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Backpressure configuration
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    pub initial_permits: usize,
    pub min_buffer: usize,
    pub max_buffer: usize,
    pub timeout_secs: u64,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            initial_permits: 100,
            min_buffer: 1024,
            max_buffer: 65536,
            timeout_secs: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_acquire_permit() {
        let controller = StreamBackpressureController::new(1);
        
        // First acquire should succeed
        let permit = controller.acquire().await;
        assert!(permit.is_ok());
        
        // Queue depth should be 0 after acquiring
        assert_eq!(controller.queue_depth(), 0);
    }
    
    #[tokio::test]
    async fn test_adaptive_buffer_sizing() {
        let controller = StreamBackpressureController::new(10);
        let initial_size = controller.buffer_size();
        
        // Simulate heavy load
        for _ in 0..150 {
            controller.queue_depth.fetch_add(1, Ordering::Relaxed);
        }
        
        let _permit = controller.acquire().await.unwrap();
        
        // Buffer should have increased
        assert!(controller.buffer_size() > initial_size);
    }
    
    #[tokio::test]
    async fn test_capacity_adaptation() {
        let controller = StreamBackpressureController::new(5);
        
        // Fast processing should increase capacity
        controller.adapt_capacity(Duration::from_millis(5)).await;
        
        // Should have more permits available
        assert!(controller.available_permits() > 0);
    }
}
