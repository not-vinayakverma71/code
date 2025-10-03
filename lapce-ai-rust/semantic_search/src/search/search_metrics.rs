// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Search Metrics for Performance Tracking

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

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
    
    /// Record a search query
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
    }
    
    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        self.total_queries.fetch_add(1, Ordering::Relaxed);
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
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
