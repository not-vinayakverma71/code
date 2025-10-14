/// Optimized shared memory listener using worker pool and SPSC rings
/// Designed for ≥1M msg/s throughput with channel bridge to Tokio

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, bail};

use crate::ipc::spsc_shm_ring::{SpscRing, RingHeader};
use crate::ipc::shm_io_workers::{ShmWorkerPool, WorkerMessage};
use crate::ipc::shm_waiter_cross_os::ShmWaiter;
use crate::ipc::shm_metrics_optimized::OptimizedMetricsCollector;
use crate::ipc::shm_namespace::create_namespaced_path;

/// Configuration for optimized listener
#[derive(Clone)]
pub struct OptimizedListenerConfig {
    /// Number of worker threads (recommended: num_cores / 4)
    pub num_workers: usize,
    /// Ring buffer size per connection (default: 2MB)
    pub ring_size: usize,
    /// Whether to pin workers to CPU cores
    pub pin_cores: bool,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Base path for shared memory objects
    pub base_path: String,
}

impl Default for OptimizedListenerConfig {
    fn default() -> Self {
        let num_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            num_workers: (num_cores / 4).max(1),
            ring_size: 2 * 1024 * 1024, // 2MB
            pin_cores: true,
            enable_metrics: true,
            base_path: "/tmp/lapce_ipc".to_string(),
        }
    }
}

/// Optimized shared memory listener
pub struct OptimizedShmListener {
    config: OptimizedListenerConfig,
    worker_pool: ShmWorkerPool,
    metrics: OptimizedMetricsCollector,
    active_connections: HashMap<u64, ConnectionInfo>,
    next_conn_id: u64,
}

struct ConnectionInfo {
    send_ring: Arc<SpscRing>,
    recv_ring: Arc<SpscRing>,
    waiter: Arc<ShmWaiter>,
}

impl OptimizedShmListener {
    /// Create and bind a new optimized listener
    pub async fn bind(config: OptimizedListenerConfig) -> Result<Self> {
        // Create worker rings
        let rings = Self::create_worker_rings(&config)?;
        
        // Create worker pool with optional CPU pinning
        let worker_pool = ShmWorkerPool::new(
            config.num_workers,
            rings,
            config.pin_cores,
        )?;
        
        // Create metrics collector
        let metrics = if config.enable_metrics {
            OptimizedMetricsCollector::default()
        } else {
            let mut m = OptimizedMetricsCollector::default();
            m.disable();
            m
        };
        
        Ok(Self {
            config,
            worker_pool,
            metrics,
            active_connections: HashMap::new(),
            next_conn_id: 1,
        })
    }
    
    /// Create SPSC rings for workers
    fn create_worker_rings(config: &OptimizedListenerConfig) -> Result<Vec<Arc<SpscRing>>> {
        use std::alloc::{alloc_zeroed, Layout};
        use std::ptr;
        
        let mut rings = Vec::new();
        
        for worker_id in 0..config.num_workers {
            unsafe {
                let header_layout = Layout::new::<RingHeader>();
                let data_layout = Layout::from_size_align(config.ring_size, 64)
                    .map_err(|e| anyhow::anyhow!("Invalid layout: {}", e))?;
                
                let header = alloc_zeroed(header_layout) as *mut RingHeader;
                let data = alloc_zeroed(data_layout);
                
                // Pre-touch pages
                for offset in (0..config.ring_size).step_by(4096) {
                    ptr::write_volatile(data.add(offset), 0);
                }
                
                let ring = Arc::new(SpscRing::from_raw(header, data, config.ring_size));
                rings.push(ring);
            }
        }
        
        Ok(rings)
    }
    
    /// Accept a new connection (creates connection rings)
    pub async fn accept(&mut self) -> Result<OptimizedConnection> {
        let conn_id = self.next_conn_id;
        self.next_conn_id += 1;
        
        // Create connection-specific rings
        let (send_ring, recv_ring) = Self::create_connection_rings(&self.config, conn_id)?;
        
        let waiter = Arc::new(ShmWaiter::new()?);
        
        // Store connection info
        self.active_connections.insert(conn_id, ConnectionInfo {
            send_ring: send_ring.clone(),
            recv_ring: recv_ring.clone(),
            waiter: waiter.clone(),
        });
        
        Ok(OptimizedConnection {
            conn_id,
            send_ring,
            recv_ring,
            waiter,
            metrics: self.metrics.clone(),
        })
    }
    
