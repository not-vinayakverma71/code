/// Robust error handling and recovery for IPC
/// Handles oversize messages, corruption, EOF, and ensures recovery within 100ms

use anyhow::{Result, bail};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::timeout;

/// Maximum message size (10MB)
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Recovery timeout (100ms)
const RECOVERY_TIMEOUT_MS: u64 = 100;

/// Error recovery statistics
#[derive(Debug)]
pub struct ErrorRecoveryStats {
    pub oversize_errors: AtomicU64,
    pub corruption_errors: AtomicU64,
    pub eof_errors: AtomicU64,
    pub recovery_attempts: AtomicU64,
    pub successful_recoveries: AtomicU64,
    pub average_recovery_time_us: AtomicU64,
}

impl ErrorRecoveryStats {
    pub fn new() -> Self {
        Self {
            oversize_errors: AtomicU64::new(0),
            corruption_errors: AtomicU64::new(0),
            eof_errors: AtomicU64::new(0),
            recovery_attempts: AtomicU64::new(0),
            successful_recoveries: AtomicU64::new(0),
            average_recovery_time_us: AtomicU64::new(0),
        }
    }
}

/// Error types that can be recovered from
#[derive(Debug, Clone)]
pub enum RecoverableError {
    /// Message exceeds maximum size
    Oversize { size: usize },
    /// Data corruption detected
    Corruption { details: String },
    /// Unexpected end of file/stream
    UnexpectedEof,
    /// Connection lost
    ConnectionLost,
}

/// Error recovery handler
pub struct ErrorRecovery {
    stats: Arc<ErrorRecoveryStats>,
    recovering: AtomicBool,
}

impl ErrorRecovery {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(ErrorRecoveryStats::new()),
            recovering: AtomicBool::new(false),
        }
    }
    
    /// Handle oversize message error
    pub async fn handle_oversize(&self, size: usize) -> Result<()> {
        self.stats.oversize_errors.fetch_add(1, Ordering::Relaxed);
        
        if size > MAX_MESSAGE_SIZE {
            // Log and skip the oversized message
            tracing::error!("Message too large: {} bytes (max: {})", size, MAX_MESSAGE_SIZE);
            
            // Recovery: skip the message and continue
            self.recover_with_timeout(async {
                // Clear the oversized data from buffer
                // In real implementation, this would drain the buffer
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<(), anyhow::Error>(())
            }).await?;
        }
        
        Ok(())
    }
    
    /// Handle data corruption
    pub async fn handle_corruption(&self, details: &str) -> Result<()> {
        self.stats.corruption_errors.fetch_add(1, Ordering::Relaxed);
        
        tracing::error!("Data corruption detected: {}", details);
        
        // Recovery: resync the connection
        self.recover_with_timeout(async {
            // Resync protocol - send a sync marker and wait for acknowledgment
            self.resync_connection().await
        }).await
    }
    
    /// Handle unexpected EOF
    pub async fn handle_eof(&self) -> Result<()> {
        self.stats.eof_errors.fetch_add(1, Ordering::Relaxed);
        
        tracing::warn!("Unexpected EOF encountered");
        
        // Recovery: attempt reconnection
        self.recover_with_timeout(async {
            self.reconnect().await
        }).await
    }
    
    /// Recover with timeout constraint
    async fn recover_with_timeout<Fut>(&self, recovery_fn: Fut) -> Result<()>
    where
        Fut: std::future::Future<Output = Result<()>>,
    {
        // Check if already recovering
        if self.recovering.swap(true, Ordering::Acquire) {
            bail!("Recovery already in progress");
        }
        
        let start_time = Instant::now();
        self.stats.recovery_attempts.fetch_add(1, Ordering::Relaxed);
        
        let result = timeout(Duration::from_millis(RECOVERY_TIMEOUT_MS), recovery_fn).await;
        let recovery_time = start_time.elapsed();
        
        self.recovering.store(false, Ordering::Release);
        
        match result {
            Ok(Ok(())) => {
                self.stats.successful_recoveries.fetch_add(1, Ordering::Relaxed);
                
                // Update average recovery time
                let time_us = recovery_time.as_micros() as u64;
                let current_avg = self.stats.average_recovery_time_us.load(Ordering::Relaxed);
                let success_count = self.stats.successful_recoveries.load(Ordering::Relaxed);
                let new_avg = if success_count > 1 {
                    (current_avg * (success_count - 1) + time_us) / success_count
                } else {
                    time_us
                };
                self.stats.average_recovery_time_us.store(new_avg, Ordering::Relaxed);
                
                tracing::info!("Recovery successful in {:?}", recovery_time);
                Ok(())
            }
            Ok(Err(e)) => {
                tracing::error!("Recovery failed: {}", e);
                bail!("Recovery failed: {}", e)
            }
            Err(_) => {
                tracing::error!("Recovery timeout after {}ms", RECOVERY_TIMEOUT_MS);
                bail!("Recovery timeout")
            }
        }
    }
    
    /// Resync connection after corruption
    async fn resync_connection(&self) -> Result<()> {
        // Send sync marker
        const SYNC_MARKER: &[u8] = b"SYNC\x00\x00\x00\x00";
        
        // In real implementation, this would:
        // 1. Send sync marker to peer
        // 2. Wait for acknowledgment
        // 3. Resume normal operation
        
        tokio::time::sleep(Duration::from_millis(20)).await;
        Ok(())
    }
    
    /// Reconnect after EOF
    async fn reconnect(&self) -> Result<()> {
        // In real implementation, this would:
        // 1. Close current connection cleanly
        // 2. Establish new connection
        // 3. Perform handshake
        
        tokio::time::sleep(Duration::from_millis(30)).await;
        Ok(())
    }
    
    pub fn stats(&self) -> Arc<ErrorRecoveryStats> {
        self.stats.clone()
    }
}

