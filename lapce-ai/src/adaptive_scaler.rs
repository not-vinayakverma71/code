/// Adaptive Connection Pool Scaling
/// Dynamically adjusts pool size based on load

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{info, debug};

use crate::connection_pool_manager::ConnectionPoolManager;
use crate::connection_pool_manager::ConnectionStats;

/// Adaptive scaler for connection pools
pub struct AdaptiveScaler {
    pool: Arc<ConnectionPoolManager>,
    metrics: Arc<ConnectionStats>,
    config: Arc<RwLock<ScalerConfig>>,
    last_scale_time: Arc<RwLock<Instant>>,
}

#[derive(Debug, Clone)]
struct DetailedStats {
    total_connections: u64,
    active_connections: u32,
    idle_connections: u32,
    failed_connections: u64,
    avg_wait_time_ns: u64,
}

impl DetailedStats {
    fn avg_wait_time_ms(&self) -> f64 {
        self.avg_wait_time_ns as f64 / 1_000_000.0
    }
}

#[derive(Clone, Debug)]
pub struct ScalerConfig {
    pub min_connections: u32,
    pub max_connections: u32,
    pub scale_up_threshold: f64,  // Utilization percentage
    pub scale_down_threshold: f64,
    pub scale_factor: f64,
    pub cooldown_period: Duration,
    pub monitoring_interval: Duration,
}

impl Default for ScalerConfig {
    fn default() -> Self {
        Self {
            min_connections: 10,
            max_connections: 500,
            scale_up_threshold: 0.8,
            scale_down_threshold: 0.3,
            scale_factor: 1.2,
            cooldown_period: Duration::from_secs(30),
            monitoring_interval: Duration::from_secs(10),
        }
    }
}

