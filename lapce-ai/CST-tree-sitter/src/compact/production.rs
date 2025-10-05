//! Production hardening for CompactTree
//! Optimization, monitoring, and reliability features

use super::{CompactTree, CompactTreeBuilder};
use super::interning::{intern_stats, INTERN_POOL};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::time::{Instant, Duration};
use std::collections::HashMap;
use parking_lot::RwLock;
use anyhow::Result;
use tracing::{info, warn, error, debug, instrument};

/// Production metrics for CompactTree system
pub struct CompactMetrics {
    pub total_trees: AtomicUsize,
    pub total_nodes: AtomicU64,
    pub total_memory_bytes: AtomicU64,
    pub peak_memory_bytes: AtomicU64,
    
    // Build metrics
    pub builds_completed: AtomicUsize,
    pub builds_failed: AtomicUsize,
    pub total_build_time_ms: AtomicU64,
    pub avg_build_time_ms: AtomicU64,
    pub total_source_bytes: AtomicU64,
    pub total_compact_bytes: AtomicU64,
    
    // Navigation metrics
    pub navigation_ops: AtomicU64,
    pub navigation_time_ns: AtomicU64,
    // Cache metrics
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    
    // Interning metrics
    pub intern_strings: AtomicUsize,
    pub intern_bytes: AtomicUsize,
    pub intern_hit_rate: RwLock<f64>,
    pub intern_cap_exceeded: AtomicU64,
    
    // Error metrics
    pub parse_errors: AtomicUsize,
    pub build_errors: AtomicUsize,
    pub compression_ratio: RwLock<f64>,
}

impl CompactMetrics {
    pub fn new() -> Self {
        Self {
            total_trees: AtomicUsize::new(0),
            total_nodes: AtomicU64::new(0),
            total_memory_bytes: AtomicU64::new(0),
            peak_memory_bytes: AtomicU64::new(0),
            
            builds_completed: AtomicUsize::new(0),
            builds_failed: AtomicUsize::new(0),
            total_build_time_ms: AtomicU64::new(0),
            avg_build_time_ms: AtomicU64::new(0),
            
            navigation_ops: AtomicU64::new(0),
            navigation_time_ns: AtomicU64::new(0),
            
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            intern_strings: AtomicUsize::new(0),
            intern_bytes: AtomicUsize::new(0),
            intern_hit_rate: RwLock::new(0.0),
            intern_cap_exceeded: AtomicU64::new(0),
            parse_errors: AtomicUsize::new(0),
            build_errors: AtomicUsize::new(0),
            total_source_bytes: AtomicU64::new(0),
            total_compact_bytes: AtomicU64::new(0),
            compression_ratio: RwLock::new(0.0),
        }
    }
    
    /// Record tree build
    pub fn record_build(&self, success: bool, duration: Duration, nodes: usize, memory: usize) {
        if success {
            self.builds_completed.fetch_add(1, Ordering::Relaxed);
            self.total_nodes.fetch_add(nodes as u64, Ordering::Relaxed);
            self.total_memory_bytes.fetch_add(memory as u64, Ordering::Relaxed);
            
            // Update peak memory
            let current = self.total_memory_bytes.load(Ordering::Relaxed);
            let mut peak = self.peak_memory_bytes.load(Ordering::Relaxed);
            while current > peak {
                match self.peak_memory_bytes.compare_exchange(
                    peak,
                    current,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(x) => peak = x,
                }
            }
        } else {
            self.builds_failed.fetch_add(1, Ordering::Relaxed);
        }
        
        let millis = duration.as_millis() as u64;
        self.total_build_time_ms.fetch_add(millis, Ordering::Relaxed);
        
        // Update average
        let total_builds = self.builds_completed.load(Ordering::Relaxed) as u64;
        if total_builds > 0 {
            let total_time = self.total_build_time_ms.load(Ordering::Relaxed);
            self.avg_build_time_ms.store(total_time / total_builds, Ordering::Relaxed);
        }
    }
    
