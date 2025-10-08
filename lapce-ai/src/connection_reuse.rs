/// Connection Reuse Tracking and Optimization
/// Monitors and optimizes connection reuse patterns

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};
use bb8::{Pool, PooledConnection};
use tracing::{info, debug};

use crate::https_connection_manager_real::HttpsConnectionManager;
use crate::connection_metrics::ConnectionStats;

/// Connection reuse guard for automatic tracking
pub struct ConnectionReuseGuard<'a> {
    connection: Option<PooledConnection<'a, HttpsConnectionManager>>,
    pool: &'a Pool<HttpsConnectionManager>,
    reuse_count: Arc<AtomicU32>,
    connection_id: u64,
    acquired_at: Instant,
    stats: Arc<ConnectionStats>,
}

impl<'a> ConnectionReuseGuard<'a> {
    pub fn new(
        pool: &'a Pool<HttpsConnectionManager>,
        stats: Arc<ConnectionStats>,
    ) -> Self {
        Self {
            connection: None,
            pool,
            reuse_count: Arc::new(AtomicU32::new(0)),
            connection_id: rand::random(),
            acquired_at: Instant::now(),
            stats,
        }
    }
    
    /// Get or acquire connection
    pub async fn get_connection(&mut self) -> Result<&HttpsConnectionManager> {
        if self.connection.is_none() {
            let start = Instant::now();
            self.connection = Some(self.pool.get().await
                .map_err(|e| anyhow!("Failed to get connection from pool: {:?}", e))?);
            
            let wait_time = start.elapsed();
            self.stats.record_acquisition(wait_time);
            self.reuse_count.fetch_add(1, Ordering::Relaxed);
            
            debug!(
                "Acquired connection {} after {:?}", 
                self.connection_id, wait_time
            );
        } else {
            // Reusing existing connection
            self.stats.record_hit();
            self.reuse_count.fetch_add(1, Ordering::Relaxed);
            debug!("Reusing connection {} (count: {})", 
                   self.connection_id, 
                   self.reuse_count.load(Ordering::Relaxed));
        }
        
        Ok(self.connection.as_ref().unwrap())
    }
    
    /// Execute request with automatic reuse
    pub async fn execute<F, Fut, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&HttpsConnectionManager) -> Fut,
        Fut: std::future::Future<Output = Result<R>>,
    {
        let conn = self.get_connection().await?;
        f(conn).await
    }
    
    /// Get reuse statistics
    pub fn get_reuse_count(&self) -> u32 {
        self.reuse_count.load(Ordering::Relaxed)
    }
    
    /// Get connection age
    pub fn get_age(&self) -> Duration {
        self.acquired_at.elapsed()
    }
}

impl<'a> Drop for ConnectionReuseGuard<'a> {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            let reuse_count = self.reuse_count.load(Ordering::Relaxed);
            let age = self.acquired_at.elapsed();
            
            self.stats.record_return();
            
            debug!(
                "Returning connection {} to pool after {} uses, age: {:?}",
                self.connection_id, reuse_count, age
            );
            
            // Connection automatically returned to pool
        }
    }
}

/// Connection affinity manager for session persistence
pub struct ConnectionAffinityManager {
    affinity_map: Arc<RwLock<HashMap<String, u64>>>,
    connection_map: Arc<RwLock<HashMap<u64, Arc<HttpsConnectionManager>>>>,
}

impl ConnectionAffinityManager {
    pub fn new() -> Self {
        Self {
            affinity_map: Arc::new(RwLock::new(HashMap::new())),
            connection_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get affinity connection for a session
    pub async fn get_affinity_connection<'a>(
        &self, 
        session_id: &str,
        pool: &'a Pool<HttpsConnectionManager>,
    ) -> Result<PooledConnection<'a, HttpsConnectionManager>> {
        let affinity_map = self.affinity_map.read().await;
        
        if let Some(conn_id) = affinity_map.get(session_id) {
            debug!("Found affinity connection {} for session {}", conn_id, session_id);
            // In real implementation, we'd need to track specific connections
        }
        
        // For now, just get from pool
        pool.get().await.map_err(|e| anyhow::anyhow!("Connection pool error"))
    }
    
    /// Set affinity for a session
    pub async fn set_affinity(&self, session_id: String, connection_id: u64) {
        self.affinity_map.write().await.insert(session_id.clone(), connection_id);
        debug!("Set affinity: session {} -> connection {}", session_id, connection_id);
    }
    
    /// Clear affinity for a session
    pub async fn clear_affinity(&self, session_id: &str) {
        self.affinity_map.write().await.remove(session_id);
        debug!("Cleared affinity for session {}", session_id);
    }
    
    /// Get affinity statistics
    pub async fn get_stats(&self) -> AffinityStats {
        let affinity_map = self.affinity_map.read().await;
        let connection_map = self.connection_map.read().await;
        
        AffinityStats {
            total_sessions: affinity_map.len(),
            total_connections: connection_map.len(),
        }
    }
}

/// Affinity statistics
#[derive(Debug, Clone)]
pub struct AffinityStats {
    pub total_sessions: usize,
    pub total_connections: usize,
}

