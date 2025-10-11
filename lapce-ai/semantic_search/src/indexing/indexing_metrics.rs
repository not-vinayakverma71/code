//! Telemetry metrics for incremental indexing (CST-B08)
//!
//! Provides Prometheus metrics for:
//! - Stable ID generation and reuse
//! - Cache hit/miss rates
//! - Incremental vs full parse performance
//! - Encoding/decoding times
//! - Overall speedup measurements

use prometheus::{
    register_counter_vec, register_histogram_vec, register_gauge,
    CounterVec, HistogramVec, Gauge,
};
use lazy_static::lazy_static;
use std::time::Instant;

lazy_static! {
    // Stable ID metrics
    pub static ref STABLE_IDS_GENERATED_TOTAL: CounterVec = register_counter_vec!(
        "indexing_stable_ids_generated_total",
        "Total number of stable IDs generated",
        &["language"]
    ).unwrap();
    
    pub static ref STABLE_IDS_REUSED_TOTAL: CounterVec = register_counter_vec!(
        "indexing_stable_ids_reused_total",
        "Total number of stable IDs reused from cache",
        &["language"]
    ).unwrap();
    
    pub static ref STABLE_ID_CACHE_SIZE: Gauge = register_gauge!(
        "indexing_stable_id_cache_size",
        "Current number of entries in stable ID cache"
    ).unwrap();
    
    // Cache performance metrics
    pub static ref EMBEDDING_CACHE_HITS_TOTAL: CounterVec = register_counter_vec!(
        "indexing_embedding_cache_hits_total",
        "Total number of embedding cache hits",
        &["file_type"]
    ).unwrap();
    
    pub static ref EMBEDDING_CACHE_MISSES_TOTAL: CounterVec = register_counter_vec!(
        "indexing_embedding_cache_misses_total",
        "Total number of embedding cache misses",
        &["file_type"]
    ).unwrap();
    
    pub static ref EMBEDDING_CACHE_EVICTIONS_TOTAL: CounterVec = register_counter_vec!(
        "indexing_embedding_cache_evictions_total",
        "Total number of cache evictions",
        &["reason"]
    ).unwrap();
    
    // Change detection metrics
    pub static ref NODES_UNCHANGED_TOTAL: CounterVec = register_counter_vec!(
        "indexing_nodes_unchanged_total",
        "Total number of unchanged nodes detected",
        &["language"]
    ).unwrap();
    
    pub static ref NODES_MODIFIED_TOTAL: CounterVec = register_counter_vec!(
        "indexing_nodes_modified_total",
        "Total number of modified nodes detected",
        &["language"]
    ).unwrap();
    
    pub static ref NODES_ADDED_TOTAL: CounterVec = register_counter_vec!(
        "indexing_nodes_added_total",
        "Total number of new nodes detected",
        &["language"]
    ).unwrap();
    
    pub static ref NODES_DELETED_TOTAL: CounterVec = register_counter_vec!(
        "indexing_nodes_deleted_total",
        "Total number of deleted nodes detected",
        &["language"]
    ).unwrap();
    
    // Performance metrics
    pub static ref PARSE_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "indexing_parse_duration_seconds",
        "Time spent parsing files",
        &["strategy", "language"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
    ).unwrap();
    
    pub static ref CHANGE_DETECTION_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "indexing_change_detection_duration_seconds",
        "Time spent detecting changes",
        &["language"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    ).unwrap();
    
    pub static ref EMBEDDING_GENERATION_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "indexing_embedding_generation_duration_seconds",
        "Time spent generating embeddings",
        &["cached"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    ).unwrap();
    
    pub static ref INCREMENTAL_INDEX_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "indexing_incremental_index_duration_seconds",
        "Total time for incremental indexing operation",
        &["language"],
        vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]
    ).unwrap();
    
    // Speedup metrics
    pub static ref INCREMENTAL_SPEEDUP_RATIO: HistogramVec = register_histogram_vec!(
        "indexing_incremental_speedup_ratio",
        "Speedup ratio: full_time / incremental_time",
        &["language"],
        vec![1.0, 1.5, 2.0, 3.0, 5.0, 10.0, 20.0, 50.0]
    ).unwrap();
    
    pub static ref CACHE_HIT_RATE: HistogramVec = register_histogram_vec!(
        "indexing_cache_hit_rate",
        "Cache hit rate percentage",
        &["cache_type"],
        vec![0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 1.0]
    ).unwrap();
    
    // Async indexer metrics
    pub static ref INDEXER_QUEUE_LENGTH: Gauge = register_gauge!(
        "indexing_queue_length",
        "Current number of tasks in indexing queue"
    ).unwrap();
    
    pub static ref INDEXER_ACTIVE_TASKS: Gauge = register_gauge!(
        "indexing_active_tasks",
        "Current number of active indexing tasks"
    ).unwrap();
    
    pub static ref INDEXER_TASKS_COMPLETED_TOTAL: CounterVec = register_counter_vec!(
        "indexing_tasks_completed_total",
        "Total number of completed indexing tasks",
        &["status"]
    ).unwrap();
    
    pub static ref INDEXER_TASK_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "indexing_task_duration_seconds",
        "Time spent on individual indexing tasks",
        &["priority"],
        vec![0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]
    ).unwrap();
    
    pub static ref INDEXER_BACKPRESSURE_EVENTS_TOTAL: CounterVec = register_counter_vec!(
        "indexing_backpressure_events_total",
        "Total number of back-pressure events",
        &["reason"]
    ).unwrap();
    
    pub static ref INDEXER_TIMEOUT_EVENTS_TOTAL: CounterVec = register_counter_vec!(
        "indexing_timeout_events_total",
        "Total number of timeout events",
        &["operation"]
    ).unwrap();
}

