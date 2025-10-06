/// Stream Metrics - Performance monitoring for streaming pipeline
/// Phase 2, Task 10: StreamMetrics
/// Based on docs/08-STREAMING-PIPELINE.md lines 700-725

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Metrics collector for streaming pipeline
pub struct StreamMetrics {
    /// Total chunks processed
    chunks_processed: AtomicU64,
    
    /// Total tokens generated
    tokens_generated: AtomicU64,
    
    /// Total bytes processed
    bytes_processed: AtomicU64,
    
    /// Total errors encountered
    errors: AtomicU64,
    
    /// Average chunk size
    avg_chunk_size: AtomicU64,
    
    /// Average tokens per chunk
    avg_tokens_per_chunk: AtomicU64,
    
    /// Start time for rate calculations
    start_time: Instant,
    
    /// Whether metrics are enabled
    enabled: bool,
}

impl StreamMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            chunks_processed: AtomicU64::new(0),
            tokens_generated: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            avg_chunk_size: AtomicU64::new(0),
            avg_tokens_per_chunk: AtomicU64::new(0),
            start_time: Instant::now(),
            enabled: true,
        }
    }
    
    /// Create no-op metrics (disabled)
    pub fn noop() -> Self {
        Self {
            chunks_processed: AtomicU64::new(0),
            tokens_generated: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            avg_chunk_size: AtomicU64::new(0),
            avg_tokens_per_chunk: AtomicU64::new(0),
            start_time: Instant::now(),
            enabled: false,
        }
    }
    
    /// Record a processed chunk
    pub fn record_chunk(&self, bytes: usize, tokens: usize) {
        if !self.enabled {
            return;
        }
        
        self.chunks_processed.fetch_add(1, Ordering::Relaxed);
        self.tokens_generated.fetch_add(tokens as u64, Ordering::Relaxed);
        self.bytes_processed.fetch_add(bytes as u64, Ordering::Relaxed);
        
        // Update averages
        let chunks = self.chunks_processed.load(Ordering::Relaxed);
        if chunks > 0 {
            let avg_size = self.bytes_processed.load(Ordering::Relaxed) / chunks;
            let avg_tokens = self.tokens_generated.load(Ordering::Relaxed) / chunks;
            
            self.avg_chunk_size.store(avg_size, Ordering::Relaxed);
            self.avg_tokens_per_chunk.store(avg_tokens, Ordering::Relaxed);
        }
    }
    
    /// Record an error
    pub fn record_error(&self) {
        if self.enabled {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// Get chunks processed
    pub fn chunks_processed(&self) -> u64 {
        self.chunks_processed.load(Ordering::Relaxed)
    }
    
    /// Get tokens generated
    pub fn tokens_generated(&self) -> u64 {
        self.tokens_generated.load(Ordering::Relaxed)
    }
    
    /// Get bytes processed
    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed.load(Ordering::Relaxed)
    }
    
    /// Get error count
    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }
    
    /// Get average chunk size
    pub fn avg_chunk_size(&self) -> u64 {
        self.avg_chunk_size.load(Ordering::Relaxed)
    }
    
    /// Get average tokens per chunk
    pub fn avg_tokens_per_chunk(&self) -> u64 {
        self.avg_tokens_per_chunk.load(Ordering::Relaxed)
    }
    
    /// Get throughput in bytes per second
    pub fn bytes_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.bytes_processed() as f64 / elapsed
        } else {
            0.0
        }
    }
    
    /// Get throughput in tokens per second
    pub fn tokens_per_second(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.tokens_generated() as f64 / elapsed
        } else {
            0.0
        }
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        self.chunks_processed.store(0, Ordering::Relaxed);
        self.tokens_generated.store(0, Ordering::Relaxed);
        self.bytes_processed.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.avg_chunk_size.store(0, Ordering::Relaxed);
        self.avg_tokens_per_chunk.store(0, Ordering::Relaxed);
    }
    
    /// Get a summary of metrics
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            chunks_processed: self.chunks_processed(),
            tokens_generated: self.tokens_generated(),
            bytes_processed: self.bytes_processed(),
            errors: self.errors(),
            avg_chunk_size: self.avg_chunk_size(),
            avg_tokens_per_chunk: self.avg_tokens_per_chunk(),
            bytes_per_second: self.bytes_per_second(),
            tokens_per_second: self.tokens_per_second(),
            elapsed: self.start_time.elapsed(),
        }
    }
}

impl Default for StreamMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics summary snapshot
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub chunks_processed: u64,
    pub tokens_generated: u64,
    pub bytes_processed: u64,
    pub errors: u64,
    pub avg_chunk_size: u64,
    pub avg_tokens_per_chunk: u64,
    pub bytes_per_second: f64,
    pub tokens_per_second: f64,
    pub elapsed: Duration,
}

impl std::fmt::Display for MetricsSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Metrics Summary:\n\
             - Chunks: {}\n\
             - Tokens: {} ({:.1} tokens/s)\n\
             - Bytes: {} ({:.1} bytes/s)\n\
             - Errors: {}\n\
             - Avg chunk: {} bytes\n\
             - Avg tokens/chunk: {}\n\
             - Elapsed: {:?}",
            self.chunks_processed,
            self.tokens_generated,
            self.tokens_per_second,
            self.bytes_processed,
            self.bytes_per_second,
            self.errors,
            self.avg_chunk_size,
            self.avg_tokens_per_chunk,
            self.elapsed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_recording() {
        let metrics = StreamMetrics::new();
        
        // Record some chunks
        metrics.record_chunk(100, 10);
        metrics.record_chunk(200, 20);
        metrics.record_chunk(150, 15);
        
        assert_eq!(metrics.chunks_processed(), 3);
        assert_eq!(metrics.tokens_generated(), 45);
        assert_eq!(metrics.bytes_processed(), 450);
        assert_eq!(metrics.avg_chunk_size(), 150);
        assert_eq!(metrics.avg_tokens_per_chunk(), 15);
    }
    
    #[test]
    fn test_noop_metrics() {
        let metrics = StreamMetrics::noop();
        
        // Recording should do nothing
        metrics.record_chunk(100, 10);
        metrics.record_error();
        
        assert_eq!(metrics.chunks_processed(), 0);
        assert_eq!(metrics.tokens_generated(), 0);
        assert_eq!(metrics.errors(), 0);
    }
    
    #[test]
    fn test_metrics_reset() {
        let metrics = StreamMetrics::new();
        
        metrics.record_chunk(100, 10);
        metrics.record_error();
        
        assert!(metrics.chunks_processed() > 0);
        assert!(metrics.errors() > 0);
        
        metrics.reset();
        
        assert_eq!(metrics.chunks_processed(), 0);
        assert_eq!(metrics.errors(), 0);
    }
}
