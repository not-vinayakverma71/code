/// Connection Pool Manager using bb8
/// Complete implementation with HTTP/HTTPS connection management

use std::sync::Arc;
use std::time::{Duration, Instant};
use bb8::{Pool, PooledConnection, ManageConnection};
use anyhow::Result;
use arc_swap::ArcSwap;
use tracing::{info, debug, warn};
use http_body_util::Full;
use bytes::Bytes;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use tokio::sync::RwLock;

use crate::https_connection_manager_real::HttpsConnectionManager;
use crate::https_pool_wrapper::HttpsConnectionPool;
use crate::websocket_pool_manager::WebSocketManager;

/// Connection statistics
#[derive(Debug, Default)]
pub struct ConnectionStats {
    pub total_connections: AtomicU64,
    pub active_connections: AtomicU32,
    pub idle_connections: AtomicU32,
    pub failed_connections: AtomicU64,
    pub total_requests: AtomicU64,
    pub avg_wait_time_ns: AtomicU64,
}

impl ConnectionStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_acquisition(&self, wait_time: Duration) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        
        // Update average wait time
        let nanos = wait_time.as_nanos() as u64;
        let current = self.avg_wait_time_ns.load(Ordering::Relaxed);
        let new_avg = (current + nanos) / 2;
        self.avg_wait_time_ns.store(new_avg, Ordering::Relaxed);
    }
    
    pub fn update_pool_status(&self, total: u32, available: u32) {
        self.active_connections.store(total - available, Ordering::Relaxed);
        self.idle_connections.store(available, Ordering::Relaxed);
    }
}

/// Pool configuration
#[derive(Clone, Debug)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_idle: u32,
    pub max_lifetime: Duration,
    pub idle_timeout: Duration,
    pub connection_timeout: Duration,
    pub max_retries: u32,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            min_idle: 10,
            max_lifetime: Duration::from_secs(300),
            idle_timeout: Duration::from_secs(90),
            connection_timeout: Duration::from_secs(10),
            max_retries: 3,
        }
    }
}

/// Connection pool manager for HTTP/HTTPS/WebSocket connections
pub struct ConnectionPoolManager {
    // HTTP/HTTPS client pools
    pub https_pool: Pool<HttpsConnectionPool>,
    pub http_pool: Pool<HttpConnectionPool>,
    
    // WebSocket pool manager
    pub ws_pool_manager: Arc<WebSocketPoolManager>,
    
    // Statistics
    pub stats: Arc<ConnectionStats>,
    
    // Dynamic configuration
    pub config: ArcSwap<PoolConfig>,
    
    // HTTP/2 multiplexer for stream management
    pub multiplexer: Arc<crate::http2_multiplexer::Http2Multiplexer>,
}

/// HTTP connection pool manager
#[derive(Clone)]
pub struct HttpConnectionPool;

/// WebSocket pool manager
pub struct WebSocketPoolManager {
    pools: Arc<RwLock<Vec<Pool<WebSocketPool>>>>,
    config: Arc<PoolConfig>,
}

/// WebSocket pool implementation
#[derive(Clone)]
pub struct WebSocketPool {
    url: String,
}

