/// LSP Gateway Metrics (LSP-017)
/// p50/p95/p99 latency histograms per LSP method, parse time metrics

use prometheus::{
    Registry, HistogramVec, IntCounterVec, IntGaugeVec, Histogram,
    HistogramOpts, Opts, exponential_buckets,
};
use std::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    /// Global metrics registry
    static ref REGISTRY: Registry = Registry::new();
    
    /// LSP request latency histogram (p50, p95, p99)
    /// Labels: method (textDocument/definition, etc.)
    static ref LSP_REQUEST_DURATION: HistogramVec = {
        let opts = HistogramOpts::new(
            "lsp_request_duration_seconds",
            "LSP request processing duration in seconds"
        ).buckets(exponential_buckets(0.0001, 2.0, 15).unwrap()); // 0.1ms to ~3.2s
        
        let histogram = HistogramVec::new(opts, &["method"]).unwrap();
        REGISTRY.register(Box::new(histogram.clone())).unwrap();
        histogram
    };
    
    /// LSP request count
    /// Labels: method, status (success/error)
    static ref LSP_REQUEST_COUNT: IntCounterVec = {
        let opts = Opts::new(
            "lsp_request_total",
            "Total number of LSP requests"
        );
        let counter = IntCounterVec::new(opts, &["method", "status"]).unwrap();
        REGISTRY.register(Box::new(counter.clone())).unwrap();
        counter
    };
    
    /// Parse time histogram
    /// Labels: language_id
    static ref PARSE_DURATION: HistogramVec = {
        let opts = HistogramOpts::new(
            "lsp_parse_duration_seconds",
            "Document parsing duration in seconds"
        ).buckets(exponential_buckets(0.001, 2.0, 12).unwrap()); // 1ms to ~4s
        
        let histogram = HistogramVec::new(opts, &["language_id"]).unwrap();
        REGISTRY.register(Box::new(histogram.clone())).unwrap();
        histogram
    };
    
    /// Symbol index size
    static ref SYMBOL_INDEX_SIZE: IntGaugeVec = {
        let opts = Opts::new(
            "lsp_symbol_index_size",
            "Number of symbols in the index"
        );
        let gauge = IntGaugeVec::new(opts, &["type"]).unwrap();
        REGISTRY.register(Box::new(gauge.clone())).unwrap();
        gauge
    };
    
    /// Document count
    static ref DOCUMENT_COUNT: IntGaugeVec = {
        let opts = Opts::new(
            "lsp_document_count",
            "Number of open documents"
        );
        let gauge = IntGaugeVec::new(opts, &["language_id"]).unwrap();
        REGISTRY.register(Box::new(gauge.clone())).unwrap();
        gauge
    };
    
    /// Error count
    /// Labels: method, error_type
    static ref ERROR_COUNT: IntCounterVec = {
        let opts = Opts::new(
            "lsp_error_total",
            "Total number of LSP errors"
        );
        let counter = IntCounterVec::new(opts, &["method", "error_type"]).unwrap();
        REGISTRY.register(Box::new(counter.clone())).unwrap();
        counter
    };
    
    /// Diagnostics count
    /// Labels: language_id, severity
    static ref DIAGNOSTICS_COUNT: IntCounterVec = {
        let opts = Opts::new(
            "lsp_diagnostics_total",
            "Total number of diagnostics published"
        );
        let counter = IntCounterVec::new(opts, &["language_id", "severity"]).unwrap();
        REGISTRY.register(Box::new(counter.clone())).unwrap();
        counter
    };
    
    /// File watcher event count
    /// Labels: event_type (create, modify, delete, rename)
    static ref FILE_WATCHER_EVENTS: IntCounterVec = {
        let opts = Opts::new(
            "lsp_file_watcher_events_total",
            "Total number of file system events"
        );
        let counter = IntCounterVec::new(opts, &["event_type"]).unwrap();
        REGISTRY.register(Box::new(counter.clone())).unwrap();
        counter
    };
    
    /// Memory usage in bytes (RSS)
    static ref MEMORY_USAGE: IntGaugeVec = {
        let opts = Opts::new(
            "lsp_memory_bytes",
            "Memory usage in bytes"
        );
        let gauge = IntGaugeVec::new(opts, &["type"]).unwrap();
        REGISTRY.register(Box::new(gauge.clone())).unwrap();
        gauge
    };
}

/// LSP Gateway metrics collector
pub struct LspMetrics;

impl LspMetrics {
    /// Record request duration
    pub fn record_request_duration(method: &str, duration_secs: f64) {
        LSP_REQUEST_DURATION
            .with_label_values(&[method])
            .observe(duration_secs);
    }
    