impl AdaptiveScaler {
    /// Create new adaptive scaler
    pub fn new(pool: Arc<ConnectionPoolManager>) -> Self {
        let metrics = pool.get_stats();
        
        Self {
            pool,
            metrics,
            config: Arc::new(RwLock::new(ScalerConfig::default())),
            last_scale_time: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    /// Start adaptive scaling monitoring
    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = {
                let config = self.config.read().await;
                tokio::time::interval(config.monitoring_interval)
            };
            
            loop {
                interval.tick().await;
                if let Err(e) = self.evaluate_and_scale().await {
                    debug!("Scaling evaluation error: {}", e);
                }
            }
        });
    }
    
    /// Evaluate metrics and scale if needed
    async fn evaluate_and_scale(&self) -> Result<()> {
        // Check cooldown period
        let last_scale = *self.last_scale_time.read().await;
        let config = self.config.read().await;
        
        if last_scale.elapsed() < config.cooldown_period {
            return Ok(());
        }
        
        // Get current metrics
        let stats = DetailedStats {
            total_connections: self.metrics.total_connections.load(std::sync::atomic::Ordering::Relaxed),
            active_connections: self.metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed),
            idle_connections: self.metrics.idle_connections.load(std::sync::atomic::Ordering::Relaxed),
            failed_connections: self.metrics.failed_connections.load(std::sync::atomic::Ordering::Relaxed),
            avg_wait_time_ns: self.metrics.avg_wait_time_ns.load(std::sync::atomic::Ordering::Relaxed),
        };
        let total = stats.total_connections as f64;
        let active = stats.active_connections as f64;
        
        if total == 0.0 {
            return Ok(());
        }
        
        let utilization = active / total;
        let avg_wait_time_ms = stats.avg_wait_time_ms();
        
        // Decide on scaling action
        if utilization > config.scale_up_threshold || avg_wait_time_ms > 100.0 {
            self.scale_up(config.clone()).await?;
        } else if utilization < config.scale_down_threshold && avg_wait_time_ms < 10.0 {
            self.scale_down(config.clone()).await?;
        }
        
        Ok(())
    }
    
    /// Scale up connection pool
    async fn scale_up(&self, config: ScalerConfig) -> Result<()> {
        let current_config = self.pool.config.load();
        let current_size = current_config.max_connections;
        
        let new_size = ((current_size as f64 * config.scale_factor) as u32)
            .min(config.max_connections);
        
        if new_size > current_size {
            let mut new_config = (**current_config).clone();
            new_config.max_connections = new_size;
            new_config.min_idle = (new_size / 10).max(1);
            
            self.pool.update_config(new_config);
            *self.last_scale_time.write().await = Instant::now();
            
            info!("Scaled up pool from {} to {} connections", current_size, new_size);
        }
        
        Ok(())
    }
    
    /// Scale down connection pool
    async fn scale_down(&self, config: ScalerConfig) -> Result<()> {
        let current_config = self.pool.config.load();
        let current_size = current_config.max_connections;
        
        let new_size = ((current_size as f64 / config.scale_factor) as u32)
            .max(config.min_connections);
        
        if new_size < current_size {
            let mut new_config = (**current_config).clone();
            new_config.max_connections = new_size;
            new_config.min_idle = (new_size / 10).max(1);
            
            self.pool.update_config(new_config);
            *self.last_scale_time.write().await = Instant::now();
            
            info!("Scaled down pool from {} to {} connections", current_size, new_size);
        }
        
        Ok(())
    }
    
    /// Update scaler configuration
    pub async fn update_config(&self, config: ScalerConfig) {
        *self.config.write().await = config;
    }
    
    /// Get current scaler status
    pub async fn get_status(&self) -> ScalerStatus {
        let config = self.config.read().await;
        let stats = DetailedStats {
            total_connections: self.metrics.total_connections.load(std::sync::atomic::Ordering::Relaxed),
            active_connections: self.metrics.active_connections.load(std::sync::atomic::Ordering::Relaxed),
            idle_connections: self.metrics.idle_connections.load(std::sync::atomic::Ordering::Relaxed),
            failed_connections: self.metrics.failed_connections.load(std::sync::atomic::Ordering::Relaxed),
            avg_wait_time_ns: self.metrics.avg_wait_time_ns.load(std::sync::atomic::Ordering::Relaxed),
        };
        let current_pool_config = self.pool.config.load();
        
        let utilization = if stats.total_connections > 0 {
            (stats.active_connections as f64 / stats.total_connections as f64) * 100.0
        } else {
            0.0
        };
        
        ScalerStatus {
            current_connections: current_pool_config.max_connections,
            min_connections: config.min_connections,
            max_connections: config.max_connections,
            utilization_percent: utilization,
            avg_wait_time_ms: stats.avg_wait_time_ms(),
            last_scale_time: *self.last_scale_time.read().await,
        }
    }
}

/// Scaler status
#[derive(Debug, Clone)]
pub struct ScalerStatus {
    pub current_connections: u32,
    pub min_connections: u32,
    pub max_connections: u32,
    pub utilization_percent: f64,
    pub avg_wait_time_ms: f64,
    pub last_scale_time: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection_pool_manager::PoolConfig;
    
    #[tokio::test]
    async fn test_adaptive_scaler_creation() {
        let pool_config = PoolConfig::default();
        let pool = Arc::new(ConnectionPoolManager::new(pool_config).await.unwrap());
        let scaler = Arc::new(AdaptiveScaler::new(pool));
        
        let status = scaler.get_status().await;
        assert_eq!(status.min_connections, 10);
        assert_eq!(status.max_connections, 500);
    }
    
    #[tokio::test]
    async fn test_scaler_config_update() {
        let pool_config = PoolConfig::default();
        let pool = Arc::new(ConnectionPoolManager::new(pool_config).await.unwrap());
        let scaler = Arc::new(AdaptiveScaler::new(pool));
        
        let new_config = ScalerConfig {
            min_connections: 5,
            max_connections: 1000,
            ..Default::default()
        };
        
        scaler.update_config(new_config).await;
        
        let status = scaler.get_status().await;
        assert_eq!(status.min_connections, 5);
        assert_eq!(status.max_connections, 1000);
    }
}