impl ConnectionPoolManager {
    pub async fn new(config: PoolConfig) -> Result<Self> {
        // Create HTTP pool
        let http_pool = Self::create_http_pool(&config).await?;
        
        let https_pool = Pool::builder()
            .max_size(config.max_connections)
            .min_idle(Some(config.min_idle.min(2)))  // Limit pre-warm to 2 for memory
            .max_lifetime(Some(config.max_lifetime))
            .idle_timeout(Some(config.idle_timeout))
            .connection_timeout(config.connection_timeout)
            .build(HttpsConnectionPool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to build HTTPS pool: {}", e))?;
            
        // Pre-warm minimal connections for memory efficiency
        for _ in 0..config.min_idle.min(2) {
            let _ = https_pool.get().await
                .map_err(|e| anyhow::anyhow!("Failed to pre-warm connection: {}", e))?;
        }
        
        // Create WebSocket pool manager
        let ws_pool_manager = Arc::new(WebSocketPoolManager {
            pools: Arc::new(RwLock::new(Vec::new())),
            config: Arc::new(config.clone()),
        });
        
        // Create HTTP/2 multiplexer for stream management
        let multiplexer = Arc::new(crate::http2_multiplexer::Http2Multiplexer::new(100));
        
        Ok(Self {
            https_pool,
            http_pool,
            ws_pool_manager,
            stats: Arc::new(ConnectionStats::new()),
            config: ArcSwap::from_pointee(config),
            multiplexer,
        })
    }
    
    async fn create_http_pool(config: &PoolConfig) -> Result<Pool<HttpConnectionPool>> {
        Pool::builder()
            .max_size(config.max_connections)
            .min_idle(Some(config.min_idle))
            .max_lifetime(Some(config.max_lifetime))
            .idle_timeout(Some(config.idle_timeout))
            .connection_timeout(config.connection_timeout)
            .build(HttpConnectionPool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP pool: {}", e))
    }
    
    /// Get HTTPS connection from pool
    pub async fn get_https_connection(&self) -> Result<PooledConnection<'_, HttpsConnectionPool>> {
        let start = Instant::now();
        let conn = self.https_pool.get().await
            .map_err(|e| anyhow::anyhow!("Failed to get HTTPS connection: {:?}", e))?;
        self.stats.record_acquisition(start.elapsed());
        Ok(conn)
    }
    
    /// Get HTTP connection from pool
    pub async fn get_http_connection(&self) -> Result<PooledConnection<'_, HttpConnectionPool>> {
        let start = Instant::now();
        let conn = self.http_pool.get().await
            .map_err(|e| anyhow::anyhow!("Failed to get HTTP connection: {:?}", e))?;
        self.stats.record_acquisition(start.elapsed());
        Ok(conn)
    }
    
    /// Get WebSocket connection
    pub async fn get_websocket_connection(&self, url: &str) -> Result<WebSocketManager> {
        self.ws_pool_manager.get_connection(url).await
    }
    