    /// Increment request count
    pub fn inc_request_count(method: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        LSP_REQUEST_COUNT
            .with_label_values(&[method, status])
            .inc();
    }
    
    /// Record parse duration
    pub fn record_parse_duration(language_id: &str, duration_secs: f64) {
        PARSE_DURATION
            .with_label_values(&[language_id])
            .observe(duration_secs);
    }
    
    /// Set symbol index size
    pub fn set_symbol_index_size(definitions: i64, references: i64) {
        SYMBOL_INDEX_SIZE
            .with_label_values(&["definitions"])
            .set(definitions);
        SYMBOL_INDEX_SIZE
            .with_label_values(&["references"])
            .set(references);
    }
    
    /// Set document count
    pub fn set_document_count(language_id: &str, count: i64) {
        DOCUMENT_COUNT
            .with_label_values(&[language_id])
            .set(count);
    }
    
    /// Increment error count
    pub fn inc_error_count(method: &str, error_type: &str) {
        ERROR_COUNT
            .with_label_values(&[method, error_type])
            .inc();
    }
    
    /// Increment diagnostics count
    pub fn inc_diagnostics_count(language_id: &str, severity: &str, count: u64) {
        DIAGNOSTICS_COUNT
            .with_label_values(&[language_id, severity])
            .inc_by(count);
    }
    
    /// Increment file watcher event count
    pub fn inc_file_watcher_event(event_type: &str) {
        FILE_WATCHER_EVENTS
            .with_label_values(&[event_type])
            .inc();
    }
    
    /// Set memory usage
    pub fn set_memory_usage(type_label: &str, bytes: i64) {
        MEMORY_USAGE
            .with_label_values(&[type_label])
            .set(bytes);
    }
    
    /// Get Prometheus metrics text format
    pub fn metrics_text() -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = REGISTRY.gather();
        
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        
        String::from_utf8(buffer).unwrap()
    }
    
    /// Get metrics registry for integration with existing metrics server
    pub fn registry() -> &'static Registry {
        &REGISTRY
    }
}

/// RAII timer for automatic duration recording
pub struct RequestTimer {
    method: String,
    start: std::time::Instant,
}

impl RequestTimer {
    pub fn new(method: &str) -> Self {
        Self {
            method: method.to_string(),
            start: std::time::Instant::now(),
        }
    }
    
    pub fn finish(self, success: bool) {
        let duration = self.start.elapsed().as_secs_f64();
        LspMetrics::record_request_duration(&self.method, duration);
        LspMetrics::inc_request_count(&self.method, success);
    }
}

impl Drop for RequestTimer {
    fn drop(&mut self) {
        // If not explicitly finished, record as error
        let duration = self.start.elapsed().as_secs_f64();
        LspMetrics::record_request_duration(&self.method, duration);
        LspMetrics::inc_request_count(&self.method, false);
    }
}

/// Parse timer
pub struct ParseTimer {
    language_id: String,
    start: std::time::Instant,
}

impl ParseTimer {
    pub fn new(language_id: &str) -> Self {
        Self {
            language_id: language_id.to_string(),
            start: std::time::Instant::now(),
        }
    }
}

impl Drop for ParseTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        LspMetrics::record_parse_duration(&self.language_id, duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_recording() {
        LspMetrics::record_request_duration("textDocument/definition", 0.015);
        LspMetrics::inc_request_count("textDocument/definition", true);
        LspMetrics::record_parse_duration("rust", 0.050);
        LspMetrics::set_symbol_index_size(1000, 5000);
        LspMetrics::set_document_count("rust", 10);
        
        let metrics = LspMetrics::metrics_text();
        assert!(metrics.contains("lsp_request_duration_seconds"));
        assert!(metrics.contains("lsp_parse_duration_seconds"));
        assert!(metrics.contains("lsp_symbol_index_size"));
    }
    
    #[test]
    fn test_request_timer() {
        {
            let timer = RequestTimer::new("textDocument/hover");
            std::thread::sleep(std::time::Duration::from_millis(10));
            timer.finish(true);
        }
        
        let metrics = LspMetrics::metrics_text();
        assert!(metrics.contains("textDocument/hover"));
    }
    
    #[test]
    fn test_parse_timer() {
        {
            let _timer = ParseTimer::new("typescript");
            std::thread::sleep(std::time::Duration::from_millis(5));
            // Auto-drops
        }
        
        let metrics = LspMetrics::metrics_text();
        assert!(metrics.contains("typescript"));
    }
}
