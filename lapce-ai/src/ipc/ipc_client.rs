/// IPC Client - connects to IpcServer for round-trip message testing
/// Implements client-side connection, handshake, and message send/receive

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use anyhow::{Result, bail, Context};
use tokio::time::timeout;
use tracing::{info, debug, warn, error};
use rkyv::{Archive, Serialize as RkyvSerialize, Deserialize as RkyvDeserialize};
use rkyv::ser::serializers::AllocSerializer;
use rkyv::validation::validators::DefaultValidator;
use rkyv::CheckBytes;

use super::shared_memory_complete::SharedMemoryBuffer;
use super::binary_codec::{MessageType, BinaryCodec, Message, MessageEnvelope, HEADER_SIZE};
use super::ipc_messages::MessageType as IpcMessageType;

/// IPC Client for connecting to IpcServer
pub struct IpcClient {
    /// Connection ID assigned by server
    conn_id: Option<String>,
    
    /// Client-to-server shared memory buffer
    client_tx: Arc<SharedMemoryBuffer>,
    
    /// Server-to-client shared memory buffer
    client_rx: Arc<SharedMemoryBuffer>,
    
    /// Connection statistics
    stats: Arc<IpcClientStats>,
    
    /// Connected state
    connected: Arc<AtomicBool>,
    
    /// Base socket path
    socket_path: String,
}

/// Client statistics
#[derive(Debug, Default)]
pub struct IpcClientStats {
    pub messages_sent: AtomicU64,
    pub messages_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub errors: AtomicU64,
    pub total_latency_us: AtomicU64,
    pub message_count: AtomicU64,
}

impl IpcClientStats {
    pub fn avg_latency_us(&self) -> f64 {
        let count = self.message_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let total = self.total_latency_us.load(Ordering::Relaxed);
        total as f64 / count as f64
    }
    
    pub fn throughput_msgs_per_sec(&self, elapsed: Duration) -> f64 {
        let count = self.message_count.load(Ordering::Relaxed);
        count as f64 / elapsed.as_secs_f64()
    }
}

impl IpcClient {
    /// Connect to IPC server at the given socket path
    pub async fn connect(socket_path: impl Into<String>) -> Result<Self> {
        let socket_path = socket_path.into();
        info!("IPC client connecting to: {}", socket_path);
        
        // Generate unique connection ID
        let conn_id = uuid::Uuid::new_v4().to_string();
        
        // Create client-side shared memory channels
        // Client writes to server on tx, reads from server on rx
        let client_tx_path = format!("{}_client_{}_tx", socket_path, conn_id);
        let client_rx_path = format!("{}_client_{}_rx", socket_path, conn_id);
        
        debug!("Creating client shared memory: tx={}, rx={}", client_tx_path, client_rx_path);
        
        // Create both channels
        let client_tx = SharedMemoryBuffer::create(&client_tx_path, 1024 * 1024).await
            .context("Failed to create client tx buffer")?;
        let client_rx = SharedMemoryBuffer::create(&client_rx_path, 1024 * 1024).await
            .context("Failed to create client rx buffer")?;
        
        let mut client = Self {
            conn_id: Some(conn_id.clone()),
            client_tx: Arc::new(client_tx),
            client_rx: Arc::new(client_rx),
            stats: Arc::new(IpcClientStats::default()),
            connected: Arc::new(AtomicBool::new(false)),
            socket_path: socket_path.clone(),
        };
        
        // Perform handshake with server
        client.handshake().await?;
        
        info!("IPC client connected: conn_id={}", conn_id);
        Ok(client)
    }
    
    /// Perform connection handshake with server
    async fn handshake(&mut self) -> Result<()> {
        debug!("Starting client handshake");
        
        // For now, just mark as connected - full handshake protocol will be implemented
        // when we integrate with the actual IpcServer handler
        self.connected.store(true, Ordering::Release);
        debug!("Client handshake complete (simplified)");
        Ok(())
    }
    
    /// Send raw bytes and wait for response
    pub async fn send_bytes(&self, data: &[u8]) -> Result<Vec<u8>> {
        if !self.connected.load(Ordering::Acquire) {
            bail!("Client not connected");
        }
        
        let start = Instant::now();
        let msg_size = data.len();
        
        // Send to server
        self.client_tx.write(data).await
            .context("Failed to write message")?;
        
        self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_sent.fetch_add(msg_size as u64, Ordering::Relaxed);
        
        // Wait for response with timeout
        let response_data = timeout(
            Duration::from_secs(30),
            self.client_rx.read()
        ).await
            .map_err(|_| anyhow::anyhow!("Request timeout"))?
            .ok_or_else(|| anyhow::anyhow!("No data received"))?;
        
        self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_received.fetch_add(response_data.len() as u64, Ordering::Relaxed);
        
        // Track latency
        let latency_us = start.elapsed().as_micros() as u64;
        self.stats.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        self.stats.message_count.fetch_add(1, Ordering::Relaxed);
        
        Ok(response_data)
    }
    
    /// Send bytes without waiting for response (fire and forget)
    pub async fn send_oneway(&self, data: &[u8]) -> Result<()> {
        if !self.connected.load(Ordering::Acquire) {
            bail!("Client not connected");
        }
        
        let msg_size = data.len();
        
        self.client_tx.write(data).await?;
        
        self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_sent.fetch_add(msg_size as u64, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Get client statistics
    pub fn stats(&self) -> &IpcClientStats {
        &self.stats
    }
    
    /// Check if client is connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }
    
    /// Get connection ID
    pub fn conn_id(&self) -> Option<&str> {
        self.conn_id.as_deref()
    }
    
    /// Disconnect from server
    pub async fn disconnect(&mut self) -> Result<()> {
        if !self.connected.load(Ordering::Acquire) {
            return Ok(());
        }
        
        info!("Disconnecting IPC client");
        
        // Best effort disconnect - just mark as disconnected
        
        self.connected.store(false, Ordering::Release);
        
        info!("IPC client disconnected");
        Ok(())
    }
}

impl Drop for IpcClient {
    fn drop(&mut self) {
        // Cleanup is automatic via SharedMemoryBuffer Drop
        debug!("IpcClient dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_client_stats() {
        let stats = IpcClientStats::default();
        
        stats.messages_sent.store(100, Ordering::Relaxed);
        stats.total_latency_us.store(10000, Ordering::Relaxed);
        stats.message_count.store(100, Ordering::Relaxed);
        
        assert_eq!(stats.avg_latency_us(), 100.0);
        
        let throughput = stats.throughput_msgs_per_sec(Duration::from_secs(1));
        assert_eq!(throughput, 100.0);
    }
}
