/// Unified metrics collection for IPC, connection pools, and multiplexers
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use serde::{Serialize, Deserialize};
use dashmap::DashMap;

/// Unified metrics for all connection types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMetrics {
    // IPC metrics
    pub ipc_connections: u64,
    pub ipc_messages_sent: u64,
    pub ipc_messages_received: u64,
    pub ipc_bytes_sent: u64,
    pub ipc_bytes_received: u64,
    pub ipc_errors: u64,
    
    // HTTP/2 multiplexer metrics
    pub http2_connections: u64,
    pub http2_streams_active: u64,
    pub http2_streams_total: u64,
    pub http2_bandwidth_in: u64,
    pub http2_bandwidth_out: u64,
    
    // WebSocket metrics
    pub ws_connections: u64,
    pub ws_messages_sent: u64,
    pub ws_messages_received: u64,
    pub ws_reconnections: u64,
    
    // Connection pool metrics (bb8-style)
    pub pool_size: u64,
    pub pool_available: u64,
    pub pool_waiting: u64,
    pub pool_timeout_count: u64,
    pub pool_total_created: u64,
    pub pool_total_recycled: u64,
    
    // Performance metrics
    pub avg_latency_us: u64,
    pub p50_latency_us: u64,
    pub p99_latency_us: u64,
    pub throughput_msg_per_sec: u64,
    
    // Resource usage
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f32,
    
    // Health status
    pub health_score: u8,  // 0-100
    pub last_updated: u64,  // Unix timestamp
}

/// Metrics collector that aggregates from all sources
pub struct MetricsCollector {
    // IPC metrics
    ipc_connections: Arc<AtomicU64>,
    ipc_messages_sent: Arc<AtomicU64>,
    ipc_messages_received: Arc<AtomicU64>,
    ipc_bytes_sent: Arc<AtomicU64>,
    ipc_bytes_received: Arc<AtomicU64>,
    ipc_errors: Arc<AtomicU64>,
    
    // HTTP/2 metrics
    http2_connections: Arc<AtomicU64>,
    http2_streams_active: Arc<AtomicU64>,
    http2_streams_total: Arc<AtomicU64>,
    http2_bandwidth_in: Arc<AtomicU64>,
    http2_bandwidth_out: Arc<AtomicU64>,
    
    // WebSocket metrics
    ws_connections: Arc<AtomicU64>,
    ws_messages_sent: Arc<AtomicU64>,
    ws_messages_received: Arc<AtomicU64>,
    ws_reconnections: Arc<AtomicU64>,
    
    // Pool metrics
    pool_stats: Arc<DashMap<String, PoolMetrics>>,
    
    // Latency tracking
    latency_samples: Arc<DashMap<String, Vec<u64>>>,
    
    // Last snapshot
    last_snapshot: Arc<parking_lot::RwLock<UnifiedMetrics>>,
}

#[derive(Debug)]
struct PoolMetrics {
    size: Arc<AtomicUsize>,
    available: Arc<AtomicUsize>,
    waiting: Arc<AtomicUsize>,
    timeout_count: Arc<AtomicU64>,
    total_created: Arc<AtomicU64>,
    total_recycled: Arc<AtomicU64>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            ipc_connections: Arc::new(AtomicU64::new(0)),
            ipc_messages_sent: Arc::new(AtomicU64::new(0)),
            ipc_messages_received: Arc::new(AtomicU64::new(0)),
            ipc_bytes_sent: Arc::new(AtomicU64::new(0)),
            ipc_bytes_received: Arc::new(AtomicU64::new(0)),
            ipc_errors: Arc::new(AtomicU64::new(0)),
            
            http2_connections: Arc::new(AtomicU64::new(0)),
            http2_streams_active: Arc::new(AtomicU64::new(0)),
            http2_streams_total: Arc::new(AtomicU64::new(0)),
            http2_bandwidth_in: Arc::new(AtomicU64::new(0)),
            http2_bandwidth_out: Arc::new(AtomicU64::new(0)),
            
            ws_connections: Arc::new(AtomicU64::new(0)),
            ws_messages_sent: Arc::new(AtomicU64::new(0)),
            ws_messages_received: Arc::new(AtomicU64::new(0)),
            ws_reconnections: Arc::new(AtomicU64::new(0)),
            
