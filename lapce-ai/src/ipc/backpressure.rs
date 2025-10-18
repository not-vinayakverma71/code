/// Backpressure implementation for shared memory IPC
/// Non-blocking writes with exponential backoff and jitter

use anyhow::{Result, bail};
use rand::Rng;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use std::sync::Arc;
use std::io::{self, ErrorKind};

/// Backpressure configuration
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    /// Initial backoff in microseconds
    pub initial_backoff_us: u64,
    /// Maximum backoff in microseconds
    pub max_backoff_us: u64,
    /// Backoff multiplier
    pub multiplier: f64,
    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Enable non-blocking mode
    pub non_blocking: bool,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            initial_backoff_us: 100,      // 100 microseconds
            max_backoff_us: 100_000,      // 100 milliseconds
            multiplier: 2.0,
            jitter_factor: 0.3,           // ±30% jitter
            max_retries: 10,
            non_blocking: true,
        }
    }
}

/// Backpressure statistics
#[derive(Debug)]
pub struct BackpressureStats {
    pub total_backoffs: u64,
    pub total_wait_time_ms: u64,
    pub available_permits: usize,
    pub dropped_messages: u64,
    pub saturated: bool,
    pub max_wait_time_ms: u64,
}

/// Backpressure manager
#[derive(Debug)]
pub struct BackpressureManager {
    /// Semaphore for limiting concurrent operations
    permits: Arc<Semaphore>,
    
    /// Metrics
    total_backoffs: AtomicU64,
    total_wait_time_ms: AtomicU64,
    dropped_messages: AtomicU64,
    saturated: AtomicBool,
    max_wait_time_ms: AtomicU64,
}

impl BackpressureManager {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(capacity)),
            total_backoffs: AtomicU64::new(0),
            total_wait_time_ms: AtomicU64::new(0),
            dropped_messages: AtomicU64::new(0),
            saturated: AtomicBool::new(false),
            max_wait_time_ms: AtomicU64::new(0),
        }
    }
    
    /// Try to send without blocking - returns WouldBlock if saturated
    pub fn try_send(&self) -> Result<(), io::Error> {
        match self.permits.try_acquire() {
            Ok(_permit) => {
                self.saturated.store(false, Ordering::Relaxed);
                Ok(())
            },
            Err(_) => {
                self.total_backoffs.fetch_add(1, Ordering::Relaxed);
                self.saturated.store(true, Ordering::Relaxed);
                Err(io::Error::new(ErrorKind::WouldBlock, "Backpressure: no permits available"))
            }
        }
    }
    
    /// Bounded blocking send with timeout
    pub async fn send_bounded(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        
        match tokio::time::timeout(timeout, self.permits.acquire()).await {
            Ok(Ok(_permit)) => {
                let wait_ms = start.elapsed().as_millis() as u64;
                self.total_wait_time_ms.fetch_add(wait_ms, Ordering::Relaxed);
                
                // Track max wait time
                let mut current_max = self.max_wait_time_ms.load(Ordering::Relaxed);
                while wait_ms > current_max {
                    match self.max_wait_time_ms.compare_exchange_weak(
                        current_max, wait_ms, Ordering::Relaxed, Ordering::Relaxed
                    ) {
                        Ok(_) => break,
                        Err(x) => current_max = x,
                    }
                }
                
                self.saturated.store(false, Ordering::Relaxed);
                Ok(())
            },
            Ok(Err(e)) => bail!("Failed to acquire permit: {}", e),
            Err(_) => {
                self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                self.saturated.store(true, Ordering::Relaxed);
                bail!("Send timeout after {:?} - system saturated", timeout)
            }
        }
    }
    
    /// Apply exponential backoff with jitter and retry
    pub async fn apply_backpressure_retry<F, T>(
        &self,
        mut operation: F,
        max_retries: u32,
    ) -> Result<T>
    where
        F: FnMut() -> Result<T, io::Error>,
    {
        let mut attempt = 0;
        let mut last_error = None;
        
        while attempt < max_retries {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // Apply exponential backoff
                    let base_delay_ms = 10u64.min(1000u64.saturating_mul(2u64.pow(attempt)));
                    
                    // Add jitter (0-25% of base delay)
                    let jitter = rand::random::<u64>() % (base_delay_ms / 4 + 1);
                    let total_delay_ms = base_delay_ms + jitter;
                    
                    self.total_wait_time_ms.fetch_add(total_delay_ms, Ordering::Relaxed);
                    self.total_backoffs.fetch_add(1, Ordering::Relaxed);
                    
                    tokio::time::sleep(Duration::from_millis(total_delay_ms)).await;
                    
                    attempt += 1;
                    last_error = Some(e);
                }
                Err(e) => return Err(anyhow::anyhow!("Operation failed: {}", e)),
            }
        }
        
        self.dropped_messages.fetch_add(1, Ordering::Relaxed);
        bail!("Operation failed after {} retries: {:?}", max_retries, last_error)
    }
}

/// Backpressure handler
pub struct BackpressureHandler {
    config: BackpressureConfig,
    stats: BackpressureStats,
}

impl BackpressureHandler {
    pub fn new(config: BackpressureConfig) -> Self {
        Self {
            config,
            stats: BackpressureStats::new(),
        }
    }
    
    /// Calculate backoff with jitter
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        let base_backoff = (self.config.initial_backoff_us as f64)
            * self.config.multiplier.powi(attempt as i32);
        let capped_backoff = base_backoff.min(self.config.max_backoff_us as f64);
        
        // Add jitter
        let mut rng = rand::thread_rng();
        let jitter_range = capped_backoff * self.config.jitter_factor;
        let jitter = rng.gen_range(-jitter_range..=jitter_range);
        let final_backoff = (capped_backoff + jitter).max(0.0) as u64;
        
        Duration::from_micros(final_backoff)
    }
    
    /// Execute write with backpressure handling
    pub async fn write_with_backpressure<F, Fut>(
        &self,
        mut write_fn: F,
    ) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<bool>>, // Returns Ok(true) if written, Ok(false) if would block
    {
        let start = Instant::now();
        let mut total_backoff_us = 0u64;
        
        for attempt in 0..=self.config.max_retries {
            match write_fn().await? {
                true => {
                    // Success
                    self.stats.record_write(attempt, total_backoff_us, true);
                    return Ok(());
                }
                false if self.config.non_blocking && attempt == 0 => {
                    // Non-blocking mode, first attempt failed
                    self.stats.record_write(0, 0, false);
                    bail!("Buffer full (non-blocking mode)");
                }
                false => {
                    // Need to retry with backoff
                    if attempt < self.config.max_retries {
                        let backoff = self.calculate_backoff(attempt);
                        total_backoff_us += backoff.as_micros() as u64;
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }
        
        // All retries exhausted
        self.stats.record_write(self.config.max_retries, total_backoff_us, false);
        bail!("Write failed after {} retries ({:.2}ms total)", 
              self.config.max_retries,
              start.elapsed().as_secs_f64() * 1000.0)
    }
    
    pub fn get_stats(&self) -> &BackpressureStats {
        &self.stats
    }
}