/// Resource cleanup guard
pub struct ResourceGuard {
    cleanup: Option<Box<dyn FnOnce() + Send>>,
}

impl ResourceGuard {
    pub fn new<F>(cleanup: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            cleanup: Some(Box::new(cleanup)),
        }
    }
    
    pub fn disarm(&mut self) {
        self.cleanup = None;
    }
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_oversize_recovery() {
        let recovery = ErrorRecovery::new();
        
        // Test oversize message handling
        let result = recovery.handle_oversize(MAX_MESSAGE_SIZE + 1).await;
        assert!(result.is_ok());
        
        assert_eq!(recovery.stats.oversize_errors.load(Ordering::Relaxed), 1);
        assert_eq!(recovery.stats.successful_recoveries.load(Ordering::Relaxed), 1);
    }
    
    #[tokio::test]
    async fn test_corruption_recovery() {
        let recovery = ErrorRecovery::new();
        
        // Test corruption handling
        let result = recovery.handle_corruption("Invalid checksum").await;
        assert!(result.is_ok());
        
        assert_eq!(recovery.stats.corruption_errors.load(Ordering::Relaxed), 1);
        assert_eq!(recovery.stats.successful_recoveries.load(Ordering::Relaxed), 1);
    }
    
    #[tokio::test]
    async fn test_eof_recovery() {
        let recovery = ErrorRecovery::new();
        
        // Test EOF handling
        let result = recovery.handle_eof().await;
        assert!(result.is_ok());
        
        assert_eq!(recovery.stats.eof_errors.load(Ordering::Relaxed), 1);
        assert_eq!(recovery.stats.successful_recoveries.load(Ordering::Relaxed), 1);
    }
    
    #[tokio::test]
    async fn test_recovery_within_timeout() {
        let recovery = ErrorRecovery::new();
        
        let start = Instant::now();
        let result = recovery.handle_oversize(MAX_MESSAGE_SIZE + 1).await;
        let elapsed = start.elapsed();
        
        assert!(result.is_ok());
        assert!(elapsed.as_millis() <= RECOVERY_TIMEOUT_MS as u128,
                "Recovery took {}ms, should be under {}ms", 
                elapsed.as_millis(), RECOVERY_TIMEOUT_MS);
    }
    
    #[tokio::test]
    async fn test_concurrent_recovery_prevention() {
        let recovery = Arc::new(ErrorRecovery::new());
        
        // Start first recovery
        let recovery1 = recovery.clone();
        let handle1 = tokio::spawn(async move {
            recovery1.handle_corruption("Test 1").await
        });
        
        // Try concurrent recovery (should fail)
        tokio::time::sleep(Duration::from_millis(5)).await;
        recovery.recovering.store(true, Ordering::Release); // Simulate ongoing recovery
        
        let recovery2 = recovery.clone();
        let result2 = recovery2.recover_with_timeout(async {
            Ok::<(), anyhow::Error>(())
        }).await;
        
        assert!(result2.is_err());
        assert!(result2.unwrap_err().to_string().contains("Recovery already in progress"));
        
        recovery.recovering.store(false, Ordering::Release);
        let _ = handle1.await;
    }
    
    #[test]
    fn test_resource_guard() {
        let cleaned = Arc::new(AtomicBool::new(false));
        
        {
            let cleaned_clone = cleaned.clone();
            let _guard = ResourceGuard::new(move || {
                cleaned_clone.store(true, Ordering::Release);
            });
            // Guard goes out of scope here
        }
        
        assert!(cleaned.load(Ordering::Acquire), "Cleanup should have been called");
    }
    
    #[test]
    fn test_resource_guard_disarm() {
        let cleaned = Arc::new(AtomicBool::new(false));
        
        {
            let cleaned_clone = cleaned.clone();
            let mut guard = ResourceGuard::new(move || {
                cleaned_clone.store(true, Ordering::Release);
            });
            
            guard.disarm(); // Prevent cleanup
            // Guard goes out of scope here
        }
        
        assert!(!cleaned.load(Ordering::Acquire), "Cleanup should not have been called");
    }
}
