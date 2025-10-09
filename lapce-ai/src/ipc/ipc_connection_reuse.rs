/// IPC Connection Reuse Guard
/// Tracks connection reuse metrics and lifecycle similar to HTTP connection reuse

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Statistics for connection reuse
#[derive(Debug, Clone)]
pub struct ConnectionReuseStats {
    pub reuse_count: u64,
    pub total_messages: u64,
    pub total_bytes: u64,
    pub connection_age_secs: u64,
    pub last_used: Instant,
}

/// Guard for tracking IPC connection reuse
pub struct IpcConnectionReuseGuard {
    connection_id: u64,
    created_at: Instant,
    last_used: Arc<parking_lot::Mutex<Instant>>,
    reuse_count: Arc<AtomicU64>,
    total_messages: Arc<AtomicU64>,
    total_bytes: Arc<AtomicU64>,
    is_active: Arc<AtomicBool>,
    max_reuse: u64,
    max_age: Duration,
}

impl IpcConnectionReuseGuard {
    pub fn new(connection_id: u64, max_reuse: u64, max_age: Duration) -> Self {
        Self {
            connection_id,
            created_at: Instant::now(),
            last_used: Arc::new(parking_lot::Mutex::new(Instant::now())),
            reuse_count: Arc::new(AtomicU64::new(0)),
            total_messages: Arc::new(AtomicU64::new(0)),
            total_bytes: Arc::new(AtomicU64::new(0)),
            is_active: Arc::new(AtomicBool::new(true)),
            max_reuse,
            max_age,
        }
    }
    
    /// Mark connection as used
    pub fn touch(&self) {
        *self.last_used.lock() = Instant::now();
        self.reuse_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record a message sent/received
    pub fn record_message(&self, bytes: usize) {
        self.total_messages.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
        self.touch();
    }
    
    /// Check if connection can be reused
    pub fn can_reuse(&self) -> bool {
        if !self.is_active.load(Ordering::Acquire) {
            return false;
        }
        
        let reuse_count = self.reuse_count.load(Ordering::Relaxed);
        let age = self.created_at.elapsed();
        
        reuse_count < self.max_reuse && age < self.max_age
    }
    
    /// Mark connection as closed
    pub fn close(&self) {
        self.is_active.store(false, Ordering::Release);
    }
    
    /// Get connection statistics
    pub fn stats(&self) -> ConnectionReuseStats {
        ConnectionReuseStats {
            reuse_count: self.reuse_count.load(Ordering::Relaxed),
            total_messages: self.total_messages.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            connection_age_secs: self.created_at.elapsed().as_secs(),
            last_used: *self.last_used.lock(),
        }
    }
    
    /// Get connection ID
    pub fn connection_id(&self) -> u64 {
        self.connection_id
    }
    
    /// Check if connection is idle
    pub fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_used.lock().elapsed() > idle_timeout
    }
}

/// Manager for connection reuse guards
pub struct ConnectionReuseManager {
    connections: Arc<dashmap::DashMap<u64, Arc<IpcConnectionReuseGuard>>>,
    default_max_reuse: u64,
    default_max_age: Duration,
    idle_timeout: Duration,
}

impl ConnectionReuseManager {
    pub fn new(max_reuse: u64, max_age: Duration, idle_timeout: Duration) -> Self {
        Self {
            connections: Arc::new(dashmap::DashMap::new()),
            default_max_reuse: max_reuse,
            default_max_age: max_age,
            idle_timeout,
        }
    }
    
    /// Create a new reuse guard for a connection
    pub fn create_guard(&self, connection_id: u64) -> Arc<IpcConnectionReuseGuard> {
        let guard = Arc::new(IpcConnectionReuseGuard::new(
            connection_id,
            self.default_max_reuse,
            self.default_max_age,
        ));
        
        self.connections.insert(connection_id, guard.clone());
        guard
    }
    
    /// Get existing guard for a connection
    pub fn get_guard(&self, connection_id: u64) -> Option<Arc<IpcConnectionReuseGuard>> {
        self.connections.get(&connection_id).map(|g| g.clone())
    }
    
    /// Remove guard for a connection
    pub fn remove_guard(&self, connection_id: u64) {
        if let Some((_, guard)) = self.connections.remove(&connection_id) {
            guard.close();
        }
    }
    
    /// Clean up idle connections
    pub fn cleanup_idle(&self) -> usize {
        let mut removed = 0;
        
        self.connections.retain(|_, guard| {
            if guard.is_idle(self.idle_timeout) || !guard.can_reuse() {
                guard.close();
                removed += 1;
                false
            } else {
                true
            }
        });
        
        removed
    }
    
    /// Get statistics for all connections
    pub fn all_stats(&self) -> Vec<(u64, ConnectionReuseStats)> {
        self.connections
            .iter()
            .map(|entry| (*entry.key(), entry.value().stats()))
            .collect()
    }
    
    /// Get total reuse count across all connections
    pub fn total_reuse_count(&self) -> u64 {
        self.connections
            .iter()
            .map(|entry| entry.value().stats().reuse_count)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connection_reuse_guard() {
        let guard = IpcConnectionReuseGuard::new(
            1,
            100,
            Duration::from_secs(60)
        );
        
        assert!(guard.can_reuse());
        
        // Use the connection
        guard.record_message(1024);
        guard.record_message(2048);
        
        let stats = guard.stats();
        assert_eq!(stats.total_messages, 2);
        assert_eq!(stats.total_bytes, 3072);
        assert_eq!(stats.reuse_count, 2);
        
        // Close the connection
        guard.close();
        assert!(!guard.can_reuse());
    }
    
    #[test]
    fn test_max_reuse_limit() {
        let guard = IpcConnectionReuseGuard::new(
            1,
            3,  // Max 3 reuses
            Duration::from_secs(60)
        );
        
        // Use up the reuse limit
        for _ in 0..3 {
            guard.touch();
            assert!(guard.can_reuse() || guard.reuse_count.load(Ordering::Relaxed) == 3);
        }
        
        // Should not be reusable after limit
        assert!(!guard.can_reuse());
    }
    
    #[tokio::test]
    async fn test_connection_manager() {
        let manager = ConnectionReuseManager::new(
            100,
            Duration::from_secs(60),
            Duration::from_secs(10)
        );
        
        // Create guards
        let guard1 = manager.create_guard(1);
        let guard2 = manager.create_guard(2);
        
        guard1.record_message(1024);
        guard2.record_message(2048);
        
        assert_eq!(manager.connections.len(), 2);
        
        // Remove a guard
        manager.remove_guard(1);
        assert_eq!(manager.connections.len(), 1);
        
        // Check total reuse
        let total = manager.total_reuse_count();
        assert_eq!(total, 1);  // Only guard2's reuse counts
    }
}