    /// Create rings for a specific connection
    fn create_connection_rings(
        config: &OptimizedListenerConfig,
        conn_id: u64,
    ) -> Result<(Arc<SpscRing>, Arc<SpscRing>)> {
        use std::alloc::{alloc_zeroed, Layout};
        
        unsafe {
            let header_layout = Layout::new::<RingHeader>();
            let data_layout = Layout::from_size_align(config.ring_size, 64)
                .map_err(|e| anyhow::anyhow!("Invalid layout: {}", e))?;
            
            // Send ring (server → client)
            let send_header = alloc_zeroed(header_layout) as *mut RingHeader;
            let send_data = alloc_zeroed(data_layout);
            let send_ring = Arc::new(SpscRing::from_raw(send_header, send_data, config.ring_size));
            
            // Recv ring (client → server)
            let recv_header = alloc_zeroed(header_layout) as *mut RingHeader;
            let recv_data = alloc_zeroed(data_layout);
            let recv_ring = Arc::new(SpscRing::from_raw(recv_header, recv_data, config.ring_size));
            
            Ok((send_ring, recv_ring))
        }
    }
    
    /// Receive message from any worker
    pub async fn recv(&self) -> Result<WorkerMessage> {
        let msg = self.worker_pool.recv_async().await?;
        self.metrics.record_read(msg.data.len());
        Ok(msg)
    }
    
    /// Get connection by ID
    pub fn get_connection(&self, conn_id: u64) -> Option<&ConnectionInfo> {
        self.active_connections.get(&conn_id)
    }
    
    /// Remove connection
    pub fn remove_connection(&mut self, conn_id: u64) -> Option<ConnectionInfo> {
        self.active_connections.remove(&conn_id)
    }
    
    /// Get metrics collector
    pub fn metrics(&self) -> &OptimizedMetricsCollector {
        &self.metrics
    }
    
    /// Shutdown listener
    pub async fn shutdown(self) -> Result<()> {
        self.worker_pool.shutdown()
    }
}

/// Optimized connection handle
pub struct OptimizedConnection {
    conn_id: u64,
    send_ring: Arc<SpscRing>,
    recv_ring: Arc<SpscRing>,
    waiter: Arc<ShmWaiter>,
    metrics: OptimizedMetricsCollector,
}

impl OptimizedConnection {
    /// Get connection ID
    pub fn conn_id(&self) -> u64 {
        self.conn_id
    }
    
    /// Write message to client
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        // Try write with retries
        let mut retries = 0;
        while !self.send_ring.try_write(data) {
            if retries > 1000 {
                bail!("Write timeout: ring full");
            }
            tokio::task::yield_now().await;
            retries += 1;
        }
        
        // Wake client
        self.waiter.wake_one(self.send_ring.write_seq_ptr());
        self.metrics.record_write(data.len());
        
        Ok(())
    }
    
    /// Write batch of messages
    pub async fn write_batch(&self, messages: &[&[u8]]) -> Result<usize> {
        let written = self.send_ring.try_write_batch(messages, messages.len());
        
        if written > 0 {
            self.waiter.wake_one(self.send_ring.write_seq_ptr());
            for msg in &messages[..written] {
                self.metrics.record_write(msg.len());
            }
        }
        
        Ok(written)
    }
    
    /// Read message from client
    pub async fn read(&self) -> Result<Vec<u8>> {
        loop {
            if let Some(data) = self.recv_ring.try_read() {
                self.metrics.record_read(data.len());
                return Ok(data);
            }
            
            // Wait for data
            tokio::task::yield_now().await;
        }
    }
    
    /// Read batch of messages
    pub async fn read_batch(&self, max: usize) -> Vec<Vec<u8>> {
        let messages = self.recv_ring.try_read_batch(max);
        for msg in &messages {
            self.metrics.record_read(msg.len());
        }
        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_optimized_listener_bind() {
        let config = OptimizedListenerConfig {
            num_workers: 2,
            pin_cores: false, // Don't pin in tests
            ..Default::default()
        };
        
        let listener = OptimizedShmListener::bind(config).await.unwrap();
        assert_eq!(listener.worker_pool.num_workers(), 2);
    }
    
    #[tokio::test]
    async fn test_connection_creation() {
        let config = OptimizedListenerConfig {
            num_workers: 1,
            pin_cores: false,
            ..Default::default()
        };
        
        let mut listener = OptimizedShmListener::bind(config).await.unwrap();
        
        let conn = listener.accept().await.unwrap();
        assert_eq!(conn.conn_id(), 1);
        
        let conn2 = listener.accept().await.unwrap();
        assert_eq!(conn2.conn_id(), 2);
    }
    
    #[tokio::test]
    async fn test_connection_write_read() {
        let config = OptimizedListenerConfig {
            num_workers: 1,
            pin_cores: false,
            ..Default::default()
        };
        
        let mut listener = OptimizedShmListener::bind(config).await.unwrap();
        let conn = listener.accept().await.unwrap();
        
        let msg = b"Hello from server";
        conn.write(msg).await.unwrap();
        
        // Simulate client read
        let read_msg = conn.send_ring.try_read().unwrap();
        assert_eq!(read_msg, msg);
    }
}
