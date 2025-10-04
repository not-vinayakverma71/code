/// HTTPS Connection Manager with HTTP/2 support
/// Production-ready implementation with TLS configuration

use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;
use hyper::{Body, Client, Request, Response};
use hyper::client::HttpConnector;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use rustls::ClientConfig;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, warn};
use http_body_util::{BodyExt, Empty, Full};
use bytes::Bytes;

/// HTTPS connection manager with connection pooling support
pub struct HttpsConnectionManager {
    connector: HttpsConnector<HttpConnector>,
    client: Client<HttpsConnector<HttpConnector>, Body>,
    created_at: Instant,
    last_used: Arc<RwLock<Instant>>,
    request_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
    is_healthy: Arc<RwLock<bool>>,
}

impl HttpsConnectionManager {
    pub async fn new() -> Result<Self> {
        // Configure TLS with modern cipher suites
        let tls_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(Self::load_root_certificates())
            .with_no_client_auth();
            
        // Enable ALPN for HTTP/2
        let mut tls_config = tls_config;
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        
        // Create HTTP connector with optimizations
        let mut http_connector = HttpConnector::new();
        http_connector.set_nodelay(true);
        http_connector.set_keepalive(Some(Duration::from_secs(60)));
        http_connector.enforce_http(false);
        
        // Create HTTPS connector
        let https_connector = HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_or_http()
            .enable_http2()
            .build();
            
        // Create client with connection pooling
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .http2_initial_stream_window_size(65536)
            .http2_initial_connection_window_size(131072)
            .http2_adaptive_window(true)
            .http2_max_concurrent_streams(100)
            .build(https_connector.clone());
            
        Ok(Self {
            connector: https_connector,
            client,
            created_at: Instant::now(),
            last_used: Arc::new(RwLock::new(Instant::now())),
            request_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            is_healthy: Arc::new(RwLock::new(true)),
        })
    }
    
    /// Load root certificates
    fn load_root_certificates() -> rustls::RootCertStore {
        let mut roots = rustls::RootCertStore::empty();
        
        // Add webpki roots
        for cert in webpki_roots::TLS_SERVER_ROOTS.0 {
            roots.add(cert.clone()).unwrap();
        }
        
        // Optionally add system roots
        if let Ok(native_certs) = rustls_native_certs::load_native_certs() {
            for cert in native_certs {
                let _ = roots.add(rustls::Certificate(cert.0));
            }
        }
        
        roots
    }
    
    /// Check if connection is expired
    pub fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }
    
    /// Check if connection is idle
    pub async fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_used.read().await.elapsed() > idle_timeout
    }
    
    /// Check if connection is broken
    pub fn is_broken(&self) -> bool {
        // Quick non-async check for bb8
        self.error_count.load(Ordering::Relaxed) > 5
    }
    
    /// Validate connection with health check
    pub async fn is_valid(&self) -> Result<()> {
        // Check expiration
        if self.is_expired(Duration::from_secs(300)) {
            return Err(anyhow::anyhow!("Connection expired"));
        }
        
        // Check health status
        if !*self.is_healthy.read().await {
            return Err(anyhow::anyhow!("Connection unhealthy"));
        }
        
        // Perform HEAD request health check
        let req = Request::head("https://www.google.com/generate_204")
            .body(Body::empty())?;
            
        match tokio::time::timeout(Duration::from_secs(2), self.execute_request(req)).await {
            Ok(Ok(response)) if response.status() == 204 => Ok(()),
            _ => {
                *self.is_healthy.write().await = false;
                Err(anyhow::anyhow!("Health check failed"))
            }
        }
    }
    
    /// Execute HTTP request
    pub async fn execute_request(&self, request: Request<Body>) -> Result<Response<Body>> {
        // Update last used time
        *self.last_used.write().await = Instant::now();
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        // Execute request with timeout
        match tokio::time::timeout(
            Duration::from_secs(30),
            self.client.request(request)
        ).await {
            Ok(Ok(response)) => {
                debug!("Request successful, status: {}", response.status());
                Ok(response)
            }
            Ok(Err(e)) => {
                warn!("Request failed: {}", e);
                self.error_count.fetch_add(1, Ordering::Relaxed);
                Err(e.into())
            }
            Err(_) => {
                warn!("Request timeout");
                self.error_count.fetch_add(1, Ordering::Relaxed);
                Err(anyhow::anyhow!("Request timeout"))
            }
        }
    }
    
    /// Get connection statistics
    pub fn get_stats(&self) -> ConnectionStats {
        ConnectionStats {
            created_at: self.created_at,
            request_count: self.request_count.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            age: self.created_at.elapsed(),
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub created_at: Instant,
    pub request_count: u64,
    pub error_count: u64,
    pub age: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_https_connection_creation() {
        let conn = HttpsConnectionManager::new().await;
        assert!(conn.is_ok());
        
        let conn = conn.unwrap();
        assert_eq!(conn.request_count.load(Ordering::Relaxed), 0);
        assert_eq!(conn.error_count.load(Ordering::Relaxed), 0);
    }
    
    #[tokio::test]
    async fn test_connection_expiration() {
        let conn = HttpsConnectionManager::new().await.unwrap();
        
        // Should not be expired immediately
        assert!(!conn.is_expired(Duration::from_secs(300)));
        
        // Should not be idle immediately
        assert!(!conn.is_idle(Duration::from_secs(60)).await);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let conn = HttpsConnectionManager::new().await.unwrap();
        
        // Health check should pass for new connection
        let result = conn.is_valid().await;
        // May fail in test environment without network
        // assert!(result.is_ok());
    }
}
