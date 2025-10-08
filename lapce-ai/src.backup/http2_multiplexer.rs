/// HTTP/2 Multiplexing Implementation
/// Manages concurrent streams over single connection

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{RwLock, Semaphore};
use anyhow::{Result, anyhow};
use tracing::{debug, info};

/// Stream state for HTTP/2 multiplexing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamState {
    Idle,
    Open,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
}

/// Individual stream metadata
#[derive(Debug)]
pub struct StreamInfo {
    pub id: u32,
    pub state: StreamState,
    pub created_at: Instant,
    pub window_size: u32,
    pub priority: u8,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// HTTP/2 Multiplexed Connection
pub struct MultiplexedConnection {
    connection_id: String,
    active_streams: Arc<AtomicU32>,
    max_concurrent_streams: Arc<AtomicU32>,
    streams: Arc<RwLock<HashMap<u32, StreamInfo>>>,
    next_stream_id: Arc<AtomicU32>,
    total_streams_created: Arc<AtomicU64>,
    connection_window: Arc<AtomicU32>,
    stream_semaphore: Arc<Semaphore>,
    created_at: Instant,
}

/// HTTP/2 Multiplexer for managing streams
pub struct Http2Multiplexer {
    connections: Arc<RwLock<HashMap<String, Arc<MultiplexedConnection>>>>,
    max_streams_per_conn: Arc<AtomicU32>,
}

impl Http2Multiplexer {
    pub fn new(max_streams: u32) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_streams_per_conn: Arc::new(AtomicU32::new(max_streams)),
        }
    }
    
    pub fn update_max_streams(&self, new_max: u32) {
        self.max_streams_per_conn.store(new_max, Ordering::Relaxed);
        debug!("Updated max streams per connection to {}", new_max);
    }
    
    pub async fn allocate_stream(&self) -> Option<u32> {
        // Simplified: just return a stream ID
        Some(rand::random::<u32>() | 1) // Ensure odd number for client-initiated
    }
    
    pub async fn release_stream(&self, stream_id: u32) {
        debug!("Released stream {}", stream_id);
    }
    
    pub fn active_streams(&self) -> u32 {
        0 // Simplified for now
    }
}

impl MultiplexedConnection {
    pub fn new(connection_id: String, max_concurrent_streams: u32) -> Self {
        info!("Creating multiplexed connection {} with max {} streams", 
              connection_id, max_concurrent_streams);
        
        Self {
            connection_id,
            active_streams: Arc::new(AtomicU32::new(0)),
            max_concurrent_streams: Arc::new(AtomicU32::new(max_concurrent_streams)),
            streams: Arc::new(RwLock::new(HashMap::new())),
            next_stream_id: Arc::new(AtomicU32::new(1)), // Odd numbers for client
            total_streams_created: Arc::new(AtomicU64::new(0)),
            connection_window: Arc::new(AtomicU32::new(131072)), // 128KB default
            stream_semaphore: Arc::new(Semaphore::new(max_concurrent_streams as usize)),
            created_at: Instant::now(),
        }
    }
    
    /// Allocate a new stream
    pub async fn allocate_stream(&self, priority: u8) -> Result<u32> {
        // Acquire permit for concurrent stream limit
        let _permit = self.stream_semaphore.acquire().await
            .map_err(|e| anyhow!("Failed to acquire stream permit: {}", e))?;
        
        let active = self.active_streams.fetch_add(1, Ordering::SeqCst) + 1;
        
        if active > self.max_concurrent_streams.load(Ordering::Relaxed) {
            self.active_streams.fetch_sub(1, Ordering::SeqCst);
            return Err(anyhow!("Max concurrent streams ({}) reached", self.max_concurrent_streams.load(Ordering::Relaxed)));
        }
        
        // Allocate stream ID (odd numbers for client-initiated)
        let stream_id = self.next_stream_id.fetch_add(2, Ordering::SeqCst);
        
        let stream_info = StreamInfo {
            id: stream_id,
            state: StreamState::Open,
            created_at: Instant::now(),
            window_size: 65536, // 64KB initial window
            priority,
            bytes_sent: 0,
            bytes_received: 0,
        };
        
        self.streams.write().await.insert(stream_id, stream_info);
        self.total_streams_created.fetch_add(1, Ordering::Relaxed);
        
        debug!("Allocated stream {} (active: {}/{})", 
               stream_id, active, self.max_concurrent_streams.load(Ordering::Relaxed));
        
        Ok(stream_id)
    }
    