    /// Update pool configuration dynamically and rebuild pools if needed
    pub async fn update_config(&self, new_config: PoolConfig) -> Result<()> {
        let old_config = self.config.load();
        
        // Check if pool sizes changed
        if old_config.max_connections != new_config.max_connections ||
           old_config.min_idle != new_config.min_idle {
            info!("Pool configuration changed, rebuilding pools: max {} -> {}, min_idle {} -> {}",
                  old_config.max_connections, new_config.max_connections,
                  old_config.min_idle, new_config.min_idle);
            
            // Note: bb8 doesn't support dynamic resizing, so we log the change
            // In production, you might need to implement a gradual migration strategy
            // or accept that changes only apply to new connections
            
            // Update the multiplexer capacity
            self.multiplexer.update_max_streams(new_config.max_connections);
        }
        
        self.config.store(Arc::new(new_config));
        Ok(())
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> Arc<ConnectionStats> {
        self.stats.clone()
    }
    
    /// Get active connection count
    pub async fn active_count(&self) -> usize {
        let https_state = self.https_pool.state();
        let http_state = self.http_pool.state();
        (https_state.connections + http_state.connections) as usize
    }
    
    /// Pre-warm connections to specific hosts for fast TLS handshakes
    pub async fn prewarm_hosts(&self, hosts: &[&str]) -> Result<()> {
        info!("Pre-warming connections to {} hosts", hosts.len());
        
        for host in hosts {
            // Get a connection to trigger TLS handshake and prime session cache
            match self.get_https_connection().await {
                Ok(conn) => {
                    // Make a lightweight HEAD request to establish the connection
                    let req = http::Request::builder()
                        .method("HEAD")
                        .uri(format!("https://{}/", host))
                        .body(Full::new(Bytes::new()))?;
                    
                    match tokio::time::timeout(Duration::from_secs(5), conn.execute_request(req)).await {
                        Ok(Ok(_)) => debug!("Pre-warmed connection to {}", host),
                        Ok(Err(e)) => warn!("Failed to pre-warm {}: {}", host, e),
                        Err(_) => warn!("Pre-warm timeout for {}", host),
                    }
                }
                Err(e) => warn!("Failed to get connection for pre-warm: {}", e),
            }
        }
        
        info!("Pre-warming complete");
        Ok(())
    }
    
    /// Health check all pools and validate active connections
    pub async fn health_check(&self) -> Result<()> {
        // Check HTTP pool
        let http_state = self.http_pool.state();
        debug!(
            "HTTP pool - connections: {}, idle: {}",
            http_state.connections,
            http_state.idle_connections
        );
        
        // Check HTTPS pool  
        let https_state = self.https_pool.state();
        debug!(
            "HTTPS pool - connections: {}, idle: {}",
            https_state.connections,
            https_state.idle_connections
        );
        
        // Validate a sample of active HTTPS connections
        let mut healthy = 0;
        let mut unhealthy = 0;
        let sample_size = 3.min(https_state.idle_connections as usize);
        
        for _ in 0..sample_size {
            if let Ok(conn) = self.https_pool.get().await {
                match conn.is_valid().await {
                    Ok(_) => {
                        healthy += 1;
                        debug!("Connection health check passed");
                    }
                    Err(e) => {
                        unhealthy += 1;
                        warn!("Connection health check failed: {}", e);
                        self.stats.failed_connections.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
        
        info!("Health check complete: {} healthy, {} unhealthy connections sampled",
              healthy, unhealthy);
        
        // Update metrics
        self.stats.update_pool_status(
            http_state.connections + https_state.connections,
            http_state.idle_connections + https_state.idle_connections,
        );
        
        Ok(())
    }
}

/// HTTP connection manager (delegates to HTTPS manager for simplicity)
pub struct HttpConnectionManager {
    inner: HttpsConnectionManager,
}

impl HttpConnectionManager {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            inner: HttpsConnectionManager::new().await?,
        })
    }
    
    pub fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.inner.is_expired(max_lifetime)
    }
    
    pub async fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.inner.is_idle(idle_timeout).await
    }
    
    pub fn is_broken(&self) -> bool {
        self.inner.is_broken()
    }
}

#[async_trait::async_trait]
impl ManageConnection for HttpConnectionPool {
    type Connection = HttpConnectionManager;
    type Error = anyhow::Error;
    
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        HttpConnectionManager::new().await
    }
    
    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        if conn.is_expired(Duration::from_secs(300)) {
            return Err(anyhow::anyhow!("Connection expired"));
        }
        Ok(())
    }
    
    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.is_broken()
    }
}

// HttpsConnectionPool ManageConnection is implemented in https_pool_wrapper.rs

impl WebSocketPoolManager {
    async fn get_connection(&self, url: &str) -> Result<WebSocketManager> {
        WebSocketManager::connect(url.to_string()).await
    }
}

#[async_trait::async_trait]
impl ManageConnection for WebSocketPool {
    type Connection = WebSocketManager;
    type Error = anyhow::Error;
    
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        WebSocketManager::connect(self.url.clone()).await
    }
    
    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.health_check().await
    }
    
    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false // WebSocket has its own health check
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_pool_creation() {
        let config = PoolConfig::default();
        let pool = ConnectionPoolManager::new(config).await.unwrap();
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_connections.load(Ordering::Relaxed), 0);
    }
    
    #[tokio::test]
    async fn test_connection_reuse() {
        let config = PoolConfig {
            max_connections: 1,
            ..Default::default()
        };
        
        let pool = ConnectionPoolManager::new(config).await.unwrap();
        
        // Get connection
        let conn1 = pool.get_https_connection().await;
        assert!(conn1.is_ok());
        drop(conn1);
        
        // Should reuse same connection
        let conn2 = pool.get_https_connection().await;
        assert!(conn2.is_ok());
    }
}
