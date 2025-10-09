/// Complete Connection Pool Implementation with lifecycle management
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{RwLock, Semaphore};
use anyhow::Result;
use uuid::Uuid;
use tracing::{info, warn, debug};

use crate::ipc::shared_memory_complete::SharedMemoryStream;

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: Uuid,
    pub created_at: Instant,
    pub last_active: Instant,
    pub request_count: u64,
    pub error_count: u64,
    pub is_healthy: bool,
}

pub struct ConnectionPool {
    connections: Arc<RwLock<HashMap<Uuid, ConnectionInfo>>>,
    max_connections: usize,
    idle_timeout: Duration,
    semaphore: Arc<Semaphore>,
    cleanup_interval: Duration,
}

impl ConnectionPool {
    pub fn new(max_connections: usize, idle_timeout: Duration) -> Self {
        let pool = Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
            idle_timeout,
            semaphore: Arc::new(Semaphore::new(max_connections)),
            cleanup_interval: Duration::from_secs(30),
        };
        
        // Start cleanup task
        pool.start_cleanup_task();
        pool
    }
    
    /// Acquire a connection permit
    pub async fn acquire(&self) -> ConnectionGuard {
        let permit = self.semaphore.clone().acquire_owned().await
            .expect("Failed to acquire connection permit");
        
        let id = Uuid::new_v4();
        let info = ConnectionInfo {
            id,
            created_at: Instant::now(),
            last_active: Instant::now(),
            request_count: 0,
            error_count: 0,
            is_healthy: true,
        };
        
        self.connections.write().await.insert(id, info.clone());
        
        info!("Connection {} acquired, total: {}", id, self.active_count().await);
        
        ConnectionGuard {
            id,
            pool: self.clone(),
            _permit: permit,
        }
    }
    
    /// Register a new connection
    pub async fn register(&self, stream: SharedMemoryStream) -> Uuid {
        let id = Uuid::new_v4();
        let info = ConnectionInfo {
            id,
            created_at: Instant::now(),
            last_active: Instant::now(),
            request_count: 0,
            error_count: 0,
            is_healthy: true,
        };
        
        self.connections.write().await.insert(id, info);
        debug!("Registered connection {}", id);
        id
    }
    
    /// Update connection activity
    pub async fn touch(&self, id: Uuid) {
        if let Some(info) = self.connections.write().await.get_mut(&id) {
            info.last_active = Instant::now();
            info.request_count += 1;
        }
    }
    
    /// Record connection error
    pub async fn record_error(&self, id: Uuid) {
        if let Some(info) = self.connections.write().await.get_mut(&id) {
            info.error_count += 1;
            if info.error_count > 5 {
                info.is_healthy = false;
                warn!("Connection {} marked unhealthy after {} errors", id, info.error_count);
            }
        }
    }
    
    /// Remove a connection
    pub async fn remove(&self, id: Uuid) {
        self.connections.write().await.remove(&id);
        info!("Connection {} removed, remaining: {}", id, self.active_count().await);
    }
    
    /// Get active connection count
    pub async fn active_count(&self) -> usize {
        self.connections.read().await.len()
    }
    
    /// Get connection health status
    pub async fn health_status(&self) -> Vec<ConnectionInfo> {
        self.connections.read().await
            .values()
            .cloned()
            .collect()
    }
    
    /// Check if pool is at capacity
    pub async fn is_full(&self) -> bool {
        self.connections.read().await.len() >= self.max_connections
    }
    
    /// Cleanup idle and unhealthy connections
    async fn cleanup(&self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();
        
        {
            let connections = self.connections.read().await;
            for (id, info) in connections.iter() {
                // Remove if idle too long
                if now.duration_since(info.last_active) > self.idle_timeout {
                    to_remove.push(*id);
                    debug!("Marking connection {} for removal (idle)", id);
                }
                // Remove if unhealthy
                else if !info.is_healthy {
                    to_remove.push(*id);
                    debug!("Marking connection {} for removal (unhealthy)", id);
                }
            }
        }
        
        // Remove marked connections
        if !to_remove.is_empty() {
            let mut connections = self.connections.write().await;
            for id in to_remove {
                connections.remove(&id);
                info!("Cleaned up connection {}", id);
            }
        }
    }
    
    /// Start background cleanup task
    fn start_cleanup_task(&self) {
        let connections = self.connections.clone();
        let idle_timeout = self.idle_timeout;
        let cleanup_interval = self.cleanup_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                
                let now = Instant::now();
                let mut to_remove = Vec::new();
                
                {
                    let conns = connections.read().await;
                    for (id, info) in conns.iter() {
                        if now.duration_since(info.last_active) > idle_timeout || !info.is_healthy {
                            to_remove.push(*id);
                        }
                    }
                }
                
                if !to_remove.is_empty() {
                    let mut conns = connections.write().await;
                    for id in to_remove {
                        conns.remove(&id);
                        debug!("Cleaned up connection {} in background task", id);
                    }
                }
            }
        });
    }
    
    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let connections = self.connections.read().await;
        
        let total = connections.len();
        let healthy = connections.values().filter(|c| c.is_healthy).count();
        let unhealthy = total - healthy;
        
        let total_requests: u64 = connections.values().map(|c| c.request_count).sum();
        let total_errors: u64 = connections.values().map(|c| c.error_count).sum();
        
        let oldest = connections.values()
            .map(|c| c.created_at)
            .min()
            .map(|t| Instant::now().duration_since(t));
        
        let reuse_rate = if total_requests > 0 {
            (total_requests as f64 - total as f64) / total_requests as f64 * 100.0
        } else {
            0.0
        };
        
        PoolStats {
            total_connections: total,
            healthy_connections: healthy,
            unhealthy_connections: unhealthy,
            total_requests,
            total_errors,
            oldest_connection: oldest,
            max_connections: self.max_connections,
            pool_size: total,
            total_created: total as u64,
            total_recycled: if total_requests > total as u64 { total_requests - total as u64 } else { 0 },
            avg_acquisition_time_ms: 0.5, // Placeholder for real measurement
            reuse_rate,
        }
    }
    
    /// Perform health check on a connection
    pub async fn health_check(&self, id: Uuid) -> bool {
        if let Some(info) = self.connections.read().await.get(&id) {
            // Check if connection is healthy based on error rate and age
            let error_rate = if info.request_count > 0 {
                info.error_count as f64 / info.request_count as f64
            } else {
                0.0
            };
            
            info.is_healthy && error_rate < 0.1 // Less than 10% error rate
        } else {
            false
        }
    }
    
    /// Adaptive scaling - adjust pool size based on load
    pub async fn scale(&self, target_size: usize) -> Result<()> {
        let current = self.active_count().await;
        
        if target_size < current {
            // Scale down - remove excess connections
            let to_remove = current - target_size;
            let mut removed = 0;
            
            let mut connections = self.connections.write().await;
            let idle_conns: Vec<Uuid> = connections
                .iter()
                .filter(|(_, info)| {
                    Instant::now().duration_since(info.last_active) > Duration::from_secs(60)
                })
                .take(to_remove)
                .map(|(id, _)| *id)
                .collect();
            
            for id in idle_conns {
                connections.remove(&id);
                removed += 1;
            }
            
            info!("Scaled down pool by {} connections", removed);
        }
        
        Ok(())
    }
    
    /// Export metrics in Prometheus format
    pub async fn export_metrics(&self) -> String {
        let stats = self.stats().await;
        
        format!(
            "# HELP connection_pool_total Total connections in pool\n\
             # TYPE connection_pool_total gauge\n\
             connection_pool_total {}\n\
             # HELP connection_pool_healthy Healthy connections\n\
             # TYPE connection_pool_healthy gauge\n\
             connection_pool_healthy {}\n\
             # HELP connection_pool_requests Total requests processed\n\
             # TYPE connection_pool_requests counter\n\
             connection_pool_requests {}\n\
             # HELP connection_pool_errors Total errors encountered\n\
             # TYPE connection_pool_errors counter\n\
             connection_pool_errors {}\n\
             # HELP connection_pool_reuse_rate Connection reuse rate percentage\n\
             # TYPE connection_pool_reuse_rate gauge\n\
             connection_pool_reuse_rate {:.2}\n",
            stats.total_connections,
            stats.healthy_connections,
            stats.total_requests,
            stats.total_errors,
            stats.reuse_rate
        )
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            max_connections: self.max_connections,
            idle_timeout: self.idle_timeout,
            semaphore: self.semaphore.clone(),
            cleanup_interval: self.cleanup_interval,
        }
    }
}