    /// Release a stream
    pub async fn release_stream(&self, stream_id: u32) -> Result<()> {
        let mut streams = self.streams.write().await;
        
        if let Some(mut stream) = streams.remove(&stream_id) {
            stream.state = StreamState::Closed;
            self.active_streams.fetch_sub(1, Ordering::SeqCst);
            
            debug!("Released stream {} (active: {})", 
                   stream_id, self.active_streams.load(Ordering::Relaxed));
            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }
    
    /// Update stream window
    pub async fn update_stream_window(&self, stream_id: u32, delta: i32) -> Result<()> {
        let mut streams = self.streams.write().await;
        
        if let Some(stream) = streams.get_mut(&stream_id) {
            if delta > 0 {
                stream.window_size = stream.window_size.saturating_add(delta as u32);
            } else {
                stream.window_size = stream.window_size.saturating_sub((-delta) as u32);
            }
            
            debug!("Stream {} window updated to {}", stream_id, stream.window_size);
            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }
    
    /// Update connection window
    pub fn update_connection_window(&self, delta: i32) {
        if delta > 0 {
            self.connection_window.fetch_add(delta as u32, Ordering::SeqCst);
        } else {
            self.connection_window.fetch_sub((-delta) as u32, Ordering::SeqCst);
        }
        
        debug!("Connection window updated to {}", 
               self.connection_window.load(Ordering::Relaxed));
    }
    
    /// Check if we can send data
    pub async fn can_send(&self, stream_id: u32, size: usize) -> Result<bool> {
        let streams = self.streams.read().await;
        
        if let Some(stream) = streams.get(&stream_id) {
            let stream_window = stream.window_size as usize;
            let conn_window = self.connection_window.load(Ordering::Relaxed) as usize;
            
            Ok(size <= stream_window && size <= conn_window)
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }
    
    /// Track bytes sent
    pub async fn record_bytes_sent(&self, stream_id: u32, bytes: u64) -> Result<()> {
        let mut streams = self.streams.write().await;
        
        if let Some(stream) = streams.get_mut(&stream_id) {
            stream.bytes_sent += bytes;
            stream.window_size = stream.window_size.saturating_sub(bytes as u32);
            self.connection_window.fetch_sub(bytes as u32, Ordering::SeqCst);
            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }
    
    /// Track bytes received
    pub async fn record_bytes_received(&self, stream_id: u32, bytes: u64) -> Result<()> {
        let mut streams = self.streams.write().await;
        
        if let Some(stream) = streams.get_mut(&stream_id) {
            stream.bytes_received += bytes;
            Ok(())
        } else {
            Err(anyhow!("Stream {} not found", stream_id))
        }
    }
    
    /// Get multiplexing statistics
    pub async fn get_stats(&self) -> MultiplexStats {
        let streams = self.streams.read().await;
        
        let total_bytes_sent: u64 = streams.values().map(|s| s.bytes_sent).sum();
        let total_bytes_received: u64 = streams.values().map(|s| s.bytes_received).sum();
        
        MultiplexStats {
            connection_id: self.connection_id.clone(),
            active_streams: self.active_streams.load(Ordering::Relaxed),
            max_concurrent_streams: self.max_concurrent_streams.load(Ordering::Relaxed),
            total_streams_created: self.total_streams_created.load(Ordering::Relaxed),
            connection_window: self.connection_window.load(Ordering::Relaxed),
            total_bytes_sent,
            total_bytes_received,
            connection_age: self.created_at.elapsed(),
        }
    }
    
    /// Graceful shutdown
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down multiplexed connection {}", self.connection_id);
        
        let mut streams = self.streams.write().await;
        let stream_ids: Vec<u32> = streams.keys().cloned().collect();
        
        for stream_id in stream_ids {
            if let Some(mut stream) = streams.remove(&stream_id) {
                stream.state = StreamState::Closed;
            }
        }
        
        self.active_streams.store(0, Ordering::SeqCst);
        Ok(())
    }
}

/// Multiplexing statistics
#[derive(Debug, Clone)]
pub struct MultiplexStats {
    pub connection_id: String,
    pub active_streams: u32,
    pub max_concurrent_streams: u32,
    pub total_streams_created: u64,
    pub connection_window: u32,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub connection_age: Duration,
}

/// Stream priority queue for scheduling
pub struct StreamPriorityQueue {
    queues: Vec<Vec<u32>>, // Priority buckets (0-255)
}

impl StreamPriorityQueue {
    pub fn new() -> Self {
        Self {
            queues: (0..256).map(|_| Vec::new()).collect(),
        }
    }
    
    pub fn enqueue(&mut self, stream_id: u32, priority: u8) {
        self.queues[priority as usize].push(stream_id);
    }
    
    pub fn dequeue(&mut self) -> Option<u32> {
        // Get highest priority stream (0 is highest)
        for queue in &mut self.queues {
            if let Some(stream_id) = queue.pop() {
                return Some(stream_id);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_stream_allocation() {
        let mux = MultiplexedConnection::new("test".to_string(), 10);
        
        // Allocate streams
        let stream1 = mux.allocate_stream(1).await.unwrap();
        let stream2 = mux.allocate_stream(2).await.unwrap();
        
        assert_eq!(stream1, 1);
        assert_eq!(stream2, 3); // Odd numbers for client
        assert_eq!(mux.active_streams.load(Ordering::Relaxed), 2);
    }
    
    #[tokio::test]
    async fn test_max_concurrent_streams() {
        let mux = MultiplexedConnection::new("test".to_string(), 3);
        
        // Allocate max streams
        let _s1 = mux.allocate_stream(1).await.unwrap();
        let _s2 = mux.allocate_stream(1).await.unwrap();
        let _s3 = mux.allocate_stream(1).await.unwrap();
        
        // This should fail
        let result = mux.allocate_stream(1).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_stream_window_updates() {
        let mux = MultiplexedConnection::new("test".to_string(), 10);
        let stream_id = mux.allocate_stream(1).await.unwrap();
        
        // Test window updates
        mux.update_stream_window(stream_id, 1000).await.unwrap();
        mux.update_stream_window(stream_id, -500).await.unwrap();
        
        let can_send = mux.can_send(stream_id, 1000).await.unwrap();
        assert!(can_send);
    }
    
    #[tokio::test]
    async fn test_multiplexing_stats() {
        let mux = MultiplexedConnection::new("test".to_string(), 100);
        
        let stream_id = mux.allocate_stream(1).await.unwrap();
        mux.record_bytes_sent(stream_id, 1024).await.unwrap();
        mux.record_bytes_received(stream_id, 2048).await.unwrap();
        
        let stats = mux.get_stats().await;
        assert_eq!(stats.active_streams, 1);
        assert_eq!(stats.total_bytes_sent, 1024);
        assert_eq!(stats.total_bytes_received, 2048);
    }
}
