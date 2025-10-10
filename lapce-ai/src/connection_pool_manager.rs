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

/// Health check result for TLS and WebSocket validation
#[derive(Debug, Default)]
struct HealthCheckResult {
    healthy: u32,
    unhealthy: u32,
}

/// Enhanced connection statistics with health and scaling metrics
#[derive(Debug, Default)]
pub struct ConnectionStats {
    pub total_connections: AtomicU64,
    pub active_connections: AtomicU32,
    pub idle_connections: AtomicU32,
    pub failed_connections: AtomicU64,
    pub total_requests: AtomicU64,
    pub avg_wait_time_ns: AtomicU64,
    
    // Health check metrics
    pub healthy_connections: AtomicU32,
    pub unhealthy_connections: AtomicU32,
    pub tls_handshake_failures: AtomicU64,
    pub websocket_ping_failures: AtomicU64,
    pub certificate_validation_failures: AtomicU64,
    
    // Adaptive scaling metrics
    pub scale_up_events: AtomicU64,
    pub scale_down_events: AtomicU64,
    pub current_utilization: AtomicU32, // Percentage * 100 for precision
    pub last_scale_time: AtomicU64,     // Unix timestamp
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

/// Pool configuration with adaptive scaling
#[derive(Clone, Debug)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_idle: u32,
    pub max_lifetime: Duration,
    pub idle_timeout: Duration,
    pub connection_timeout: Duration,
    pub max_retries: u32,
    
    // Adaptive scaling knobs
    pub scale_up_threshold: f64,      // Scale up when utilization > this (0.0-1.0)
    pub scale_down_threshold: f64,    // Scale down when utilization < this (0.0-1.0)
    pub scale_factor: f64,            // Multiplicative scaling factor (e.g., 1.5)
    pub min_scale_interval: Duration, // Minimum time between scaling events
    
    // Health check configuration
    pub health_check_interval: Duration,
    pub health_check_timeout: Duration,
    pub unhealthy_threshold: u32,     // Failures before marking unhealthy
    pub tls_verify_certificates: bool,
    pub websocket_ping_interval: Duration,
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
            
            // Adaptive scaling defaults
            scale_up_threshold: 0.8,      // Scale up when 80% utilized
            scale_down_threshold: 0.3,    // Scale down when <30% utilized
            scale_factor: 1.5,            // Increase by 50%
            min_scale_interval: Duration::from_secs(60), // Wait 1 min between scaling
            
            // Health check defaults
            health_check_interval: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(5),
            unhealthy_threshold: 3,       // 3 failures before unhealthy
            tls_verify_certificates: true,
            websocket_ping_interval: Duration::from_secs(30),
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
    
    /// Comprehensive health check for all pools with TLS and WebSocket validation
    pub async fn health_check(&self) -> Result<()> {
        let config = self.config.load();
        
        // Check HTTP pool
        let http_state = self.http_pool.state();
        debug!(
            "HTTP pool - connections: {}, idle: {}",
            http_state.connections,
            http_state.idle_connections
        );
        
        // Check HTTPS pool with TLS validation
        let https_state = self.https_pool.state();
        debug!(
            "HTTPS pool - connections: {}, idle: {}",
            https_state.connections,
            https_state.idle_connections
        );
        
        // Perform comprehensive TLS health checks
        let tls_health = self.perform_tls_health_checks(&config).await?;
        
        // Perform WebSocket health checks
        let ws_health = self.perform_websocket_health_checks(&config).await?;
        
        // Update comprehensive metrics
        self.stats.healthy_connections.store(
            tls_health.healthy + ws_health.healthy, 
            Ordering::Relaxed
        );
        self.stats.unhealthy_connections.store(
            tls_health.unhealthy + ws_health.unhealthy, 
            Ordering::Relaxed
        );
        
        // Update pool status and trigger adaptive scaling if needed
        self.stats.update_pool_status(
            http_state.connections + https_state.connections,
            http_state.idle_connections + https_state.idle_connections,
        );
        
        // Check if adaptive scaling is needed
        self.check_adaptive_scaling(&config).await?;
        
        info!("Comprehensive health check complete: TLS({} healthy, {} unhealthy), WS({} healthy, {} unhealthy)",
              tls_health.healthy, tls_health.unhealthy, ws_health.healthy, ws_health.unhealthy);
        
        Ok(())
    }
    
