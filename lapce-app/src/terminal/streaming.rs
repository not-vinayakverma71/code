// Terminal Pre-IPC: Output streaming with chunking and backpressure
// Part of HP2: Output Streaming feature

use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::time::{Duration, Instant};
use anyhow::Result;

/// Maximum buffer size per terminal (10MB)
const MAX_BUFFER_SIZE: usize = 10 * 1024 * 1024;

/// Chunk size for streaming output (64KB)
const CHUNK_SIZE: usize = 64 * 1024;

/// Bounded channel capacity (prevents unbounded memory growth)
const CHANNEL_CAPACITY: usize = 100;

/// Output chunk with metadata
#[derive(Debug, Clone)]
pub struct OutputChunk {
    /// Chunk data
    pub data: Vec<u8>,
    
    /// Sequence number for ordering
    pub sequence: u64,
    
    /// Timestamp when chunk was created
    pub timestamp: Instant,
    
    /// Whether this is the last chunk in a sequence
    pub is_final: bool,
}

impl OutputChunk {
    /// Create a new output chunk
    pub fn new(data: Vec<u8>, sequence: u64, is_final: bool) -> Self {
        Self {
            data,
            sequence,
            timestamp: Instant::now(),
            is_final,
        }
    }
    
    /// Get chunk size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
    
    /// Get age of chunk
    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// Output stream with chunking and backpressure
pub struct OutputStream {
    /// Sender for output chunks (bounded)
    sender: SyncSender<OutputChunk>,
    
    /// Current sequence number
    sequence: u64,
    
    /// Total bytes buffered
    buffered_bytes: usize,
    
    /// Statistics
    stats: StreamStats,
}

impl OutputStream {
    /// Create a new output stream
    pub fn new(capacity: usize) -> (Self, Receiver<OutputChunk>) {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        
        let stream = Self {
            sender,
            sequence: 0,
            buffered_bytes: 0,
            stats: StreamStats::default(),
        };
        
        (stream, receiver)
    }
    
    /// Create with default capacity
    pub fn with_default_capacity() -> (Self, Receiver<OutputChunk>) {
        Self::new(CHANNEL_CAPACITY)
    }
    
    /// Send data, chunking if necessary
    pub fn send(&mut self, data: Vec<u8>) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        // Check if total buffer size would exceed limit
        if self.buffered_bytes + data.len() > MAX_BUFFER_SIZE {
            self.stats.dropped_bytes += data.len();
            return Err(anyhow::anyhow!(
                "Buffer size limit exceeded: {} + {} > {}",
                self.buffered_bytes,
                data.len(),
                MAX_BUFFER_SIZE
            ));
        }
        
        // Chunk large data
        if data.len() > CHUNK_SIZE {
            self.send_chunked(data)?;
        } else {
            self.send_single(data)?;
        }
        
        Ok(())
    }
    
    /// Send data as a single chunk
    fn send_single(&mut self, data: Vec<u8>) -> Result<()> {
        let size = data.len();
        let chunk = OutputChunk::new(data, self.sequence, true);
        self.sequence += 1;
        
        // Try to send with backpressure handling
        match self.sender.try_send(chunk) {
            Ok(_) => {
                self.buffered_bytes += size;
                self.stats.chunks_sent += 1;
                self.stats.bytes_sent += size;
                Ok(())
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.stats.backpressure_events += 1;
                // Block until space available (applies backpressure)
                self.sender.send(OutputChunk::new(
                    Vec::new(),
                    self.sequence - 1,
                    true,
                ))?;
                Err(anyhow::anyhow!("Channel full, backpressure applied"))
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                Err(anyhow::anyhow!("Channel disconnected"))
            }
        }
    }
    
    /// Send data as multiple chunks
    fn send_chunked(&mut self, data: Vec<u8>) -> Result<()> {
        let total_size = data.len();
        let chunks = data.chunks(CHUNK_SIZE);
        let chunk_count = chunks.len();
        
        for (i, chunk_data) in chunks.enumerate() {
            let is_final = i == chunk_count - 1;
            let chunk = OutputChunk::new(
                chunk_data.to_vec(),
                self.sequence,
                is_final,
            );
            
            self.sender.send(chunk)?;
            self.buffered_bytes += chunk_data.len();
            self.stats.chunks_sent += 1;
            
            if is_final {
                self.sequence += 1;
            }
        }
        
        self.stats.bytes_sent += total_size;
        Ok(())
    }
    