    /// Record navigation operation
    pub fn record_navigation(&self, duration: Duration) {
        self.navigation_ops.fetch_add(1, Ordering::Relaxed);
        self.navigation_time_ns.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }
    
    /// Update compression ratio
    pub fn update_compression(&self, source_bytes: usize, compact_bytes: usize) {
        self.total_source_bytes.fetch_add(source_bytes as u64, Ordering::Relaxed);
        self.total_compact_bytes.fetch_add(compact_bytes as u64, Ordering::Relaxed);
        
        let total_source = self.total_source_bytes.load(Ordering::Relaxed) as f64;
        let total_compact = self.total_compact_bytes.load(Ordering::Relaxed) as f64;
        
        if total_compact > 0.0 {
            *self.compression_ratio.write() = total_source / total_compact;
        }
    }
    
    /// Get current statistics
    pub fn stats(&self) -> MetricsSnapshot {
        // Get interning stats
        let intern_pool_stats = intern_stats();
        
        MetricsSnapshot {
            total_trees: self.total_trees.load(Ordering::Relaxed),
            total_nodes: self.total_nodes.load(Ordering::Relaxed),
            memory_mb: self.total_memory_bytes.load(Ordering::Relaxed) as f64 / 1_048_576.0,
            peak_memory_mb: self.peak_memory_bytes.load(Ordering::Relaxed) as f64 / 1_048_576.0,
            builds_completed: self.builds_completed.load(Ordering::Relaxed),
            builds_failed: self.builds_failed.load(Ordering::Relaxed),
            avg_build_time_ms: self.avg_build_time_ms.load(Ordering::Relaxed) as f64,
            compression_ratio: *self.compression_ratio.read(),
            cache_hit_rate: {
                let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
                let misses = self.cache_misses.load(Ordering::Relaxed) as f64;
                if hits + misses > 0.0 {
                    hits / (hits + misses)
                } else {
                    0.0
                }
            },
            // Interning metrics
            intern_strings: intern_pool_stats.total_strings,
            intern_bytes: intern_pool_stats.total_bytes,
            intern_hit_rate: {
                let hits = intern_pool_stats.hit_count as f64;
                let misses = intern_pool_stats.miss_count as f64;
                if hits + misses > 0.0 {
                    hits / (hits + misses)
                } else {
                    0.0
                }
            },
            intern_cap_exceeded: intern_pool_stats.cap_exceeded_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub total_trees: usize,
    pub total_nodes: u64,
    pub memory_mb: f64,
    pub peak_memory_mb: f64,
    pub builds_completed: usize,
    pub builds_failed: usize,
    pub avg_build_time_ms: f64,
    pub compression_ratio: f64,
    pub cache_hit_rate: f64,
    // Interning metrics
    pub intern_strings: usize,
    pub intern_bytes: usize,
    pub intern_hit_rate: f64,
    pub intern_cap_exceeded: u64,
}

/// Optimized CompactTree builder with production features
pub struct ProductionTreeBuilder {
    /// Base builder
    builder: CompactTreeBuilder,
    
    /// Metrics
    metrics: Arc<CompactMetrics>,
    
    /// Build options
    options: BuildOptions,
}

#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Enable memory optimization
    pub optimize_memory: bool,
    
    /// Enable cache-line alignment
    pub cache_align: bool,
    
    /// Maximum tree size (nodes)
    pub max_tree_size: usize,
    
    /// Enable telemetry
    pub telemetry: bool,
    
    /// Compression level (0-9)
    pub compression_level: u8,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            optimize_memory: true,
            cache_align: true,
            max_tree_size: 1_000_000,
            telemetry: true,
            compression_level: 6,
        }
    }
}

impl ProductionTreeBuilder {
    pub fn new(metrics: Arc<CompactMetrics>, options: BuildOptions) -> Self {
        Self {
            builder: CompactTreeBuilder::new(),
            metrics,
            options,
        }
    }
    