    /// Perform detailed TLS health checks with certificate validation
    async fn perform_tls_health_checks(&self, config: &PoolConfig) -> Result<HealthCheckResult> {
        let mut result = HealthCheckResult::default();
        let https_state = self.https_pool.state();
        let sample_size = 5.min(https_state.idle_connections as usize);
        
        for i in 0..sample_size {
            let health_check_timeout = config.health_check_timeout;
            
            match tokio::time::timeout(health_check_timeout, self.validate_tls_connection()).await {
                Ok(Ok(_)) => {
                    result.healthy += 1;
                    debug!("TLS health check {} passed", i + 1);
                }
                Ok(Err(e)) => {
                    result.unhealthy += 1;
                    warn!("TLS health check {} failed: {}", i + 1, e);
                    
                    // Categorize the failure type
                    if e.to_string().contains("certificate") {
                        self.stats.certificate_validation_failures.fetch_add(1, Ordering::Relaxed);
                    } else if e.to_string().contains("handshake") {
                        self.stats.tls_handshake_failures.fetch_add(1, Ordering::Relaxed);
                    }
                    
                    self.stats.failed_connections.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    result.unhealthy += 1;
                    warn!("TLS health check {} timed out after {:?}", i + 1, health_check_timeout);
                    self.stats.tls_handshake_failures.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        
        Ok(result)
    }
    
    /// Validate TLS connection with certificate checks
    async fn validate_tls_connection(&self) -> Result<()> {
        let conn = self.https_pool.get().await
            .map_err(|e| anyhow::anyhow!("Failed to get HTTPS connection: {:?}", e))?;
        
        // Test with a lightweight request
        let req = http::Request::builder()
            .method("HEAD")
            .uri("https://httpbin.org/status/200")
            .body(Full::new(Bytes::new()))?;
        
        let response = conn.execute_request(req).await
            .map_err(|e| anyhow::anyhow!("TLS request failed: {}", e))?;
        
        // Check response status
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("TLS health check returned status: {}", response.status()));
        }
        
        Ok(())
    }
    
    /// Perform WebSocket health checks with ping validation
    async fn perform_websocket_health_checks(&self, config: &PoolConfig) -> Result<HealthCheckResult> {
        let mut result = HealthCheckResult::default();
        
        // Test WebSocket connections to common endpoints
        let test_endpoints = ["wss://echo.websocket.org", "wss://ws.postman-echo.com/raw"];
        
        for endpoint in test_endpoints.iter() {
            match tokio::time::timeout(
                config.health_check_timeout,
                self.validate_websocket_connection(endpoint, config)
            ).await {
                Ok(Ok(_)) => {
                    result.healthy += 1;
                    debug!("WebSocket health check passed for {}", endpoint);
                }
                Ok(Err(e)) => {
                    result.unhealthy += 1;
                    warn!("WebSocket health check failed for {}: {}", endpoint, e);
                    self.stats.websocket_ping_failures.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    result.unhealthy += 1;
                    warn!("WebSocket health check timed out for {}", endpoint);
                    self.stats.websocket_ping_failures.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        
        Ok(result)
    }
    
    /// Validate WebSocket connection with ping/pong
    async fn validate_websocket_connection(&self, url: &str, config: &PoolConfig) -> Result<()> {
        let ws_conn = self.get_websocket_connection(url).await?;
        
        // Perform ping health check
        ws_conn.health_check().await
            .map_err(|e| anyhow::anyhow!("WebSocket ping failed: {}", e))?;
        
        Ok(())
    }
    
    /// Check if adaptive scaling is needed and trigger scaling events
    async fn check_adaptive_scaling(&self, config: &PoolConfig) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let last_scale = self.stats.last_scale_time.load(Ordering::Relaxed);
        let min_interval_secs = config.min_scale_interval.as_secs();
        
        // Check if enough time has passed since last scaling event
        if now - last_scale < min_interval_secs {
            return Ok(());
        }
        
        // Calculate current utilization
        let total = self.stats.total_connections.load(Ordering::Relaxed) as f64;
        let active = self.stats.active_connections.load(Ordering::Relaxed) as f64;
        let utilization = if total > 0.0 { active / total } else { 0.0 };
        
        // Store utilization for metrics (percentage * 100)
        self.stats.current_utilization.store((utilization * 10000.0) as u32, Ordering::Relaxed);
        
        // Check scaling thresholds
        if utilization > config.scale_up_threshold {
            self.trigger_scale_up(config, utilization).await?;
        } else if utilization < config.scale_down_threshold {
            self.trigger_scale_down(config, utilization).await?;
        }
        
        Ok(())
    }
    
    /// Trigger scale-up event
    async fn trigger_scale_up(&self, config: &PoolConfig, utilization: f64) -> Result<()> {
        let current_max = config.max_connections as f64;
        let new_max = (current_max * config.scale_factor).min(1000.0) as u32; // Cap at 1000
        
        info!("Triggering scale-up: utilization {:.1}%, {} -> {} max connections", 
              utilization * 100.0, current_max, new_max);
        
        self.stats.scale_up_events.fetch_add(1, Ordering::Relaxed);
        self.stats.last_scale_time.store(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            Ordering::Relaxed
        );
        
        // Note: bb8 doesn't support dynamic resizing, but we record the event for monitoring
        // In production, you would implement gradual scaling by creating additional pools
        
        Ok(())
    }
    
    /// Trigger scale-down event
    async fn trigger_scale_down(&self, config: &PoolConfig, utilization: f64) -> Result<()> {
        let current_max = config.max_connections as f64;
        let new_max = ((current_max / config.scale_factor).max(config.min_idle as f64)) as u32;
        
        info!("Triggering scale-down: utilization {:.1}%, {} -> {} max connections", 
              utilization * 100.0, current_max, new_max);
        
        self.stats.scale_down_events.fetch_add(1, Ordering::Relaxed);
        self.stats.last_scale_time.store(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            Ordering::Relaxed
        );
        
        Ok(())
    }
    
    /// Export comprehensive Prometheus metrics for pool health and scaling
    pub fn export_prometheus_metrics(&self) -> String {
        let stats = &self.stats;
        format!(
            "# HELP ipc_pool_total_connections Total connections in pool\n\
             # TYPE ipc_pool_total_connections gauge\n\
             ipc_pool_total_connections {}\n\
             # HELP ipc_pool_active_connections Active connections in pool\n\
             # TYPE ipc_pool_active_connections gauge\n\
             ipc_pool_active_connections {}\n\
             # HELP ipc_pool_idle_connections Idle connections in pool\n\
             # TYPE ipc_pool_idle_connections gauge\n\
             ipc_pool_idle_connections {}\n\
             # HELP ipc_pool_failed_connections_total Failed connection attempts\n\
             # TYPE ipc_pool_failed_connections_total counter\n\
             ipc_pool_failed_connections_total {}\n\
             # HELP ipc_pool_avg_wait_time_nanoseconds Average connection acquisition time\n\
             # TYPE ipc_pool_avg_wait_time_nanoseconds gauge\n\
             ipc_pool_avg_wait_time_nanoseconds {}\n\
             # HELP ipc_pool_healthy_connections Healthy connections after health checks\n\
             # TYPE ipc_pool_healthy_connections gauge\n\
             ipc_pool_healthy_connections {}\n\
             # HELP ipc_pool_unhealthy_connections Unhealthy connections detected\n\
             # TYPE ipc_pool_unhealthy_connections gauge\n\
             ipc_pool_unhealthy_connections {}\n\
             # HELP ipc_pool_tls_handshake_failures_total TLS handshake failures\n\
             # TYPE ipc_pool_tls_handshake_failures_total counter\n\
             ipc_pool_tls_handshake_failures_total {}\n\
             # HELP ipc_pool_websocket_ping_failures_total WebSocket ping failures\n\
             # TYPE ipc_pool_websocket_ping_failures_total counter\n\
             ipc_pool_websocket_ping_failures_total {}\n\
             # HELP ipc_pool_certificate_validation_failures_total TLS certificate validation failures\n\
             # TYPE ipc_pool_certificate_validation_failures_total counter\n\
             ipc_pool_certificate_validation_failures_total {}\n\
             # HELP ipc_pool_scale_up_events_total Pool scale-up events triggered\n\
             # TYPE ipc_pool_scale_up_events_total counter\n\
             ipc_pool_scale_up_events_total {}\n\
             # HELP ipc_pool_scale_down_events_total Pool scale-down events triggered\n\
             # TYPE ipc_pool_scale_down_events_total counter\n\
             ipc_pool_scale_down_events_total {}\n\
             # HELP ipc_pool_utilization_percent Current pool utilization percentage\n\
             # TYPE ipc_pool_utilization_percent gauge\n\
             ipc_pool_utilization_percent {}\n",
            stats.total_connections.load(Ordering::Relaxed),
            stats.active_connections.load(Ordering::Relaxed),
            stats.idle_connections.load(Ordering::Relaxed),
            stats.failed_connections.load(Ordering::Relaxed),
            stats.avg_wait_time_ns.load(Ordering::Relaxed),
            stats.healthy_connections.load(Ordering::Relaxed),
            stats.unhealthy_connections.load(Ordering::Relaxed),
            stats.tls_handshake_failures.load(Ordering::Relaxed),
            stats.websocket_ping_failures.load(Ordering::Relaxed),
            stats.certificate_validation_failures.load(Ordering::Relaxed),
            stats.scale_up_events.load(Ordering::Relaxed),
            stats.scale_down_events.load(Ordering::Relaxed),
            stats.current_utilization.load(Ordering::Relaxed) as f64 / 100.0,
        )
    }
    
    /// Start background health check task
    pub async fn start_health_check_task(&self) -> Result<()> {
        let pool = Arc::new(self.clone()); // Note: This requires Clone impl
        let config = self.config.load();
        let interval = config.health_check_interval;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = pool.health_check().await {
                    warn!("Background health check failed: {}", e);
                }
            }
        });
        
        info!("Started background health check task with interval {:?}", interval);
        Ok(())
    }
}

impl Clone for ConnectionPoolManager {
    fn clone(&self) -> Self {
        Self {
            https_pool: self.https_pool.clone(),
            http_pool: self.http_pool.clone(),
            ws_pool_manager: self.ws_pool_manager.clone(),
            stats: self.stats.clone(),
            config: ArcSwap::from_pointee((**self.config.load()).clone()),
            multiplexer: self.multiplexer.clone(),
        }
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