/// Guard for automatic connection cleanup
pub struct ConnectionGuard {
    id: Uuid,
    pool: ConnectionPool,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl ConnectionGuard {
    pub fn id(&self) -> Uuid {
        self.id
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let pool = self.pool.clone();
        let id = self.id;
        tokio::spawn(async move {
            pool.remove(id).await;
        });
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub healthy_connections: usize,
    pub unhealthy_connections: usize,
    pub total_requests: u64,
    pub total_errors: u64,
    pub oldest_connection: Option<Duration>,
    pub max_connections: usize,
    pub pool_size: usize,
    pub total_created: u64,
    pub total_recycled: u64,
    pub avg_acquisition_time_ms: f64,
    pub reuse_rate: f64,
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub min_connections: usize,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_pool() {
        let pool = ConnectionPool::new(10, Duration::from_secs(60));
        
        // Test acquiring connections
        let guard1 = pool.acquire().await;
        let guard2 = pool.acquire().await;
        
        assert_eq!(pool.active_count().await, 2);
        
        // Test connection removal
        drop(guard1);
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(pool.active_count().await, 1);
        
        // Test stats
        let stats = pool.stats().await;
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.healthy_connections, 1);
    }
    
    #[tokio::test]
    async fn test_cleanup() {
        let pool = ConnectionPool::new(10, Duration::from_millis(100));
        
        let guard = pool.acquire().await;
        let id = guard.id();
        
        // Connection should exist
        assert_eq!(pool.active_count().await, 1);
        
        // Wait for idle timeout
        tokio::time::sleep(Duration::from_millis(200)).await;
        pool.cleanup().await;
        
        // Connection should be removed
        assert_eq!(pool.active_count().await, 0);
    }
}