    #[instrument(skip(self, tree, source))]
    pub fn build(
        mut self,
        tree: &tree_sitter::Tree,
        source: &[u8],
    ) -> Result<CompactTree, BuildError> {
        let start = Instant::now();
        
        // Check tree size limit
        let node_count = tree.root_node().descendant_count();
        if node_count > self.options.max_tree_size {
            self.metrics.record_build(false, start.elapsed(), 0, 0);
            return Err(BuildError::TreeTooLarge(node_count));
        }
        
        // Build compact tree
        let compact_tree = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.builder.build(tree, source)
        })) {
            Ok(tree) => tree,
            Err(_) => {
                error!("Panic during tree build");
                self.metrics.record_build(false, start.elapsed(), 0, 0);
                return Err(BuildError::BuildPanic);
            }
        };
        
        // Validate tree
        if let Err(e) = compact_tree.validate() {
            warn!("Tree validation failed: {}", e);
            self.metrics.record_build(false, start.elapsed(), 0, 0);
            return Err(BuildError::ValidationFailed(e));
        }
        
        // Record metrics
        let memory = compact_tree.memory_bytes();
        self.metrics.record_build(true, start.elapsed(), node_count, memory);
        self.metrics.update_compression(source.len(), memory);
        
        if self.options.telemetry {
            info!(
                "Built CompactTree: {} nodes, {} KB, {:.2}x compression",
                node_count,
                memory / 1024,
                source.len() as f64 / memory as f64
            );
        }
        
        Ok(compact_tree)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Tree too large: {0} nodes")]
    TreeTooLarge(usize),
    
    #[error("Build panic")]
    BuildPanic,
    
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

/// Memory pool for CompactTree allocation
pub struct MemoryPool {
    /// Pre-allocated buffers
    buffers: RwLock<Vec<Vec<u8>>>,
    
    /// Buffer size
    buffer_size: usize,
    
    /// Maximum pool size
    max_pool_size: usize,
}

impl MemoryPool {
    pub fn new(buffer_size: usize, max_pool_size: usize) -> Self {
        Self {
            buffers: RwLock::new(Vec::new()),
            buffer_size,
            max_pool_size,
        }
    }
    
    /// Get buffer from pool
    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buffers = self.buffers.write();
        if let Some(buffer) = buffers.pop() {
            buffer
        } else {
            Vec::with_capacity(self.buffer_size)
        }
    }
    
    /// Return buffer to pool
    pub fn return_buffer(&self, mut buffer: Vec<u8>) {
        buffer.clear();
        
        let mut buffers = self.buffers.write();
        if buffers.len() < self.max_pool_size {
            buffers.push(buffer);
        }
    }
}

/// Health monitoring for CompactTree system
pub struct HealthMonitor {
    metrics: Arc<CompactMetrics>,
    thresholds: HealthThresholds,
    last_check: RwLock<Instant>,
}

