/// Off-CPU metrics collection with atomic counters and background exporter
/// Designed for minimal hot-path overhead while maintaining full observability
/// 
/// Features:
/// - Lock-free atomic counters (no contention)
/// - Sampling for histograms (1:1000 default)
/// - Background exporter to Prometheus (500ms interval)
/// - Feature flag for emergency disable

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

/// Sampling rate for histogram metrics (1:N)
const DEFAULT_SAMPLE_RATE: u64 = 1000;

/// Metrics collector with lock-free counters
#[derive(Clone)]
pub struct OptimizedMetricsCollector {
    inner: Arc<MetricsInner>,
}

struct MetricsInner {
    // Counters (incremented on hot path)
    write_count: AtomicU64,
    read_count: AtomicU64,
    write_bytes: AtomicU64,
    read_bytes: AtomicU64,
    backpressure_count: AtomicU64,
    
    // Error counters
    write_errors: AtomicU64,
    read_errors: AtomicU64,
    
    // Sampling
    sample_rate: u64,
    
    // Control
    enabled: AtomicBool,
}

impl OptimizedMetricsCollector {
    /// Create new metrics collector
    pub fn new(sample_rate: u64) -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                write_count: AtomicU64::new(0),
                read_count: AtomicU64::new(0),
                write_bytes: AtomicU64::new(0),
                read_bytes: AtomicU64::new(0),
                backpressure_count: AtomicU64::new(0),
                write_errors: AtomicU64::new(0),
                read_errors: AtomicU64::new(0),
                sample_rate,
                enabled: AtomicBool::new(true),
            }),
        }
    }
    
    /// Record a write operation (hot path - lock-free)
    #[inline(always)]
    pub fn record_write(&self, bytes: usize) {
        if !self.inner.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        self.inner.write_count.fetch_add(1, Ordering::Relaxed);
        self.inner.write_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
    }
    
    /// Record a read operation (hot path - lock-free)
    #[inline(always)]
    pub fn record_read(&self, bytes: usize) {
        if !self.inner.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        self.inner.read_count.fetch_add(1, Ordering::Relaxed);
        self.inner.read_bytes.fetch_add(bytes as u64, Ordering::Relaxed);
    }
    
    /// Record backpressure event
    #[inline(always)]
    pub fn record_backpressure(&self) {
        if !self.inner.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        self.inner.backpressure_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record write error
    #[inline(always)]
    pub fn record_write_error(&self) {
        if !self.inner.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        self.inner.write_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record read error
    #[inline(always)]
    pub fn record_read_error(&self) {
        if !self.inner.enabled.load(Ordering::Relaxed) {
            return;
        }
        
        self.inner.read_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Check if this operation should be sampled for histogram
    #[inline(always)]
    pub fn should_sample(&self) -> bool {
        if !self.inner.enabled.load(Ordering::Relaxed) {
            return false;
        }
        
        let count = self.inner.write_count.load(Ordering::Relaxed);
        count % self.inner.sample_rate == 0
    }
    
    /// Enable metrics collection
    pub fn enable(&self) {
        self.inner.enabled.store(true, Ordering::Relaxed);
    }
    
    /// Disable metrics collection (emergency killswitch)
    pub fn disable(&self) {
        self.inner.enabled.store(false, Ordering::Relaxed);
    }
    
    /// Get snapshot of current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            write_count: self.inner.write_count.load(Ordering::Relaxed),
            read_count: self.inner.read_count.load(Ordering::Relaxed),
            write_bytes: self.inner.write_bytes.load(Ordering::Relaxed),
            read_bytes: self.inner.read_bytes.load(Ordering::Relaxed),
            backpressure_count: self.inner.backpressure_count.load(Ordering::Relaxed),
            write_errors: self.inner.write_errors.load(Ordering::Relaxed),
            read_errors: self.inner.read_errors.load(Ordering::Relaxed),
        }
    }
    
    /// Reset all counters (for rate calculations)
    pub fn reset(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            write_count: self.inner.write_count.swap(0, Ordering::Relaxed),
            read_count: self.inner.read_count.swap(0, Ordering::Relaxed),
            write_bytes: self.inner.write_bytes.swap(0, Ordering::Relaxed),
            read_bytes: self.inner.read_bytes.swap(0, Ordering::Relaxed),
            backpressure_count: self.inner.backpressure_count.swap(0, Ordering::Relaxed),
            write_errors: self.inner.write_errors.swap(0, Ordering::Relaxed),
            read_errors: self.inner.read_errors.swap(0, Ordering::Relaxed),
        }
    }
}

impl Default for OptimizedMetricsCollector {
    fn default() -> Self {
        Self::new(DEFAULT_SAMPLE_RATE)
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, Copy)]
pub struct MetricsSnapshot {
    pub write_count: u64,
    pub read_count: u64,
    pub write_bytes: u64,
    pub read_bytes: u64,
    pub backpressure_count: u64,
    pub write_errors: u64,
    pub read_errors: u64,
}

impl MetricsSnapshot {
    /// Calculate rates per second given duration
    pub fn rates(&self, duration: Duration) -> MetricsRates {
        let secs = duration.as_secs_f64();
        
        MetricsRates {
            writes_per_sec: self.write_count as f64 / secs,
            reads_per_sec: self.read_count as f64 / secs,
            write_bytes_per_sec: self.write_bytes as f64 / secs,
            read_bytes_per_sec: self.read_bytes as f64 / secs,
            backpressure_per_sec: self.backpressure_count as f64 / secs,
        }
    }
}

