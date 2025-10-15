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

#[cfg(unix)] 
use super::shared_memory_complete::SharedMemoryStream;
#[cfg(windows)]
use super::windows_shared_memory::SharedMemoryStream;

use super::binary_codec::{MessageType, BinaryCodec, Message, MessageEnvelope, HEADER_SIZE};
use super::ipc_messages::MessageType as IpcMessageType;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// IPC Client for connecting to IpcServer
#[derive(Clone)]
pub struct IpcClient {
    /// Shared memory stream for bidirectional communication
    stream: Arc<tokio::sync::Mutex<SharedMemoryStream>>,
    
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
        
        // Use SharedMemoryStream to properly connect via lock-file mechanism
        let stream = SharedMemoryStream::connect(&socket_path).await
            .context("Failed to connect to IPC server")?;
        
        let conn_id = stream.conn_id();
        eprintln!("[CLIENT connect()] Created stream with conn_id={}", conn_id);
        
        let client = Self {
            stream: Arc::new(tokio::sync::Mutex::new(stream)),
            stats: Arc::new(IpcClientStats::default()),
            connected: Arc::new(AtomicBool::new(true)),
            socket_path: socket_path.clone(),
        };
        
        info!("IPC client connected successfully with conn_id={}", conn_id);
        Ok(client)
    }
    
    /// Send raw bytes and wait for response
    pub async fn send_bytes(&self, data: &[u8]) -> Result<Vec<u8>> {
        eprintln!("[CLIENT send_bytes] Starting, {} bytes", data.len());
        if !self.connected.load(Ordering::Acquire) {
            bail!("Client not connected");
        }
        
        let start = Instant::now();
        let msg_size = data.len();
        
        // Lock stream for exclusive access
        eprintln!("[CLIENT send_bytes] Locking stream...");
        let mut stream = self.stream.lock().await;
        eprintln!("[CLIENT send_bytes] Got stream lock");
        
        // Send to server
        eprintln!("[CLIENT send_bytes] Calling write_all...");
        stream.write_all(data).await
            .context("Failed to write message")?;
        eprintln!("[CLIENT send_bytes] Calling flush...");
        stream.flush().await
            .context("Failed to flush")?;
        eprintln!("[CLIENT send_bytes] Write complete");
        
        self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_sent.fetch_add(msg_size as u64, Ordering::Relaxed);
        
        // Read response with timeout
        eprintln!("[CLIENT send_bytes] Reading response...");
        let mut response_data = vec![0u8; 1024 * 1024]; // 1MB buffer
        let n = timeout(
            Duration::from_secs(30),
            stream.read(&mut response_data)
        ).await
            .map_err(|_| anyhow::anyhow!("Request timeout"))?
            .context("Failed to read response")?;
        
        eprintln!("[CLIENT send_bytes] Read {} bytes", n);
        if n == 0 {
            bail!("No data received (connection closed)");
        }
        
        response_data.truncate(n);
        
        self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_received.fetch_add(n as u64, Ordering::Relaxed);
        
        // Track latency
        let latency_us = start.elapsed().as_micros() as u64;
        self.stats.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        self.stats.message_count.fetch_add(1, Ordering::Relaxed);
        
        Ok(response_data)
    }
    
    /// Get client statistics
    pub fn stats(&self) -> &IpcClientStats {
        &self.stats
    }
    
    /// Check if client is connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
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
