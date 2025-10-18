// Metrics Tests - SEM-008-B, SEM-010-C
use lancedb::search::search_metrics::{
    SearchMetrics, export_metrics, CACHE_HITS_TOTAL, CACHE_MISSES_TOTAL,
    CACHE_SIZE, SEARCH_LATENCY, AWS_TITAN_REQUEST_LATENCY, MEMORY_RSS_BYTES,
    INDEX_OPERATIONS_TOTAL, INDEX_LATENCY, SEARCH_ERRORS_TOTAL,
};
use std::time::Duration;

#[test]
fn test_cache_hit_counter() {
    let metrics = SearchMetrics::new();
    let initial_hits = CACHE_HITS_TOTAL.get();
    
    metrics.record_cache_hit(Duration::from_millis(5));
    
    assert_eq!(CACHE_HITS_TOTAL.get(), initial_hits + 1.0);
}

#[test]
fn test_cache_miss_counter() {
    let metrics = SearchMetrics::new();
    let initial_misses = CACHE_MISSES_TOTAL.get();
    
    metrics.record_cache_miss();
    
    assert_eq!(CACHE_MISSES_TOTAL.get(), initial_misses + 1.0);
}

#[test]
fn test_cache_size_gauge() {
    let metrics = SearchMetrics::new();
    
    metrics.update_cache_size(100);
    assert_eq!(CACHE_SIZE.get(), 100.0);
    
    metrics.update_cache_size(200);
    assert_eq!(CACHE_SIZE.get(), 200.0);
}

#[test]
fn test_search_latency_histogram() {
    let metrics = SearchMetrics::new();
    
    metrics.record_search(Duration::from_millis(50), 10);
    metrics.record_search(Duration::from_millis(100), 20);
    metrics.record_search(Duration::from_millis(150), 15);
    
    // Verify histogram has recorded values
    let exported = export_metrics();
    assert!(exported.contains("semantic_search_latency_seconds"));
}

#[test]
fn test_aws_titan_metrics() {
    let metrics = SearchMetrics::new();
    
    metrics.record_aws_titan_request(Duration::from_millis(200), "create_embeddings");
    metrics.record_aws_titan_error("throttling");
    
    let exported = export_metrics();
    assert!(exported.contains("aws_titan_request_latency_seconds"));
    assert!(exported.contains("aws_titan_errors_total"));
}

#[test]
fn test_memory_rss_gauge() {
    let metrics = SearchMetrics::new();
    
    metrics.update_memory_rss(1024 * 1024 * 100); // 100MB
    
    let exported = export_metrics();
    assert!(exported.contains("semantic_search_memory_rss_bytes"));
}

#[test]
fn test_metrics_export_format() {
    let metrics = SearchMetrics::new();
    
    // Perform various operations
    metrics.record_cache_hit(Duration::from_millis(5));
    metrics.record_cache_miss();
    metrics.update_cache_size(50);
    metrics.record_search(Duration::from_millis(100), 10);
    
    let exported = export_metrics();
    
    // Verify Prometheus format
    assert!(exported.contains("# HELP"));
    assert!(exported.contains("# TYPE"));
    assert!(exported.contains("semantic_search_cache_hits_total"));
    assert!(exported.contains("semantic_search_cache_misses_total"));
    assert!(exported.contains("semantic_search_cache_size"));
}

#[test]
fn test_metrics_summary() {
    let metrics = SearchMetrics::new();
    
    // Record some operations
    metrics.record_cache_hit(Duration::from_millis(5));
    metrics.record_cache_hit(Duration::from_millis(10));
    metrics.record_search(Duration::from_millis(100), 10);
    metrics.record_indexing(5, 20, Duration::from_millis(500));
    
    let summary = metrics.summary();
    
    assert_eq!(summary.total_queries, 3); // 2 cache hits + 1 search
    assert!(summary.cache_hit_rate > 0.0);
    assert_eq!(summary.files_indexed, 5);
    assert_eq!(summary.chunks_created, 20);
}

#[test]
fn test_metrics_reset() {
    let metrics = SearchMetrics::new();
    
    // Record operations
    metrics.record_cache_hit(Duration::from_millis(5));
    metrics.record_search(Duration::from_millis(100), 10);
    metrics.record_indexing(5, 20, Duration::from_millis(500));
    
    // Reset
    metrics.reset();
    
    let summary = metrics.summary();
    assert_eq!(summary.total_queries, 0);
    assert_eq!(summary.files_indexed, 0);
    assert_eq!(summary.chunks_created, 0);
}

#[test]
fn test_all_metrics_present_in_export() {
    let metrics = SearchMetrics::new();
    
    // Perform various operations to populate metrics
    metrics.record_cache_hit(Duration::from_millis(5));
    metrics.record_cache_miss();
    metrics.update_cache_size(50);
    metrics.record_search(Duration::from_millis(100), 10);
    metrics.record_aws_titan_request(Duration::from_millis(200), "embeddings");
    metrics.record_aws_titan_error("throttling");
    metrics.update_memory_rss(100 * 1024 * 1024);
    
    let exported = export_metrics();
    
    // Verify all key metrics are present
    assert!(exported.contains("semantic_search_cache_hits_total"), "Cache hits metric missing");
    assert!(exported.contains("semantic_search_cache_misses_total"), "Cache misses metric missing");
    assert!(exported.contains("semantic_search_cache_size"), "Cache size metric missing");
    assert!(exported.contains("semantic_search_latency_seconds"), "Search latency metric missing");
    assert!(exported.contains("semantic_search_cache_hit_latency_seconds"), "Cache hit latency metric missing");
    assert!(exported.contains("aws_titan_request_latency_seconds"), "AWS Titan latency metric missing");
    assert!(exported.contains("aws_titan_errors_total"), "AWS Titan errors metric missing");
    assert!(exported.contains("semantic_search_memory_rss_bytes"), "Memory RSS metric missing");
    assert!(exported.contains("semantic_search_index_operations_total"), "Index operations metric missing");
    assert!(exported.contains("semantic_search_index_latency_seconds"), "Index latency metric missing");
    assert!(exported.contains("semantic_search_errors_total"), "Search errors metric missing");
}

#[test]
fn test_prometheus_format_compliance() {
    let metrics = SearchMetrics::new();
    
    // Trigger some metrics
    metrics.record_search(Duration::from_millis(50), 5);
    metrics.update_cache_size(100);
    
    let exported = export_metrics();
    
    // Verify Prometheus format
    let lines: Vec<&str> = exported.lines().collect();
    
    for line in lines {
        if line.starts_with("#") {
            // Comment lines should be HELP or TYPE
            assert!(
                line.starts_with("# HELP") || line.starts_with("# TYPE"),
                "Invalid comment line: {}",
                line
            );
        } else if !line.is_empty() {
            // Metric lines should have name and value
            assert!(
                line.contains(' ') || line.contains('\t'),
                "Invalid metric line format: {}",
                line
            );
        }
    }
}

#[test]
fn test_metric_labels() {
    let metrics = SearchMetrics::new();
    
    // Record operations with labels
    metrics.record_aws_titan_request(Duration::from_millis(100), "create_embeddings");
    metrics.record_aws_titan_error("rate_limit");
    
    let exported = export_metrics();
    
    // Check for labeled metrics
    assert!(exported.contains("operation=\"create_embeddings\""), "Operation label missing");
    assert!(exported.contains("error_type=\"rate_limit\""), "Error type label missing");
}