            pool_stats: Arc::new(DashMap::new()),
            latency_samples: Arc::new(DashMap::new()),
            last_snapshot: Arc::new(parking_lot::RwLock::new(UnifiedMetrics::default())),
        }
    }
    
    /// Record IPC connection event
    pub fn record_ipc_connection(&self, connected: bool) {
        if connected {
            self.ipc_connections.fetch_add(1, Ordering::Relaxed);
        } else {
            self.ipc_connections.fetch_sub(1, Ordering::Relaxed);
        }
    }
    
    /// Record IPC message
    pub fn record_ipc_message(&self, sent: bool, bytes: usize) {
        if sent {
            self.ipc_messages_sent.fetch_add(1, Ordering::Relaxed);
            self.ipc_bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
        } else {
            self.ipc_messages_received.fetch_add(1, Ordering::Relaxed);
            self.ipc_bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
        }
    }
    
    /// Record HTTP/2 stream
    pub fn record_http2_stream(&self, active: bool) {
        self.http2_streams_total.fetch_add(1, Ordering::Relaxed);
        if active {
            self.http2_streams_active.fetch_add(1, Ordering::Relaxed);
        } else {
            self.http2_streams_active.fetch_sub(1, Ordering::Relaxed);
        }
    }
    
    /// Update pool statistics
    pub fn update_pool_stats(&self, pool_name: &str, size: usize, available: usize, waiting: usize) {
        let entry = self.pool_stats.entry(pool_name.to_string())
            .or_insert_with(|| PoolMetrics {
                size: Arc::new(AtomicUsize::new(0)),
                available: Arc::new(AtomicUsize::new(0)),
                waiting: Arc::new(AtomicUsize::new(0)),
                timeout_count: Arc::new(AtomicU64::new(0)),
                total_created: Arc::new(AtomicU64::new(0)),
                total_recycled: Arc::new(AtomicU64::new(0)),
            });
        
        entry.size.store(size, Ordering::Relaxed);
        entry.available.store(available, Ordering::Relaxed);
        entry.waiting.store(waiting, Ordering::Relaxed);
    }
    
    /// Record latency sample
    pub fn record_latency(&self, component: &str, latency_us: u64) {
        let mut samples = self.latency_samples.entry(component.to_string())
            .or_insert_with(Vec::new);
        
        samples.value_mut().push(latency_us);
        
        // Keep only last 1000 samples
        if samples.value().len() > 1000 {
            let drain_count = samples.value().len() - 1000;
            samples.value_mut().drain(0..drain_count);
        }
    }
    
    /// Generate metrics snapshot
    pub fn snapshot(&self) -> UnifiedMetrics {
        // Aggregate pool stats
        let mut pool_size = 0u64;
        let mut pool_available = 0u64;
        let mut pool_waiting = 0u64;
        
        for entry in self.pool_stats.iter() {
            pool_size += entry.size.load(Ordering::Relaxed) as u64;
            pool_available += entry.available.load(Ordering::Relaxed) as u64;
            pool_waiting += entry.waiting.load(Ordering::Relaxed) as u64;
        }
        
        // Calculate latency percentiles
        let mut all_latencies = Vec::new();
        for entry in self.latency_samples.iter() {
            all_latencies.extend_from_slice(&entry.value());
        }
        
        all_latencies.sort_unstable();
        
        let p50 = if !all_latencies.is_empty() {
            all_latencies[all_latencies.len() / 2]
        } else {
            0
        };
        
        let p99 = if !all_latencies.is_empty() {
            all_latencies[all_latencies.len() * 99 / 100]
        } else {
            0
        };
        
        let avg = if !all_latencies.is_empty() {
            all_latencies.iter().sum::<u64>() / all_latencies.len() as u64
        } else {
            0
        };
        
        // Calculate health score (0-100)
        let health_score = self.calculate_health_score();
        
        let metrics = UnifiedMetrics {
            ipc_connections: self.ipc_connections.load(Ordering::Relaxed),
            ipc_messages_sent: self.ipc_messages_sent.load(Ordering::Relaxed),
            ipc_messages_received: self.ipc_messages_received.load(Ordering::Relaxed),
            ipc_bytes_sent: self.ipc_bytes_sent.load(Ordering::Relaxed),
            ipc_bytes_received: self.ipc_bytes_received.load(Ordering::Relaxed),
            ipc_errors: self.ipc_errors.load(Ordering::Relaxed),
            
            http2_connections: self.http2_connections.load(Ordering::Relaxed),
            http2_streams_active: self.http2_streams_active.load(Ordering::Relaxed),
            http2_streams_total: self.http2_streams_total.load(Ordering::Relaxed),
            http2_bandwidth_in: self.http2_bandwidth_in.load(Ordering::Relaxed),
            http2_bandwidth_out: self.http2_bandwidth_out.load(Ordering::Relaxed),
            
            ws_connections: self.ws_connections.load(Ordering::Relaxed),
            ws_messages_sent: self.ws_messages_sent.load(Ordering::Relaxed),
            ws_messages_received: self.ws_messages_received.load(Ordering::Relaxed),
            ws_reconnections: self.ws_reconnections.load(Ordering::Relaxed),
            
            pool_size,
            pool_available,
            pool_waiting,
            pool_timeout_count: 0,  // TODO: aggregate from pools
            pool_total_created: 0,  // TODO: aggregate from pools
            pool_total_recycled: 0,  // TODO: aggregate from pools
            
            avg_latency_us: avg,
            p50_latency_us: p50,
            p99_latency_us: p99,
            throughput_msg_per_sec: self.calculate_throughput(),
            
            memory_usage_bytes: self.get_memory_usage(),
            cpu_usage_percent: 0.0,  // TODO: implement CPU tracking
            
            health_score,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
        };
        
        // Store snapshot
        *self.last_snapshot.write() = metrics.clone();
        
        metrics
    }
    
    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let metrics = self.snapshot();
        
        let mut output = String::new();
        
        // IPC metrics
        output.push_str(&format!("# HELP ipc_connections Active IPC connections\n"));
        output.push_str(&format!("# TYPE ipc_connections gauge\n"));
        output.push_str(&format!("ipc_connections {}\n", metrics.ipc_connections));
        
        output.push_str(&format!("# HELP ipc_messages_total Total IPC messages\n"));
        output.push_str(&format!("# TYPE ipc_messages_total counter\n"));
        output.push_str(&format!("ipc_messages_total{{direction=\"sent\"}} {}\n", metrics.ipc_messages_sent));
        output.push_str(&format!("ipc_messages_total{{direction=\"received\"}} {}\n", metrics.ipc_messages_received));
        
        // HTTP/2 metrics
        output.push_str(&format!("# HELP http2_streams_active Active HTTP/2 streams\n"));
        output.push_str(&format!("# TYPE http2_streams_active gauge\n"));
        output.push_str(&format!("http2_streams_active {}\n", metrics.http2_streams_active));
        
        // Pool metrics
        output.push_str(&format!("# HELP connection_pool_size Total connections in pool\n"));
        output.push_str(&format!("# TYPE connection_pool_size gauge\n"));
        output.push_str(&format!("connection_pool_size {}\n", metrics.pool_size));
        
        output.push_str(&format!("# HELP connection_pool_available Available connections\n"));
        output.push_str(&format!("# TYPE connection_pool_available gauge\n"));
        output.push_str(&format!("connection_pool_available {}\n", metrics.pool_available));
        
        // Latency metrics
        output.push_str(&format!("# HELP request_latency_microseconds Request latency\n"));
        output.push_str(&format!("# TYPE request_latency_microseconds histogram\n"));
        output.push_str(&format!("request_latency_microseconds{{quantile=\"0.5\"}} {}\n", metrics.p50_latency_us));
        output.push_str(&format!("request_latency_microseconds{{quantile=\"0.99\"}} {}\n", metrics.p99_latency_us));
        output.push_str(&format!("request_latency_microseconds_avg {}\n", metrics.avg_latency_us));
        
        // Throughput
        output.push_str(&format!("# HELP throughput_messages_per_second Message throughput\n"));
        output.push_str(&format!("# TYPE throughput_messages_per_second gauge\n"));
        output.push_str(&format!("throughput_messages_per_second {}\n", metrics.throughput_msg_per_sec));
        
        // Health
        output.push_str(&format!("# HELP health_score System health score (0-100)\n"));
        output.push_str(&format!("# TYPE health_score gauge\n"));
        output.push_str(&format!("health_score {}\n", metrics.health_score));
        
        output
    }
    
    fn calculate_throughput(&self) -> u64 {
        // Simple calculation based on messages per second
        let total_messages = self.ipc_messages_sent.load(Ordering::Relaxed) 
            + self.ipc_messages_received.load(Ordering::Relaxed);
        
        // TODO: track time window for accurate calculation
        total_messages  // Placeholder
    }
    
    fn calculate_health_score(&self) -> u8 {
        let mut score = 100u8;
        
        // Deduct points for errors
        let errors = self.ipc_errors.load(Ordering::Relaxed);
        if errors > 100 {
            score = score.saturating_sub(20);
        } else if errors > 10 {
            score = score.saturating_sub(10);
        }
        
        // Check latency
        if let Some(latencies) = self.latency_samples.get("ipc") {
            let avg = latencies.iter().sum::<u64>() / latencies.len().max(1) as u64;
            if avg > 1000 {  // > 1ms
                score = score.saturating_sub(10);
            }
        }
        
        score
    }
    
    fn get_memory_usage(&self) -> u64 {
        // Estimate memory usage
        // TODO: use actual memory tracking
        0
    }
}

impl Default for UnifiedMetrics {
    fn default() -> Self {
        Self {
            ipc_connections: 0,
            ipc_messages_sent: 0,
            ipc_messages_received: 0,
            ipc_bytes_sent: 0,
            ipc_bytes_received: 0,
            ipc_errors: 0,
            
            http2_connections: 0,
            http2_streams_active: 0,
            http2_streams_total: 0,
            http2_bandwidth_in: 0,
            http2_bandwidth_out: 0,
            
            ws_connections: 0,
            ws_messages_sent: 0,
            ws_messages_received: 0,
            ws_reconnections: 0,
            
            pool_size: 0,
            pool_available: 0,
            pool_waiting: 0,
            pool_timeout_count: 0,
            pool_total_created: 0,
            pool_total_recycled: 0,
            
            avg_latency_us: 0,
            p50_latency_us: 0,
            p99_latency_us: 0,
            throughput_msg_per_sec: 0,
            
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            
            health_score: 100,
            last_updated: 0,
        }
    }
}

// Global metrics instance
use once_cell::sync::Lazy;
pub static METRICS: Lazy<MetricsCollector> = Lazy::new(|| MetricsCollector::new());
