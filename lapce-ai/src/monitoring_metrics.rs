/// Monitoring & Metrics Implementation (Days 43-44)
use prometheus::{Encoder, TextEncoder, Counter, Gauge, Histogram, HistogramOpts};
use std::time::Instant;
use anyhow::Result;

use once_cell::sync::Lazy;

// Counters
pub static MESSAGES_PROCESSED: Lazy<Counter> = Lazy::new(|| Counter::new(
    "ipc_messages_processed_total", "Total messages processed"
).unwrap());

pub static CACHE_HITS: Lazy<Counter> = Lazy::new(|| Counter::new(
    "cache_hits_total", "Total cache hits"
).unwrap());

pub static CACHE_MISSES: Lazy<Counter> = Lazy::new(|| Counter::new(
    "cache_misses_total", "Total cache misses"
).unwrap());

pub static ERRORS_TOTAL: Lazy<Counter> = Lazy::new(|| Counter::new(
    "errors_total", "Total errors"
).unwrap());

// Gauges
pub static ACTIVE_CONNECTIONS: Lazy<Gauge> = Lazy::new(|| Gauge::new(
    "active_connections", "Current active connections"
).unwrap());

pub static MEMORY_USAGE_BYTES: Lazy<Gauge> = Lazy::new(|| Gauge::new(
    "memory_usage_bytes", "Current memory usage in bytes"
).unwrap());

pub static CPU_USAGE_PERCENT: Lazy<Gauge> = Lazy::new(|| Gauge::new(
    "cpu_usage_percent", "Current CPU usage percentage"
).unwrap());

// Histograms
pub static MESSAGE_LATENCY: Lazy<Histogram> = Lazy::new(|| Histogram::with_opts(
    HistogramOpts::new("message_latency_seconds", "Message processing latency")
        .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
).unwrap());

pub static CACHE_OPERATION_DURATION: Lazy<Histogram> = Lazy::new(|| Histogram::with_opts(
    HistogramOpts::new("cache_operation_duration_seconds", "Cache operation duration")
        .buckets(vec![0.00001, 0.0001, 0.001, 0.01, 0.1])
).unwrap());

pub struct MetricsCollector {
    start_time: Instant,
    registry: prometheus::Registry,
}

impl MetricsCollector {
    pub fn new() -> Result<Self> {
        let registry = prometheus::Registry::new();
        
        // Register all metrics
        registry.register(Box::new(MESSAGES_PROCESSED.clone()))?;
        registry.register(Box::new(CACHE_HITS.clone()))?;
        registry.register(Box::new(CACHE_MISSES.clone()))?;
        registry.register(Box::new(ERRORS_TOTAL.clone()))?;
        registry.register(Box::new(ACTIVE_CONNECTIONS.clone()))?;
        registry.register(Box::new(MEMORY_USAGE_BYTES.clone()))?;
        registry.register(Box::new(CPU_USAGE_PERCENT.clone()))?;
        registry.register(Box::new(MESSAGE_LATENCY.clone()))?;
        registry.register(Box::new(CACHE_OPERATION_DURATION.clone()))?;
        
        Ok(Self {
            start_time: Instant::now(),
            registry,
        })
    }
    
    pub fn export_metrics(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
    
    pub fn update_system_metrics(&self) {
        // Update memory usage
        if let Ok(mem_info) = sys_info::mem_info() {
            let used = (mem_info.total - mem_info.free) * 1024;
            MEMORY_USAGE_BYTES.set(used as f64);
        }
        
        // Update CPU usage
        if let Ok(loadavg) = sys_info::loadavg() {
            CPU_USAGE_PERCENT.set(loadavg.one * 100.0);
        }
    }
}

/// Health check endpoint data
#[derive(serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub uptime_seconds: f64,
    pub messages_processed: f64,
    pub active_connections: f64,
    pub cache_hit_ratio: f64,
    pub error_rate: f64,
}

impl MetricsCollector {
    pub fn health_status(&self) -> HealthStatus {
        let uptime = self.start_time.elapsed().as_secs_f64();
        let hits = CACHE_HITS.get();
        let misses = CACHE_MISSES.get();
        let total_cache = hits + misses;
        
        HealthStatus {
            status: "healthy".to_string(),
            uptime_seconds: uptime,
            messages_processed: MESSAGES_PROCESSED.get(),
            active_connections: ACTIVE_CONNECTIONS.get(),
            cache_hit_ratio: if total_cache > 0.0 { hits / total_cache } else { 0.0 },
            error_rate: ERRORS_TOTAL.get() / MESSAGES_PROCESSED.get().max(1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_export() {
        let collector = MetricsCollector::new().unwrap();
        
        // Record some metrics
        MESSAGES_PROCESSED.inc();
        CACHE_HITS.inc_by(10.0);
        ACTIVE_CONNECTIONS.set(5.0);
        
        let metrics = collector.export_metrics().unwrap();
        assert!(metrics.contains("ipc_messages_processed_total"));
        assert!(metrics.contains("cache_hits_total"));
        assert!(metrics.contains("active_connections"));
    }
    
    #[test]
    fn test_health_status() {
        let collector = MetricsCollector::new().unwrap();
        
        MESSAGES_PROCESSED.inc_by(100.0);
        CACHE_HITS.inc_by(80.0);
        CACHE_MISSES.inc_by(20.0);
        ACTIVE_CONNECTIONS.set(10.0);
        
        let health = collector.health_status();
        assert_eq!(health.status, "healthy");
        assert_eq!(health.cache_hit_ratio, 0.8);
    }
}