    /// Mark bytes as consumed (reduce buffer count)
    pub fn mark_consumed(&mut self, bytes: usize) {
        self.buffered_bytes = self.buffered_bytes.saturating_sub(bytes);
        self.stats.bytes_consumed += bytes;
    }
    
    /// Get current buffered bytes
    pub fn buffered_bytes(&self) -> usize {
        self.buffered_bytes
    }
    
    /// Get statistics
    pub fn stats(&self) -> &StreamStats {
        &self.stats
    }
    
    /// Check if stream is healthy (not experiencing excessive backpressure)
    pub fn is_healthy(&self) -> bool {
        self.buffered_bytes < MAX_BUFFER_SIZE / 2
            && self.stats.backpressure_events < 10
    }
}

/// Streaming statistics
#[derive(Debug, Clone, Default)]
pub struct StreamStats {
    /// Total chunks sent
    pub chunks_sent: u64,
    
    /// Total bytes sent
    pub bytes_sent: usize,
    
    /// Total bytes consumed
    pub bytes_consumed: usize,
    
    /// Backpressure events (channel full)
    pub backpressure_events: u64,
    
    /// Dropped bytes (buffer limit)
    pub dropped_bytes: usize,
}

/// Output consumer that reads from stream
pub struct OutputConsumer {
    /// Receiver for chunks
    receiver: Receiver<OutputChunk>,
    
    /// Buffer for reassembling chunks
    buffer: Vec<u8>,
    
    /// Current sequence being assembled
    current_sequence: Option<u64>,
}

impl OutputConsumer {
    /// Create a new consumer
    pub fn new(receiver: Receiver<OutputChunk>) -> Self {
        Self {
            receiver,
            buffer: Vec::new(),
            current_sequence: None,
        }
    }
    
