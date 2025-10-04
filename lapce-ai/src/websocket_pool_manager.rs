/// WebSocket Connection Pool Manager
/// Handles WebSocket connections with auto-reconnection and health checks

use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tokio_tungstenite::tungstenite::{Message, Error as WsError};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use url::Url;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{info, debug, warn, error};

/// WebSocket connection manager
pub struct WebSocketManager {
    stream: Arc<Mutex<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    url: Url,
    created_at: Instant,
    last_active: Arc<RwLock<Instant>>,
    message_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
    is_connected: Arc<RwLock<bool>>,
    reconnect_attempts: Arc<AtomicU64>,
}

impl WebSocketManager {
    /// Connect to WebSocket endpoint
    pub async fn connect(url: String) -> Result<Self> {
        let url = Url::parse(&url)?;
        let (ws_stream, _) = connect_async(&url).await?;
        
        info!("WebSocket connected to {}", url);
        
        Ok(Self {
            stream: Arc::new(Mutex::new(Some(ws_stream))),
            url,
            created_at: Instant::now(),
            last_active: Arc::new(RwLock::new(Instant::now())),
            message_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            is_connected: Arc::new(RwLock::new(true)),
            reconnect_attempts: Arc::new(AtomicU64::new(0)),
        })
    }
    
    /// Send message through WebSocket
    pub async fn send_message(&self, msg: Message) -> Result<()> {
        let mut stream_guard = self.stream.lock().await;
        
        if let Some(stream) = stream_guard.as_mut() {
            stream.send(msg).await?;
            self.message_count.fetch_add(1, Ordering::Relaxed);
            *self.last_active.write().await = Instant::now();
            Ok(())
        } else {
            Err(anyhow::anyhow!("WebSocket not connected"))
        }
    }
    
    /// Receive message from WebSocket
    pub async fn receive_message(&self) -> Result<Option<Message>> {
        let mut stream_guard = self.stream.lock().await;
        
        if let Some(stream) = stream_guard.as_mut() {
            match stream.next().await {
                Some(Ok(msg)) => {
                    *self.last_active.write().await = Instant::now();
                    Ok(Some(msg))
                }
                Some(Err(e)) => {
                    self.error_count.fetch_add(1, Ordering::Relaxed);
                    Err(e.into())
                }
                None => Ok(None),
            }
        } else {
            Err(anyhow::anyhow!("WebSocket not connected"))
        }
    }
    
    /// Health check with ping/pong
    pub async fn health_check(&self) -> Result<()> {
        // Send ping
        self.send_message(Message::Ping(vec![])).await?;
        
        // Wait for pong with timeout
        let start = Instant::now();
        let timeout = Duration::from_secs(5);
        
        while start.elapsed() < timeout {
            if let Ok(Some(msg)) = self.receive_message().await {
                if matches!(msg, Message::Pong(_)) {
                    debug!("WebSocket health check passed");
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        warn!("WebSocket health check timeout");
        Err(anyhow::anyhow!("Health check timeout"))
    }
    
    /// Reconnect to WebSocket
    pub async fn reconnect(&self) -> Result<()> {
        *self.is_connected.write().await = false;
        self.reconnect_attempts.fetch_add(1, Ordering::Relaxed);
        
        // Close existing stream
        {
            let mut stream_guard = self.stream.lock().await;
            if let Some(mut stream) = stream_guard.take() {
                let _ = stream.close(None).await;
            }
        }
        
        // Exponential backoff
        let attempts = self.reconnect_attempts.load(Ordering::Relaxed);
        let delay = Duration::from_millis(100 * 2_u64.pow(attempts.min(10) as u32));
        tokio::time::sleep(delay).await;
        
        // Attempt reconnection
        match connect_async(&self.url).await {
            Ok((ws_stream, _)) => {
                *self.stream.lock().await = Some(ws_stream);
                *self.is_connected.write().await = true;
                self.reconnect_attempts.store(0, Ordering::Relaxed);
                info!("WebSocket reconnected to {}", self.url);
                Ok(())
            }
            Err(e) => {
                error!("WebSocket reconnection failed: {}", e);
                Err(e.into())
            }
        }
    }
    
    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }
    
    /// Get connection age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
    
    /// Get last activity time
    pub async fn last_activity(&self) -> Duration {
        self.last_active.read().await.elapsed()
    }
    
    /// Close WebSocket connection
    pub async fn close(&self) -> Result<()> {
        let mut stream_guard = self.stream.lock().await;
        if let Some(mut stream) = stream_guard.take() {
            stream.close(None).await?;
            *self.is_connected.write().await = false;
        }
        Ok(())
    }
}

/// WebSocket connection pool
pub struct WebSocketPool {
    connections: Arc<RwLock<Vec<Arc<WebSocketManager>>>>,
    max_connections: usize,
    url: String,
}

impl WebSocketPool {
    /// Create new WebSocket pool
    pub fn new(url: String, max_connections: usize) -> Self {
        Self {
            connections: Arc::new(RwLock::new(Vec::new())),
            max_connections,
            url,
        }
    }
    
    /// Get or create WebSocket connection
    pub async fn get_connection(&self) -> Result<Arc<WebSocketManager>> {
        let mut connections = self.connections.write().await;
        
        // Find healthy connection
        for conn in connections.iter() {
            if conn.is_connected().await {
                if conn.last_activity().await < Duration::from_secs(60) {
                    return Ok(conn.clone());
                }
            }
        }
        
        // Create new connection if under limit
        if connections.len() < self.max_connections {
            let new_conn = Arc::new(WebSocketManager::connect(self.url.clone()).await?);
            connections.push(new_conn.clone());
            return Ok(new_conn);
        }
        
        // Try to reconnect an existing connection
        if let Some(conn) = connections.first() {
            if !conn.is_connected().await {
                conn.reconnect().await?;
            }
            return Ok(conn.clone());
        }
        
        Err(anyhow::anyhow!("No available WebSocket connections"))
    }
    
    /// Clean up idle connections
    pub async fn cleanup(&self) {
        let mut connections = self.connections.write().await;
        let mut to_remove = Vec::new();
        
        for (idx, conn) in connections.iter().enumerate() {
            // Remove if idle for too long or disconnected
            if conn.last_activity().await > Duration::from_secs(300) {
                to_remove.push(idx);
            } else if !conn.is_connected().await && conn.reconnect_attempts.load(Ordering::Relaxed) > 5 {
                to_remove.push(idx);
            }
        }
        
        // Remove in reverse order to maintain indices
        for idx in to_remove.iter().rev() {
            connections.remove(*idx);
        }
    }
    
    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        let connections = self.connections.read().await;
        let total = connections.len();
        let connected = connections.iter()
            .filter(|c| {
                let c = c.clone();
                tokio::task::block_in_place(move || {
                    tokio::runtime::Handle::current().block_on(c.is_connected())
                })
            })
            .count();
        
        PoolStats {
            total_connections: total,
            active_connections: connected,
            idle_connections: total - connected,
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_websocket_pool_creation() {
        let pool = WebSocketPool::new("ws://localhost:8080".to_string(), 10);
        let stats = pool.stats().await;
        assert_eq!(stats.total_connections, 0);
    }
    
    // Note: Actual WebSocket tests require a running server
    #[tokio::test]
    #[ignore]
    async fn test_websocket_connection() {
        let manager = WebSocketManager::connect("ws://echo.websocket.org".to_string()).await;
        assert!(manager.is_ok());
        
        let manager = manager.unwrap();
        assert!(manager.is_connected().await);
        
        // Test ping/pong
        let health = manager.health_check().await;
        assert!(health.is_ok());
    }
}