/// Calculated rates
#[derive(Debug, Clone, Copy)]
pub struct MetricsRates {
    pub writes_per_sec: f64,
    pub reads_per_sec: f64,
    pub write_bytes_per_sec: f64,
    pub read_bytes_per_sec: f64,
    pub backpressure_per_sec: f64,
}

/// Background exporter that publishes metrics to Prometheus
pub struct MetricsExporter {
    collector: OptimizedMetricsCollector,
    export_interval: Duration,
    handle: Option<JoinHandle<()>>,
}

impl MetricsExporter {
    /// Create new exporter
    pub fn new(collector: OptimizedMetricsCollector, export_interval: Duration) -> Self {
        Self {
            collector,
            export_interval,
            handle: None,
        }
    }
    
    /// Start background export loop
    pub fn start(&mut self) {
        let collector = self.collector.clone();
        let interval = self.export_interval;
        
        let handle = tokio::spawn(async move {
            Self::export_loop(collector, interval).await;
        });
        
        self.handle = Some(handle);
    }
    
    /// Background export loop
    async fn export_loop(collector: OptimizedMetricsCollector, interval: Duration) {
        let mut ticker = tokio::time::interval(interval);
        let mut last_snapshot = collector.snapshot();
        let mut last_time = Instant::now();
        
        loop {
            ticker.tick().await;
            
            let now = Instant::now();
            let snapshot = collector.reset();
            let duration = now - last_time;
            
            // Calculate rates
            let delta = MetricsSnapshot {
                write_count: snapshot.write_count.saturating_sub(last_snapshot.write_count),
                read_count: snapshot.read_count.saturating_sub(last_snapshot.read_count),
                write_bytes: snapshot.write_bytes.saturating_sub(last_snapshot.write_bytes),
                read_bytes: snapshot.read_bytes.saturating_sub(last_snapshot.read_bytes),
                backpressure_count: snapshot.backpressure_count.saturating_sub(last_snapshot.backpressure_count),
                write_errors: snapshot.write_errors.saturating_sub(last_snapshot.write_errors),
                read_errors: snapshot.read_errors.saturating_sub(last_snapshot.read_errors),
            };
            
            let rates = delta.rates(duration);
            
            // Export to Prometheus (would integrate with actual prometheus crate here)
            #[cfg(feature = "prometheus")]
            {
                use prometheus::{register_gauge, Gauge};
                
                static WRITES_PER_SEC: once_cell::sync::Lazy<Gauge> = 
                    once_cell::sync::Lazy::new(|| {
                        register_gauge!("ipc_writes_per_second", "Write operations per second").unwrap()
                    });
                
                WRITES_PER_SEC.set(rates.writes_per_sec);
            }
            
            // Log for debugging
            tracing::debug!(
                writes_per_sec = %rates.writes_per_sec,
                reads_per_sec = %rates.reads_per_sec,
                write_mb_per_sec = %(rates.write_bytes_per_sec / 1_000_000.0),
                read_mb_per_sec = %(rates.read_bytes_per_sec / 1_000_000.0),
                backpressure_per_sec = %rates.backpressure_per_sec,
                "IPC metrics"
            );
            
            last_snapshot = snapshot;
            last_time = now;
        }
    }
    
    /// Stop background export loop
    pub async fn stop(mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            let _ = handle.await;
        }
    }
}

/// RAII guard for timing operations with sampling
pub struct TimedOperation {
    collector: OptimizedMetricsCollector,
    start: Instant,
    sample: bool,
}

impl TimedOperation {
    pub fn new(collector: &OptimizedMetricsCollector) -> Self {
        let sample = collector.should_sample();
        Self {
            collector: collector.clone(),
            start: if sample { Instant::now() } else { Instant::now() }, // Always initialize
            sample,
        }
    }
    
    /// Get elapsed time if sampling
    pub fn elapsed_micros(&self) -> Option<u64> {
        if self.sample {
            Some(self.start.elapsed().as_micros() as u64)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_collection() {
        let collector = OptimizedMetricsCollector::new(100);
        
        // Record some operations
        collector.record_write(1024);
        collector.record_write(2048);
        collector.record_read(512);
        
        let snapshot = collector.snapshot();
        assert_eq!(snapshot.write_count, 2);
        assert_eq!(snapshot.write_bytes, 3072);
        assert_eq!(snapshot.read_count, 1);
        assert_eq!(snapshot.read_bytes, 512);
    }
    
    #[test]
    fn test_sampling() {
        let collector = OptimizedMetricsCollector::new(10);
        
        let mut sampled = 0;
        for _ in 0..100 {
            collector.record_write(100);
            if collector.should_sample() {
                sampled += 1;
            }
        }
        
        // Should sample ~10 times (1:10 rate)
        assert!(sampled >= 8 && sampled <= 12, "Sampled {} times", sampled);
    }
    
    #[test]
    fn test_disable_enable() {
        let collector = OptimizedMetricsCollector::new(1);
        
        collector.record_write(100);
        assert_eq!(collector.snapshot().write_count, 1);
        
        collector.disable();
        collector.record_write(100);
        assert_eq!(collector.snapshot().write_count, 1); // No change
        
        collector.enable();
        collector.record_write(100);
        assert_eq!(collector.snapshot().write_count, 2);
    }
    
    #[tokio::test]
    async fn test_exporter() {
        let collector = OptimizedMetricsCollector::new(1);
        let mut exporter = MetricsExporter::new(collector.clone(), Duration::from_millis(100));
        
        exporter.start();
        
        // Generate some metrics
        for _ in 0..100 {
            collector.record_write(1024);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        
        exporter.stop().await;
    }
}