/// Timer guard for automatic metric recording
pub struct MetricTimer {
    start: Instant,
    histogram: &'static HistogramVec,
    labels: Vec<String>,
}

impl MetricTimer {
    pub fn new(histogram: &'static HistogramVec, labels: Vec<&str>) -> Self {
        Self {
            start: Instant::now(),
            histogram,
            labels: labels.iter().map(|s| s.to_string()).collect(),
        }
    }
    
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

impl Drop for MetricTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        let label_refs: Vec<&str> = self.labels.iter().map(|s| s.as_str()).collect();
        self.histogram
            .with_label_values(&label_refs)
            .observe(duration.as_secs_f64());
    }
}

/// Helper to record cache hit rate
pub fn record_cache_hit_rate(cache_type: &str, hits: u64, total: u64) {
    if total > 0 {
        let rate = hits as f64 / total as f64;
        CACHE_HIT_RATE
            .with_label_values(&[cache_type])
            .observe(rate);
    }
}

/// Helper to record speedup ratio
pub fn record_speedup(language: &str, full_time_ms: f64, incremental_time_ms: f64) {
    if incremental_time_ms > 0.0 {
        let ratio = full_time_ms / incremental_time_ms;
        INCREMENTAL_SPEEDUP_RATIO
            .with_label_values(&[language])
            .observe(ratio);
    }
}

/// Helper to record change detection results
pub fn record_changeset(
    language: &str,
    unchanged: usize,
    modified: usize,
    added: usize,
    deleted: usize,
) {
    NODES_UNCHANGED_TOTAL
        .with_label_values(&[language])
        .inc_by(unchanged as f64);
    NODES_MODIFIED_TOTAL
        .with_label_values(&[language])
        .inc_by(modified as f64);
    NODES_ADDED_TOTAL
        .with_label_values(&[language])
        .inc_by(added as f64);
    NODES_DELETED_TOTAL
        .with_label_values(&[language])
        .inc_by(deleted as f64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_metric_timer() {
        let _timer = MetricTimer::new(&PARSE_DURATION_SECONDS, vec!["full", "rust"]);
        thread::sleep(Duration::from_millis(10));
        // Timer records on drop
    }
    
    #[test]
    fn test_cache_hit_rate_recording() {
        record_cache_hit_rate("embedding", 80, 100);
        // Verify it doesn't panic
    }
    
    #[test]
    fn test_speedup_recording() {
        record_speedup("rust", 1000.0, 200.0);
        // Should record 5x speedup
    }
    
    #[test]
    fn test_changeset_recording() {
        record_changeset("rust", 10, 2, 1, 0);
        // Should increment all counters
    }
    
    #[test]
    fn test_counter_increments() {
        STABLE_IDS_GENERATED_TOTAL.with_label_values(&["rust"]).inc();
        STABLE_IDS_REUSED_TOTAL.with_label_values(&["rust"]).inc_by(5.0);
        
        EMBEDDING_CACHE_HITS_TOTAL.with_label_values(&["rs"]).inc_by(10.0);
        EMBEDDING_CACHE_MISSES_TOTAL.with_label_values(&["rs"]).inc_by(2.0);
    }
    
    #[test]
    fn test_gauge_updates() {
        STABLE_ID_CACHE_SIZE.set(1000.0);
        INDEXER_QUEUE_LENGTH.set(5.0);
        INDEXER_ACTIVE_TASKS.set(2.0);
        
        INDEXER_QUEUE_LENGTH.inc();
        assert!(INDEXER_QUEUE_LENGTH.get() > 0.0);
    }
    
    #[test]
    fn test_task_completion_metrics() {
        INDEXER_TASKS_COMPLETED_TOTAL.with_label_values(&["success"]).inc();
        INDEXER_TASKS_COMPLETED_TOTAL.with_label_values(&["failure"]).inc();
        INDEXER_TASKS_COMPLETED_TOTAL.with_label_values(&["timeout"]).inc();
    }
    
    #[test]
    fn test_backpressure_metrics() {
        INDEXER_BACKPRESSURE_EVENTS_TOTAL.with_label_values(&["queue_full"]).inc();
        INDEXER_TIMEOUT_EVENTS_TOTAL.with_label_values(&["embedding"]).inc();
    }
}
