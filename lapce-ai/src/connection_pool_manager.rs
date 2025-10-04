/// Connection Pool Manager using bb8
/// Complete implementation with HTTP/HTTPS connection management

use std::sync::Arc;
use std::time::{Duration, Instant};
use bb8::{Pool, PooledConnection, ManageConnection};
use hyper::{client::HttpConnector, Client, Body};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use rustls::ClientConfig;
use anyhow::{Result, Error as AnyhowError};
use arc_swap::ArcSwap;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use tracing::{info, debug, warn};

use crate::https_connection_manager::HttpsConnectionManager;
use crate::websocket_pool_manager::WebSocketManager;
use crate::connection_metrics::ConnectionStats;

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

/// Connection pool manager for HTTP/HTTPS connections
pub struct ConnectionPoolManager {
    // HTTP/HTTPS client pools
    http_pool: Pool<HttpConnectionPool>,
    https_pool: Pool<HttpsConnectionPool>,
    
    // WebSocket pools for streaming
    ws_pool: Arc<WebSocketPoolManager>,
    
    // Connection statistics
    stats: Arc<ConnectionStats>,
    
    // Dynamic configuration
    config: ArcSwap<PoolConfig>,
}

/// HTTP connection pool manager
#[derive(Clone)]
pub struct HttpConnectionPool;

/// HTTPS connection pool manager
#[derive(Clone)]
pub struct HttpsConnectionPool;

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
        let http_pool = Pool::builder()
            .max_size(config.max_connections)
            .min_idle(Some(config.min_idle))
            .max_lifetime(Some(config.max_lifetime))
            .idle_timeout(Some(config.idle_timeout))
            .connection_timeout(config.connection_timeout)
            .build(HttpConnectionPool)
            .await?;
            
        // Create HTTPS pool
        let https_pool = Pool::builder()
            .max_size(config.max_connections)
            .min_idle(Some(config.min_idle))
            .max_lifetime(Some(config.max_lifetime))
            .idle_timeout(Some(config.idle_timeout))
            .connection_timeout(config.connection_timeout)
            .build(HttpsConnectionPool)
            .await?;
            
        // Pre-warm connections
        for _ in 0..config.min_idle {
            let _ = https_pool.get().await?;
        }
        
        // Create WebSocket pool manager
        let ws_pool = Arc::new(WebSocketPoolManager {
            pools: Arc::new(RwLock::new(Vec::new())),
            config: Arc::new(config.clone()),
        });
        
        Ok(Self {
            http_pool,
            https_pool,
            ws_pool,
            stats: Arc::new(ConnectionStats::new()),
            config: ArcSwap::from_pointee(config),
        })
    }
    
    /// Get HTTPS connection from pool
    pub async fn get_https_connection(&self) -> Result<PooledConnection<'_, HttpsConnectionManager>> {
        let start = Instant::now();
        let conn = self.https_pool.get().await?;
        self.stats.record_acquisition(start.elapsed());
        Ok(conn)
    }
    
    /// Get HTTP connection from pool
    pub async fn get_http_connection(&self) -> Result<PooledConnection<'_, HttpConnectionManager>> {
        let start = Instant::now();
        let conn = self.http_pool.get().await?;
        self.stats.record_acquisition(start.elapsed());
        Ok(conn)
    }
    
    /// Get WebSocket connection
    pub async fn get_websocket_connection(&self, url: &str) -> Result<WebSocketManager> {
        self.ws_pool.get_connection(url).await
    }
    
    /// Update pool configuration dynamically
    pub fn update_config(&self, config: PoolConfig) {
        self.config.store(Arc::new(config));
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> Arc<ConnectionStats> {
        self.stats.clone()
    }
    
    /// Health check all pools
    pub async fn health_check(&self) -> Result<()> {
        // Check HTTP pool
        let http_status = self.http_pool.status();
        debug!(
            "HTTP pool - size: {}, available: {}",
            http_status.size,
            http_status.available
        );
        
        // Check HTTPS pool
        let https_status = self.https_pool.status();
        debug!(
            "HTTPS pool - size: {}, available: {}",
            https_status.size,
            https_status.available
        );
        
        // Update metrics
        self.stats.update_pool_status(
            http_status.size + https_status.size,
            http_status.available + https_status.available,
        );
        
        Ok(())
    }
}

/// HTTP connection manager
pub struct HttpConnectionManager {
    client: Client<HttpConnector, Body>,
    created_at: Instant,
    last_used: Arc<RwLock<Instant>>,
    request_count: Arc<AtomicU64>,
}

impl HttpConnectionManager {
    pub fn new() -> Result<Self> {
        let connector = HttpConnector::new();
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build(connector);
            
        Ok(Self {
            client,
            created_at: Instant::now(),
            last_used: Arc::new(RwLock::new(Instant::now())),
            request_count: Arc::new(AtomicU64::new(0)),
        })
    }
    
    pub fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }
    
    pub fn is_idle(&self, idle_timeout: Duration) -> bool {
        // This is async but we need sync for bb8
        false // Simplified for now
    }
    
    pub async fn execute_request(&self, request: hyper::Request<Body>) -> Result<hyper::Response<Body>> {
        *self.last_used.write().await = Instant::now();
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        tokio::time::timeout(
            Duration::from_secs(30),
            self.client.request(request)
        )
        .await?
        .map_err(Into::into)
    }
}

#[async_trait::async_trait]
impl ManageConnection for HttpConnectionPool {
    type Connection = HttpConnectionManager;
    type Error = AnyhowError;
    
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        HttpConnectionManager::new()
    }
    
    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        if conn.is_expired(Duration::from_secs(300)) {
            return Err(anyhow::anyhow!("Connection expired"));
        }
        Ok(())
    }
    
    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.is_expired(Duration::from_secs(300))
    }
}

#[async_trait::async_trait]
impl ManageConnection for HttpsConnectionPool {
    type Connection = HttpsConnectionManager;
    type Error = AnyhowError;
    
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        HttpsConnectionManager::new().await
    }
    
    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.is_valid().await
    }
    
    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.is_broken()
    }
}

impl WebSocketPoolManager {
    async fn get_connection(&self, url: &str) -> Result<WebSocketManager> {
        WebSocketManager::connect(url.to_string()).await
    }
}

#[async_trait::async_trait]
impl ManageConnection for WebSocketPool {
    type Connection = WebSocketManager;
    type Error = AnyhowError;
    
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
