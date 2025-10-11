// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Search Metrics for Performance Tracking

use prometheus::{
    register_counter, register_counter_vec, register_gauge, register_histogram_vec,
    Counter, CounterVec, Gauge, HistogramVec, TextEncoder, Encoder,
};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use lazy_static::lazy_static;

lazy_static! {
    // Cache metrics
    pub static ref CACHE_HITS_TOTAL: Counter = register_counter!(
        "semantic_search_cache_hits_total",
        "Total number of cache hits"
    ).unwrap();
    
    pub static ref CACHE_MISSES_TOTAL: Counter = register_counter!(
        "semantic_search_cache_misses_total",
        "Total number of cache misses"
    ).unwrap();
    
    pub static ref CACHE_SIZE: Gauge = register_gauge!(
        "semantic_search_cache_size",
        "Current cache size in entries"
    ).unwrap();
    
    pub static ref CACHE_HIT_LATENCY: HistogramVec = register_histogram_vec!(
        "semantic_search_cache_hit_latency_seconds",
        "Cache hit latency in seconds",
        &["operation"]
    ).unwrap();
    
    // Search metrics
    pub static ref SEARCH_LATENCY: HistogramVec = register_histogram_vec!(
        "semantic_search_latency_seconds",
        "Search latency in seconds",
        &["operation"]
    ).unwrap();
    
    pub static ref SEARCH_ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "semantic_search_errors_total",
        "Total number of search errors",
        &["error_type"]
    ).unwrap();
    
    // AWS Titan metrics
    pub static ref AWS_TITAN_REQUEST_LATENCY: HistogramVec = register_histogram_vec!(
        "aws_titan_request_latency_seconds",
        "AWS Titan request latency in seconds",
        &["operation"]
    ).unwrap();
    
    pub static ref AWS_TITAN_ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "aws_titan_errors_total",
        "Total number of AWS Titan errors",
        &["error_type"]
    ).unwrap();
    
    // Memory metrics
    pub static ref MEMORY_RSS_BYTES: Gauge = register_gauge!(
        "semantic_search_memory_rss_bytes",
        "Resident set size in bytes"
    ).unwrap();
    
    // Indexing metrics
    pub static ref INDEX_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        "semantic_search_index_operations_total",
        "Total number of index operations",
        &["operation"]
    ).unwrap();
    
    pub static ref INDEX_LATENCY: HistogramVec = register_histogram_vec!(
        "semantic_search_index_latency_seconds",
        "Index operation latency in seconds",
        &["operation"]
    ).unwrap();
}

// CST canonical mapping metrics (separate block for conditional compilation)
#[cfg(feature = "cst_ts")]
lazy_static! {
    pub static ref CANONICAL_MAPPING_APPLIED_TOTAL: CounterVec = register_counter_vec!(
        "canonical_mapping_applied_total",
        "Total number of canonical mappings applied",
        &["language"]
    ).unwrap();
    
    pub static ref CANONICAL_MAPPING_UNKNOWN_TOTAL: CounterVec = register_counter_vec!(
        "canonical_mapping_unknown_total",
        "Total number of unknown canonical mappings encountered",
        &["language"]
    ).unwrap();
}

/// Metrics for tracking search performance
pub struct SearchMetrics {
    // Query metrics
    total_queries: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    
    // Performance metrics
    total_query_time_ms: AtomicU64,
    total_results_returned: AtomicU64,
    
    // Indexing metrics
    files_indexed: AtomicUsize,
    chunks_created: AtomicUsize,
    total_index_time_ms: AtomicU64,
    
    // Error tracking
    query_errors: AtomicU64,
    index_errors: AtomicU64,
}

impl SearchMetrics {
    pub fn new() -> Self {
        Self {
            total_queries: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            total_query_time_ms: AtomicU64::new(0),
            total_results_returned: AtomicU64::new(0),
            files_indexed: AtomicUsize::new(0),
            chunks_created: AtomicUsize::new(0),
            total_index_time_ms: AtomicU64::new(0),
            query_errors: AtomicU64::new(0),
            index_errors: AtomicU64::new(0),
        }
    }
    
    /// Record a search query (cache miss path)
    pub fn record_search(&self, duration: Duration, results_count: usize) {
        self.total_queries.fetch_add(1, Ordering::Relaxed);
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        self.total_query_time_ms.fetch_add(
            duration.as_millis() as u64,
            Ordering::Relaxed
        );
        self.total_results_returned.fetch_add(
            results_count as u64,
            Ordering::Relaxed
        );
        
        // Update Prometheus metrics - only once for cache miss
        CACHE_MISSES_TOTAL.inc();
        SEARCH_LATENCY.with_label_values(&["search"]).observe(duration.as_secs_f64());
    }
    