    /// Try to receive next complete output (non-blocking)
    pub fn try_recv(&mut self) -> Result<Option<Vec<u8>>, TryRecvError> {
        loop {
            match self.receiver.try_recv() {
                Ok(chunk) => {
                    // Check if this is a new sequence
                    if self.current_sequence != Some(chunk.sequence) {
                        // Clear buffer for new sequence
                        self.buffer.clear();
                        self.current_sequence = Some(chunk.sequence);
                    }
                    
                    // Append chunk data
                    self.buffer.extend_from_slice(&chunk.data);
                    
                    // If final chunk, return complete output
                    if chunk.is_final {
                        self.current_sequence = None;
                        return Ok(Some(std::mem::take(&mut self.buffer)));
                    }
                }
                Err(TryRecvError::Empty) => {
                    return Ok(None);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
    
    /// Blocking receive of next complete output
    pub fn recv(&mut self) -> Result<Vec<u8>> {
        loop {
            let chunk = self.receiver.recv()?;
            
            // Check if this is a new sequence
            if self.current_sequence != Some(chunk.sequence) {
                self.buffer.clear();
                self.current_sequence = Some(chunk.sequence);
            }
            
            // Append chunk data
            self.buffer.extend_from_slice(&chunk.data);
            
            // If final chunk, return complete output
            if chunk.is_final {
                self.current_sequence = None;
                return Ok(std::mem::take(&mut self.buffer));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_output_chunk_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let chunk = OutputChunk::new(data.clone(), 0, true);
        
        assert_eq!(chunk.data, data);
        assert_eq!(chunk.sequence, 0);
        assert!(chunk.is_final);
        assert_eq!(chunk.size(), 5);
    }
    
    #[test]
    fn test_output_stream_single_chunk() {
        let (mut stream, receiver) = OutputStream::with_default_capacity();
        let mut consumer = OutputConsumer::new(receiver);
        
        let data = vec![1, 2, 3, 4, 5];
        stream.send(data.clone()).unwrap();
        
        let received = consumer.try_recv().unwrap().unwrap();
        assert_eq!(received, data);
        assert_eq!(stream.stats().chunks_sent, 1);
        assert_eq!(stream.stats().bytes_sent, 5);
    }
    
    #[test]
    fn test_output_stream_chunking() {
        let (mut stream, receiver) = OutputStream::with_default_capacity();
        let mut consumer = OutputConsumer::new(receiver);
        
        // Create data larger than CHUNK_SIZE
        let large_data = vec![42u8; CHUNK_SIZE * 2 + 100];
        stream.send(large_data.clone()).unwrap();
        
        let received = consumer.recv().unwrap();
        assert_eq!(received, large_data);
        
        // Should have been sent as 3 chunks
        assert_eq!(stream.stats().chunks_sent, 3);
        assert_eq!(stream.stats().bytes_sent, large_data.len());
    }
    
    #[test]
    fn test_output_stream_buffer_limit() {
        let (mut stream, _receiver) = OutputStream::with_default_capacity();
        
        // Try to send more than MAX_BUFFER_SIZE
        let huge_data = vec![0u8; MAX_BUFFER_SIZE + 1];
        let result = stream.send(huge_data);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Buffer size limit exceeded"));
    }
    
    #[test]
    fn test_output_stream_mark_consumed() {
        let (mut stream, _receiver) = OutputStream::with_default_capacity();
        
        stream.send(vec![1, 2, 3]).unwrap();
        assert_eq!(stream.buffered_bytes(), 3);
        
        stream.mark_consumed(2);
        assert_eq!(stream.buffered_bytes(), 1);
        
        stream.mark_consumed(5); // More than available
        assert_eq!(stream.buffered_bytes(), 0);
    }
    
    #[test]
    fn test_output_stream_health_check() {
        let (mut stream, _receiver) = OutputStream::with_default_capacity();
        
        assert!(stream.is_healthy());
        
        // Add some data
        stream.send(vec![0u8; 1024]).unwrap();
        assert!(stream.is_healthy());
        
        // Simulate backpressure events
        stream.stats.backpressure_events = 20;
        assert!(!stream.is_healthy());
    }
    
    #[test]
    fn test_consumer_reassembly() {
        let (mut stream, receiver) = OutputStream::with_default_capacity();
        let mut consumer = OutputConsumer::new(receiver);
        
        // Send multiple chunks manually
        let chunk1 = OutputChunk::new(vec![1, 2], 0, false);
        let chunk2 = OutputChunk::new(vec![3, 4], 0, false);
        let chunk3 = OutputChunk::new(vec![5, 6], 0, true);
        
        stream.sender.send(chunk1).unwrap();
        stream.sender.send(chunk2).unwrap();
        stream.sender.send(chunk3).unwrap();
        
        let received = consumer.recv().unwrap();
        assert_eq!(received, vec![1, 2, 3, 4, 5, 6]);
    }
    
    #[test]
    fn test_consumer_try_recv_empty() {
        let (_stream, receiver) = OutputStream::with_default_capacity();
        let mut consumer = OutputConsumer::new(receiver);
        
        let result = consumer.try_recv();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
    
    #[test]
    fn test_multiple_sequences() {
        let (mut stream, receiver) = OutputStream::with_default_capacity();
        let mut consumer = OutputConsumer::new(receiver);
        
        stream.send(vec![1, 2, 3]).unwrap();
        stream.send(vec![4, 5, 6]).unwrap();
        
        let first = consumer.try_recv().unwrap().unwrap();
        assert_eq!(first, vec![1, 2, 3]);
        
        let second = consumer.try_recv().unwrap().unwrap();
        assert_eq!(second, vec![4, 5, 6]);
    }
    
    #[test]
    fn test_stream_statistics() {
        let (mut stream, _receiver) = OutputStream::with_default_capacity();
        
        stream.send(vec![1, 2, 3]).unwrap();
        stream.send(vec![4, 5]).unwrap();
        
        let stats = stream.stats();
        assert_eq!(stats.chunks_sent, 2);
        assert_eq!(stats.bytes_sent, 5);
        assert_eq!(stats.backpressure_events, 0);
        assert_eq!(stats.dropped_bytes, 0);
        
        stream.mark_consumed(5);
        assert_eq!(stream.stats().bytes_consumed, 5);
    }
}
