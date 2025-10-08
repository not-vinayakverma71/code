/// Prometheus metrics export
/// DAY 5 H5-6: Translate Prometheus export

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::metrics_collection::*;

/// Prometheus metric types
#[derive(Debug, Clone)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Prometheus metric
#[derive(Debug, Clone)]
pub struct PrometheusMetric {
    pub name: String,
    pub metric_type: MetricType,
    pub help: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

/// Prometheus exporter
pub struct PrometheusExporter {
    metrics: Arc<RwLock<Vec<PrometheusMetric>>>,
    prefix: String,
}

impl PrometheusExporter {
    pub fn new(prefix: &str) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            prefix: prefix.to_string(),
        }
    }
    
    /// Register a counter metric
    pub async fn register_counter(&self, name: &str, help: &str, value: f64, labels: HashMap<String, String>) {
        let metric = PrometheusMetric {
            name: format!("{}_{}", self.prefix, name),
            metric_type: MetricType::Counter,
            help: help.to_string(),
            value,
            labels,
        };
        self.metrics.write().await.push(metric);
    }
    
    /// Register a gauge metric
    pub async fn register_gauge(&self, name: &str, help: &str, value: f64, labels: HashMap<String, String>) {
        let metric = PrometheusMetric {
            name: format!("{}_{}", self.prefix, name),
            metric_type: MetricType::Gauge,
            help: help.to_string(),
            value,
            labels,
        };
        self.metrics.write().await.push(metric);
    }
    
    /// Register histogram metric
    pub async fn register_histogram(&self, name: &str, help: &str, buckets: Vec<f64>, values: Vec<f64>) {
        for (i, bucket) in buckets.iter().enumerate() {
            let count = values.iter().filter(|v| **v <= *bucket).count() as f64;
            let metric = PrometheusMetric {
                name: format!("{}_{}_bucket", self.prefix, name),
                metric_type: MetricType::Histogram,
                help: help.to_string(),
                value: count,
                labels: vec![("le".to_string(), bucket.to_string())].into_iter().collect(),
            };
            self.metrics.write().await.push(metric);
        }
    }
    
    /// Export metrics in Prometheus format
    pub async fn export(&self) -> String {
        let metrics = self.metrics.read().await;
        let mut output = String::new();
        
        let mut seen_metrics = std::collections::HashSet::new();
        
        for metric in metrics.iter() {
            // Add HELP and TYPE lines once per metric name
            if !seen_metrics.contains(&metric.name) {
                output.push_str(&format!("# HELP {} {}\n", metric.name, metric.help));
                output.push_str(&format!("# TYPE {} {}\n", 
                    metric.name, 
                    match metric.metric_type {
                        MetricType::Counter => "counter",
                        MetricType::Gauge => "gauge",
                        MetricType::Histogram => "histogram",
                        MetricType::Summary => "summary",
                    }
                ));
                seen_metrics.insert(metric.name.clone());
            }
            
            // Format labels
            let labels_str = if metric.labels.is_empty() {
                String::new()
            } else {
                let labels: Vec<String> = metric.labels.iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                format!("{{{}}}", labels.join(","))
            };
            
            // Add metric line
            output.push_str(&format!("{}{} {}\n", metric.name, labels_str, metric.value));
        }
        
        output
    }
    
    /// Export token usage metrics
    pub async fn export_token_usage(&self, usage: &TokenUsage) {
        self.register_counter("tokens_in_total", "Total input tokens", usage.total_tokens_in as f64, HashMap::new()).await;
        self.register_counter("tokens_out_total", "Total output tokens", usage.total_tokens_out as f64, HashMap::new()).await;
        self.register_gauge("cost_total", "Total cost in dollars", usage.total_cost, HashMap::new()).await;
        self.register_gauge("context_tokens", "Current context size in tokens", usage.context_tokens as f64, HashMap::new()).await;
        
        if let Some(cache_writes) = usage.total_cache_writes {
            self.register_counter("cache_writes_total", "Total cache writes", cache_writes as f64, HashMap::new()).await;
        }
        
        if let Some(cache_reads) = usage.total_cache_reads {
            self.register_counter("cache_reads_total", "Total cache reads", cache_reads as f64, HashMap::new()).await;
        }
    }
    
    /// Clear all metrics
    pub async fn clear(&self) {
        self.metrics.write().await.clear();
    }
}

/// System metrics collector
pub struct SystemMetrics {
    exporter: PrometheusExporter,
    start_time: std::time::Instant,
}

impl SystemMetrics {
    pub fn new() -> Self {
        Self {
            exporter: PrometheusExporter::new("lapce_ai"),
            start_time: std::time::Instant::now(),
        }
    }
    
    /// Collect system metrics
    pub async fn collect(&self) {
        // Uptime
        let uptime = self.start_time.elapsed().as_secs_f64();
        self.exporter.register_gauge("uptime_seconds", "System uptime in seconds", uptime, HashMap::new()).await;
        
        // Memory usage (simplified)
        if let Ok(mem_info) = sys_info::mem_info() {
            let used_memory = (mem_info.total - mem_info.free) as f64 / 1024.0 / 1024.0;
            self.exporter.register_gauge("memory_used_mb", "Memory used in MB", used_memory, HashMap::new()).await;
        }
        
        // CPU usage (simplified)
        if let Ok(load) = sys_info::loadavg() {
            self.exporter.register_gauge("cpu_load_1m", "CPU load average 1 minute", load.one, HashMap::new()).await;
        }
    }
    
    /// Export all metrics
    pub async fn export(&self) -> String {
        self.collect().await;
        self.exporter.export().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_prometheus_export() {
        let exporter = PrometheusExporter::new("test");
        
        exporter.register_counter("requests_total", "Total requests", 100.0, HashMap::new()).await;
        
        let mut labels = HashMap::new();
        labels.insert("method".to_string(), "GET".to_string());
        exporter.register_counter("http_requests", "HTTP requests", 50.0, labels).await;
        
        let output = exporter.export().await;
        
        assert!(output.contains("# HELP test_requests_total Total requests"));
        assert!(output.contains("# TYPE test_requests_total counter"));
        assert!(output.contains("test_requests_total 100"));
        assert!(output.contains("test_http_requests{method=\"GET\"} 50"));
    }
    
    #[tokio::test]
    async fn test_token_usage_export() {
        let exporter = PrometheusExporter::new("ai");
        
        let usage = TokenUsage {
            total_tokens_in: 1000,
            total_tokens_out: 500,
            total_cache_writes: Some(100),
            total_cache_reads: Some(200),
            total_cost: 0.05,
            context_tokens: 1500,
        };
        
        exporter.export_token_usage(&usage).await;
        let output = exporter.export().await;
        
        assert!(output.contains("ai_tokens_in_total 1000"));
        assert!(output.contains("ai_tokens_out_total 500"));
        assert!(output.contains("ai_cost_total 0.05"));
        assert!(output.contains("ai_context_tokens 1500"));
        assert!(output.contains("ai_cache_writes_total 100"));
        assert!(output.contains("ai_cache_reads_total 200"));
    }
}
