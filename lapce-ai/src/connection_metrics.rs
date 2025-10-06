/// Connection Pool Metrics and Statistics
/// Tracking and reporting for connection pool performance

use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// Connection pool statistics
#[derive(Debug)]
pub struct ConnectionStats {
    pub total_connections: AtomicU64,
    pub active_connections: AtomicU32,
    pub idle_connections: AtomicU32,
    pub failed_connections: AtomicU64,
    pub total_requests: AtomicU64,
    pub avg_wait_time_ns: AtomicU64,
    pub total_acquisitions: AtomicU64,
    pub total_returns: AtomicU64,
    pub pool_hits: AtomicU64,
    pub pool_misses: AtomicU64,
    start_time: Instant,
}

impl ConnectionStats {
    pub fn new() -> Self {
        Self {
            total_connections: AtomicU64::new(0),
            active_connections: AtomicU32::new(0),
            idle_connections: AtomicU32::new(0),
            failed_connections: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            avg_wait_time_ns: AtomicU64::new(0),
            total_acquisitions: AtomicU64::new(0),
            total_returns: AtomicU64::new(0),
            pool_hits: AtomicU64::new(0),
            pool_misses: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
    
    /// Record connection acquisition
    pub fn record_acquisition(&self, wait_time: Duration) {
        self.total_acquisitions.fetch_add(1, Ordering::Relaxed);
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        
        // Update average wait time
        let nanos = wait_time.as_nanos() as u64;
        let current = self.avg_wait_time_ns.load(Ordering::Relaxed);
        let acquisitions = self.total_acquisitions.load(Ordering::Relaxed);
        if acquisitions > 0 {
            let new_avg = (current * (acquisitions - 1) + nanos) / acquisitions;
            self.avg_wait_time_ns.store(new_avg, Ordering::Relaxed);
        }
    }
    
    /// Record connection return
    pub fn record_return(&self) {
        self.total_returns.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record pool hit
    pub fn record_hit(&self) {
        self.pool_hits.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record pool miss
    pub fn record_miss(&self) {
        self.pool_misses.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Update pool status
    pub fn update_pool_status(&self, total: u32, available: u32) {
        self.active_connections.store(total - available, Ordering::Relaxed);
        self.idle_connections.store(available, Ordering::Relaxed);
    }
    
    /// Get connection reuse rate
    pub fn get_reuse_rate(&self) -> f64 {
        let hits = self.pool_hits.load(Ordering::Relaxed) as f64;
        let total = (self.pool_hits.load(Ordering::Relaxed) + 
                     self.pool_misses.load(Ordering::Relaxed)) as f64;
        if total > 0.0 {
            (hits / total) * 100.0
        } else {
            0.0
        }
    }
    
    /// Get average acquisition latency in milliseconds
    pub fn get_avg_acquisition_latency_ms(&self) -> f64 {
        let nanos = self.avg_wait_time_ns.load(Ordering::Relaxed) as f64;
        nanos / 1_000_000.0
    }
    
    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let uptime = self.start_time.elapsed().as_secs();
        
        format!(
            "# HELP connection_pool_total Total connections created\n\
             # TYPE connection_pool_total counter\n\
             connection_pool_total {}\n\
             # HELP connection_pool_active Active connections\n\
             # TYPE connection_pool_active gauge\n\
             connection_pool_active {}\n\
             # HELP connection_pool_idle Idle connections\n\
             # TYPE connection_pool_idle gauge\n\
             connection_pool_idle {}\n\
             # HELP connection_pool_failed Failed connections\n\
             # TYPE connection_pool_failed counter\n\
             connection_pool_failed {}\n\
             # HELP connection_pool_requests Total requests\n\
             # TYPE connection_pool_requests counter\n\
             connection_pool_requests {}\n\
             # HELP connection_pool_wait_time_seconds Average wait time\n\
             # TYPE connection_pool_wait_time_seconds gauge\n\
             connection_pool_wait_time_seconds {}\n\
             # HELP connection_pool_reuse_rate Connection reuse rate percentage\n\
             # TYPE connection_pool_reuse_rate gauge\n\
             connection_pool_reuse_rate {}\n\
             # HELP connection_pool_uptime_seconds Pool uptime\n\
             # TYPE connection_pool_uptime_seconds counter\n\
             connection_pool_uptime_seconds {}\n",
            self.total_connections.load(Ordering::Relaxed),
            self.active_connections.load(Ordering::Relaxed),
            self.idle_connections.load(Ordering::Relaxed),
            self.failed_connections.load(Ordering::Relaxed),
            self.total_requests.load(Ordering::Relaxed),
            self.avg_wait_time_ns.load(Ordering::Relaxed) as f64 / 1_000_000_000.0,
            self.get_reuse_rate(),
            uptime
        )
    }
    
    /// Get detailed statistics
    pub fn get_detailed_stats(&self) -> DetailedStats {
        DetailedStats {
            total_connections: self.total_connections.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed) as u64,
            idle_connections: self.idle_connections.load(Ordering::Relaxed) as u64,
            failed_connections: self.failed_connections.load(Ordering::Relaxed),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            avg_wait_time_ms: self.get_avg_acquisition_latency_ms(),
            reuse_rate: self.get_reuse_rate(),
            uptime: self.start_time.elapsed(),
            pool_hits: self.pool_hits.load(Ordering::Relaxed),
            pool_misses: self.pool_misses.load(Ordering::Relaxed),
        }
    }
}

/// Detailed statistics structure
#[derive(Debug, Clone)]
pub struct DetailedStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub idle_connections: u64,
    pub failed_connections: u64,
    pub total_requests: u64,
    pub avg_wait_time_ms: f64,
    pub reuse_rate: f64,
    pub uptime: Duration,
    pub pool_hits: u64,
    pub pool_misses: u64,
}

impl DetailedStats {
    /// Check if metrics meet performance targets
    pub fn meets_targets(&self) -> PerformanceCheck {
        PerformanceCheck {
            memory_ok: true, // Would need actual memory measurement
            reuse_rate_ok: self.reuse_rate >= 95.0,
            latency_ok: self.avg_wait_time_ms < 1.0,
            details: format!(
                "Reuse: {:.1}% (target: >95%), Latency: {:.2}ms (target: <1ms)",
                self.reuse_rate, self.avg_wait_time_ms
            ),
        }
    }
}

/// Performance check result
#[derive(Debug)]
pub struct PerformanceCheck {
    pub memory_ok: bool,
    pub reuse_rate_ok: bool,
    pub latency_ok: bool,
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connection_stats() {
        let stats = ConnectionStats::new();
        
        // Record some acquisitions
        stats.record_acquisition(Duration::from_millis(1));
        stats.record_acquisition(Duration::from_millis(2));
        
        assert_eq!(stats.total_connections.load(Ordering::Relaxed), 2);
        assert_eq!(stats.total_acquisitions.load(Ordering::Relaxed), 2);
        
        // Check average wait time
        let avg = stats.get_avg_acquisition_latency_ms();
        assert!(avg > 0.0 && avg < 3.0);
    }
    
    #[test]
    fn test_reuse_rate() {
        let stats = ConnectionStats::new();
        
        // Record hits and misses
        for _ in 0..95 {
            stats.record_hit();
        }
        for _ in 0..5 {
            stats.record_miss();
        }
        
        let reuse_rate = stats.get_reuse_rate();
        assert_eq!(reuse_rate, 95.0);
    }
    
    #[test]
    fn test_prometheus_export() {
        let stats = ConnectionStats::new();
        stats.record_acquisition(Duration::from_millis(1));
        stats.update_pool_status(10, 7);
        
        let export = stats.export_prometheus();
        assert!(export.contains("connection_pool_total 1"));
        assert!(export.contains("connection_pool_active 3"));
        assert!(export.contains("connection_pool_idle 7"));
    }
}
