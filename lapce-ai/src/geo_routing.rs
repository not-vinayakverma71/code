/// Geographic Connection Routing
/// Route connections to nearest/best performing regions

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::debug;

use crate::connection_pool_manager::{ConnectionPoolManager, PoolConfig};

/// Geographic routing pool for multi-region support
pub struct GeoRoutingPool {
    regions: HashMap<String, Arc<ConnectionPoolManager>>,
    latency_map: Arc<RwLock<HashMap<String, Duration>>>,
    default_region: String,
}

impl GeoRoutingPool {
    /// Create new geographic routing pool
    pub async fn new(config: PoolConfig) -> Result<Self> {
        let mut regions = HashMap::new();
        
        // Initialize regional pools
        for region in &["us-east-1", "us-west-2", "eu-west-1", "ap-southeast-1"] {
            let pool = Arc::new(ConnectionPoolManager::new(config.clone()).await?);
            regions.insert(region.to_string(), pool);
        }
        
        Ok(Self {
            regions,
            latency_map: Arc::new(RwLock::new(HashMap::new())),
            default_region: "us-east-1".to_string(),
        })
    }
    
    /// Get connection from best region based on latency
    pub async fn get_best_connection(&self) -> Result<Arc<ConnectionPoolManager>> {
        // Find region with lowest latency
        let latencies = self.latency_map.read().await;
        let best_region = latencies
            .iter()
            .min_by_key(|(_, latency)| *latency)
            .map(|(region, _)| region.clone())
            .unwrap_or_else(|| self.default_region.clone());
            
        // Get connection from best region
        self.regions
            .get(&best_region)
            .ok_or_else(|| anyhow::anyhow!("Region not found: {}", best_region))
            .map(|pool| pool.clone())
    }
    
    /// Get connection from specific region
    pub async fn get_regional_connection(&self, region: &str) -> Result<Arc<ConnectionPoolManager>> {
        self.regions
            .get(region)
            .ok_or_else(|| anyhow::anyhow!("Region not found: {}", region))
            .map(|pool| pool.clone())
    }
    
    /// Update latency measurements for all regions
    pub async fn update_latencies(&self) {
        for (region, pool) in &self.regions {
            let start = Instant::now();
            
            // Ping regional endpoint
            if let Ok(conn) = pool.get_https_connection().await {
                // Simulate latency check
                let latency = start.elapsed();
                self.latency_map.write().await.insert(region.clone(), latency);
                debug!("Region {} latency: {:?}", region, latency);
            }
        }
    }
    
    /// Start background latency monitoring
    pub fn start_monitoring(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.update_latencies().await;
            }
        });
    }
    
    /// Get regional statistics
    pub async fn get_regional_stats(&self) -> HashMap<String, RegionalStats> {
        let mut stats = HashMap::new();
        let latencies = self.latency_map.read().await;
        
        for (region, pool) in &self.regions {
            let pool_stats = pool.get_stats();
            stats.insert(region.clone(), RegionalStats {
                latency: latencies.get(region).cloned(),
                total_connections: pool_stats.total_connections.load(std::sync::atomic::Ordering::Relaxed),
                active_connections: pool_stats.active_connections.load(std::sync::atomic::Ordering::Relaxed) as u64,
            });
        }
        
        stats
    }
}

/// Regional statistics
#[derive(Debug, Clone)]
pub struct RegionalStats {
    pub latency: Option<Duration>,
    pub total_connections: u64,
    pub active_connections: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_geo_routing_pool() {
        let config = PoolConfig::default();
        let pool = GeoRoutingPool::new(config).await;
        assert!(pool.is_ok());
        
        let pool = pool.unwrap();
        assert_eq!(pool.regions.len(), 4);
    }
    
    #[tokio::test]
    async fn test_regional_connection() {
        let config = PoolConfig::default();
        let pool = GeoRoutingPool::new(config).await.unwrap();
        
        let conn = pool.get_regional_connection("us-east-1").await;
        assert!(conn.is_ok());
        
        let conn = pool.get_regional_connection("invalid-region").await;
        assert!(conn.is_err());
    }
}