/// Least Recently Used (LRU) connection selector
pub struct LruConnectionSelector {
    usage_map: Arc<RwLock<HashMap<u64, Instant>>>,
}

impl LruConnectionSelector {
    pub fn new() -> Self {
        Self {
            usage_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Record connection usage
    pub async fn record_usage(&self, connection_id: u64) {
        self.usage_map.write().await.insert(connection_id, Instant::now());
        debug!("Recorded usage for connection {}", connection_id);
    }
    
    /// Get least recently used connection
    pub async fn get_lru_connection(&self) -> Option<u64> {
        let usage_map = self.usage_map.read().await;
        
        usage_map.iter()
            .min_by_key(|(_, last_used)| *last_used)
            .map(|(id, _)| *id)
    }
    
    /// Evict stale connections
    pub async fn evict_stale(&self, max_idle: Duration) -> Vec<u64> {
        let mut usage_map = self.usage_map.write().await;
        let now = Instant::now();
        let mut evicted = Vec::new();
        
        usage_map.retain(|id, last_used| {
            if now.duration_since(*last_used) > max_idle {
                evicted.push(*id);
                false
            } else {
                true
            }
        });
        
        if !evicted.is_empty() {
            info!("Evicted {} stale connections", evicted.len());
        }
        
        evicted
    }
}

/// Connection reuse optimizer
pub struct ReuseOptimizer {
    stats: Arc<ConnectionStats>,
    target_reuse_rate: f64,
    min_connections: u32,
    max_connections: u32,
}

impl ReuseOptimizer {
    pub fn new(stats: Arc<ConnectionStats>) -> Self {
        Self {
            stats,
            target_reuse_rate: 95.0,
            min_connections: 10,
            max_connections: 100,
        }
    }
    
    /// Calculate optimal pool size
    pub fn calculate_optimal_size(&self) -> u32 {
        let current_reuse = self.stats.get_reuse_rate();
        let active = self.stats.active_connections.load(Ordering::Relaxed);
        
        if current_reuse < self.target_reuse_rate {
            // Need fewer connections to increase reuse
            let new_size = (active as f64 * 0.8) as u32;
            new_size.max(self.min_connections)
        } else if current_reuse > self.target_reuse_rate + 3.0 {
            // Can afford more connections for better parallelism
            let new_size = (active as f64 * 1.2) as u32;
            new_size.min(self.max_connections)
        } else {
            // Current size is optimal
            active
        }
    }
    
    /// Get optimization recommendations
    pub fn get_recommendations(&self) -> OptimizationRecommendations {
        let reuse_rate = self.stats.get_reuse_rate();
        let avg_wait = self.stats.get_avg_acquisition_latency_ms();
        
        OptimizationRecommendations {
            current_reuse_rate: reuse_rate,
            target_reuse_rate: self.target_reuse_rate,
            optimal_pool_size: self.calculate_optimal_size(),
            action: if reuse_rate < self.target_reuse_rate {
                "Reduce pool size to increase reuse".to_string()
            } else if avg_wait > 1.0 {
                "Increase pool size to reduce wait time".to_string()
            } else {
                "Pool is optimally configured".to_string()
            },
        }
    }
}

/// Optimization recommendations
#[derive(Debug, Clone)]
pub struct OptimizationRecommendations {
    pub current_reuse_rate: f64,
    pub target_reuse_rate: f64,
    pub optimal_pool_size: u32,
    pub action: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection_pool_manager::{ConnectionPoolManager, PoolConfig};
    
    #[tokio::test]
    async fn test_connection_reuse_guard() {
        // Create a pool with the correct type
        use bb8::{Pool, Builder};
        
        let manager = HttpsConnectionManager::new().await.unwrap();
        let pool = Builder::new().build(manager).await.unwrap();
        let stats = Arc::new(ConnectionStats::new());
        
        let mut guard = ConnectionReuseGuard::new(&pool, stats);
        
        // First use - acquisition
        let _ = guard.get_connection().await.unwrap();
        assert_eq!(guard.get_reuse_count(), 1);
        
        // Second use - reuse
        let _ = guard.get_connection().await.unwrap();
        assert_eq!(guard.get_reuse_count(), 2);
    }
    
    #[tokio::test]
    async fn test_lru_selector() {
        let selector = LruConnectionSelector::new();
        
        // Record usage
        selector.record_usage(1).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        selector.record_usage(2).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        selector.record_usage(3).await;
        
        // Get LRU
        let lru = selector.get_lru_connection().await;
        assert_eq!(lru, Some(1));
    }
    
    #[tokio::test]
    async fn test_reuse_optimizer() {
        let stats = Arc::new(ConnectionStats::new());
        
        // Simulate pool hits and misses
        for _ in 0..95 {
            stats.record_hit();
        }
        for _ in 0..5 {
            stats.record_miss();
        }
        
        let optimizer = ReuseOptimizer::new(stats);
        let recommendations = optimizer.get_recommendations();
        
        assert_eq!(recommendations.current_reuse_rate, 95.0);
        assert!(recommendations.action.contains("optimally"));
    }
}