#[derive(Debug, Clone)]
pub struct HealthThresholds {
    pub max_memory_mb: f64,
    pub max_build_time_ms: f64,
    pub min_compression_ratio: f64,
    pub max_failure_rate: f64,
    pub min_intern_hit_rate: f64,
    pub max_intern_memory_mb: f64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024.0,      // 1 GB
            max_build_time_ms: 100.0,    // 100ms
            min_compression_ratio: 2.0,   // At least 2x
            max_failure_rate: 0.01,       // 1% failures
            min_intern_hit_rate: 0.8,    // 80% hit rate
            max_intern_memory_mb: 100.0,  // 100MB for intern table
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl HealthMonitor {
    pub fn new(metrics: Arc<CompactMetrics>, thresholds: HealthThresholds) -> Self {
        Self {
            metrics,
            thresholds,
            last_check: RwLock::new(Instant::now()),
        }
    }
    
    pub fn check_health(&self) -> HealthStatus {
        let mut status = HealthStatus {
            healthy: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        
        let stats = self.metrics.stats();
        
        // Check memory usage
        if stats.memory_mb > self.thresholds.max_memory_mb {
            status.errors.push(format!(
                "Memory usage too high: {:.2} MB (max: {:.2} MB)",
                stats.memory_mb, self.thresholds.max_memory_mb
            ));
            status.healthy = false;
        }
        
        // Check build time
        if stats.avg_build_time_ms > self.thresholds.max_build_time_ms {
            status.warnings.push(format!(
                "Build time high: {:.2} ms (max: {:.2} ms)",
                stats.avg_build_time_ms, self.thresholds.max_build_time_ms
            ));
        }
        
        // Check compression ratio
        if stats.compression_ratio < self.thresholds.min_compression_ratio {
            status.warnings.push(format!(
                "Compression ratio low: {:.2}x (min: {:.2}x)",
                stats.compression_ratio, self.thresholds.min_compression_ratio
            ));
        }
        
        // Check failure rate
        let total_builds = stats.builds_completed + stats.builds_failed;
        if total_builds > 0 {
            let failure_rate = stats.builds_failed as f64 / total_builds as f64;
            if failure_rate > self.thresholds.max_failure_rate {
                status.errors.push(format!(
                    "Build failure rate too high: {:.2}% (max: {:.2}%)",
                    failure_rate * 100.0, self.thresholds.max_failure_rate * 100.0
                ));
                status.healthy = false;
            }
        }
        
        // Check interning health
        if stats.intern_hit_rate < self.thresholds.min_intern_hit_rate {
            status.warnings.push(format!(
                "Intern hit rate low: {:.2}% (min: {:.2}%)",
                stats.intern_hit_rate * 100.0, self.thresholds.min_intern_hit_rate * 100.0
            ));
        }
        
        let intern_memory_mb = stats.intern_bytes as f64 / 1_048_576.0;
        if intern_memory_mb > self.thresholds.max_intern_memory_mb {
            status.warnings.push(format!(
                "Intern memory high: {:.2} MB (max: {:.2} MB)",
                intern_memory_mb, self.thresholds.max_intern_memory_mb
            ));
        }
        
        if stats.intern_cap_exceeded > 0 {
            status.warnings.push(format!(
                "Intern cap exceeded {} times", 
                stats.intern_cap_exceeded
            ));
        }
        
        *self.last_check.write() = Instant::now();
        
        status
    }
}

/// Performance profiler for CompactTree operations
pub struct Profiler {
    samples: RwLock<HashMap<String, Vec<Duration>>>,
    enabled: AtomicUsize,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            samples: RwLock::new(HashMap::new()),
            enabled: AtomicUsize::new(1),
        }
    }
    
    pub fn profile<F, R>(&self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        if self.enabled.load(Ordering::Relaxed) == 0 {
            return f();
        }
        
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        
        self.samples.write()
            .entry(name.to_string())
            .or_default()
            .push(duration);
        
        result
    }
    
    pub fn report(&self) -> HashMap<String, ProfileStats> {
        let samples = self.samples.read();
        let mut report = HashMap::new();
        
        for (name, durations) in samples.iter() {
            if durations.is_empty() {
                continue;
            }
            
            let total: Duration = durations.iter().sum();
            let avg = total / durations.len() as u32;
            let min = durations.iter().min().copied().unwrap_or_default();
            let max = durations.iter().max().copied().unwrap_or_default();
            
            report.insert(name.clone(), ProfileStats {
                count: durations.len(),
                total,
                avg,
                min,
                max,
            });
        }
        
        report
    }
    
    pub fn clear(&self) {
        self.samples.write().clear();
    }
    
    pub fn enable(&self) {
        self.enabled.store(1, Ordering::Relaxed);
    }
    
    pub fn disable(&self) {
        self.enabled.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct ProfileStats {
    pub count: usize,
    pub total: Duration,
    pub avg: Duration,
    pub min: Duration,
    pub max: Duration,
}

/// Global production instance
lazy_static::lazy_static! {
    pub static ref METRICS: Arc<CompactMetrics> = Arc::new(CompactMetrics::new());
    pub static ref PROFILER: Arc<Profiler> = Arc::new(Profiler::new());
    pub static ref MEMORY_POOL: Arc<MemoryPool> = Arc::new(MemoryPool::new(4096, 100));
}