    /// Record a cache hit
    pub fn record_cache_hit(&self, duration: Duration) {
        self.total_queries.fetch_add(1, Ordering::Relaxed);
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
        
        // Update Prometheus metrics
        CACHE_HITS_TOTAL.inc();
        CACHE_HIT_LATENCY.with_label_values(&["get"]).observe(duration.as_secs_f64());
    }
    
    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        CACHE_MISSES_TOTAL.inc();
    }
    
    /// Update cache size
    pub fn update_cache_size(&self, size: usize) {
        CACHE_SIZE.set(size as f64);
    }
    
    /// Record AWS Titan request with PII redaction
    pub fn record_aws_titan_request(&self, duration: Duration, operation: &str) {
        let safe_operation = crate::security::redaction::redact_pii(operation);
        AWS_TITAN_REQUEST_LATENCY
            .with_label_values(&[&safe_operation])
            .observe(duration.as_secs_f64());
    }
    
    /// Record AWS Titan error with PII redaction
    pub fn record_aws_titan_error(&self, error_type: &str) {
        let safe_error = crate::security::redaction::redact_pii(error_type);
        AWS_TITAN_ERRORS_TOTAL
            .with_label_values(&[&safe_error])
            .inc();
    }
    
    /// Update memory RSS
    pub fn update_memory_rss(&self, bytes: usize) {
        MEMORY_RSS_BYTES.set(bytes as f64);
    }
    
    /// Record indexing operation
    pub fn record_indexing(&self, files: usize, chunks: usize, duration: Duration) {
        self.files_indexed.fetch_add(files, Ordering::Relaxed);
        self.chunks_created.fetch_add(chunks, Ordering::Relaxed);
        self.total_index_time_ms.fetch_add(
            duration.as_millis() as u64,
            Ordering::Relaxed
        );
    }
    
    /// Record query error
    pub fn record_query_error(&self) {
        self.query_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record search error with PII redaction
    pub fn record_error(&self, error_type: &str) {
        self.query_errors.fetch_add(1, Ordering::Relaxed);
        let safe_error = crate::security::redaction::redact_pii(error_type);
        SEARCH_ERRORS_TOTAL
            .with_label_values(&[&safe_error])
            .inc();
    }
    
    /// Record index error
    pub fn record_index_error(&self) {
        self.index_errors.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get metrics summary
    pub fn summary(&self) -> MetricsSummary {
        let total_queries = self.total_queries.load(Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_hit_rate = if total_queries > 0 {
            (cache_hits as f64 / total_queries as f64) * 100.0
        } else {
            0.0
        };
        
        let avg_query_time = if total_queries > 0 {
            self.total_query_time_ms.load(Ordering::Relaxed) as f64 / total_queries as f64
        } else {
            0.0
        };
        
        MetricsSummary {
            total_queries,
            cache_hit_rate,
            avg_query_time_ms: avg_query_time,
            total_results: self.total_results_returned.load(Ordering::Relaxed),
            files_indexed: self.files_indexed.load(Ordering::Relaxed),
            chunks_created: self.chunks_created.load(Ordering::Relaxed),
            query_errors: self.query_errors.load(Ordering::Relaxed),
            index_errors: self.index_errors.load(Ordering::Relaxed),
        }
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        self.total_queries.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.total_query_time_ms.store(0, Ordering::Relaxed);
        self.total_results_returned.store(0, Ordering::Relaxed);
        self.files_indexed.store(0, Ordering::Relaxed);
        self.chunks_created.store(0, Ordering::Relaxed);
        self.total_index_time_ms.store(0, Ordering::Relaxed);
        self.query_errors.store(0, Ordering::Relaxed);
        self.index_errors.store(0, Ordering::Relaxed);
    }
}

/// Metrics summary for reporting
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub total_queries: u64,
    pub cache_hit_rate: f64,
    pub avg_query_time_ms: f64,
    pub total_results: u64,
    pub files_indexed: usize,
    pub chunks_created: usize,
    pub query_errors: u64,
    pub index_errors: u64,
}

impl std::fmt::Display for MetricsSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Search Metrics:\n\
             - Total Queries: {}\n\
             - Cache Hit Rate: {:.2}%\n\
             - Avg Query Time: {:.2}ms\n\
             - Total Results: {}\n\
             - Files Indexed: {}\n\
             - Chunks Created: {}\n\
             - Query Errors: {}\n\
             - Index Errors: {}",
            self.total_queries,
            self.cache_hit_rate,
            self.avg_query_time_ms,
            self.total_results,
            self.files_indexed,
            self.chunks_created,
            self.query_errors,
            self.index_errors,
        )
    }
}

/// Export metrics in Prometheus format
pub fn export_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// Timer guard for recording operation duration
pub struct TimerGuard<'a> {
    start: Instant,
    histogram: &'a HistogramVec,
    labels: Vec<&'a str>,
}

impl<'a> TimerGuard<'a> {
    pub fn new(histogram: &'a HistogramVec, labels: Vec<&'a str>) -> Self {
        Self {
            start: Instant::now(),
            histogram,
            labels,
        }
    }
}

impl<'a> Drop for TimerGuard<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.histogram
            .with_label_values(&self.labels)
            .observe(duration.as_secs_f64());
    }
}
