/// Health Monitoring and Endpoints Module
/// Provides health check endpoints and monitoring infrastructure

use std::sync::Arc;
use std::time::{Duration, Instant};
use axum::{
    routing::{get, post},
    Router,
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use prometheus::{Encoder, TextEncoder, Counter, Gauge, Histogram, HistogramOpts};
use tokio::sync::RwLock;
use anyhow::Result;

use crate::ipc::ipc_server::IpcServer;
use crate::ipc_config::IpcConfig;

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub connections: ConnectionHealth,
    pub memory: MemoryHealth,
    pub providers: ProvidersHealth,
    pub performance: PerformanceHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHealth {
    pub active: usize,
    pub total: usize,
    pub max_allowed: usize,
    pub utilization_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHealth {
    pub used_mb: f64,
    pub limit_mb: f64,
    pub buffer_pool_size: usize,
    pub cache_size_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersHealth {
    pub active_providers: Vec<String>,
    pub healthy: usize,
    pub unhealthy: usize,
    pub circuit_breakers_open: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceHealth {
    pub avg_latency_us: f64,
    pub p99_latency_us: f64,
    pub throughput_msg_sec: f64,
    pub error_rate_percent: f64,
}

/// Monitoring metrics
pub struct MonitoringMetrics {
    // Counters
    pub requests_total: Counter,
    pub errors_total: Counter,
    pub bytes_received: Counter,
    pub bytes_sent: Counter,
    
    // Gauges
    pub active_connections: Gauge,
    pub memory_usage_bytes: Gauge,
    pub buffer_pool_free: Gauge,
    pub cache_hit_ratio: Gauge,
    
    // Histograms
    pub request_duration: Histogram,
    pub message_size: Histogram,
    pub queue_depth: Histogram,
}

impl MonitoringMetrics {
    pub fn new() -> Self {
        Self {
            requests_total: Counter::new("ipc_requests_total", "Total IPC requests").unwrap(),
            errors_total: Counter::new("ipc_errors_total", "Total IPC errors").unwrap(),
            bytes_received: Counter::new("ipc_bytes_received", "Total bytes received").unwrap(),
            bytes_sent: Counter::new("ipc_bytes_sent", "Total bytes sent").unwrap(),
            
            active_connections: Gauge::new("ipc_active_connections", "Active connections").unwrap(),
            memory_usage_bytes: Gauge::new("ipc_memory_bytes", "Memory usage in bytes").unwrap(),
            buffer_pool_free: Gauge::new("ipc_buffer_pool_free", "Free buffers in pool").unwrap(),
            cache_hit_ratio: Gauge::new("ipc_cache_hit_ratio", "Cache hit ratio").unwrap(),
            
            request_duration: Histogram::with_opts(
                HistogramOpts::new("ipc_request_duration_seconds", "Request duration")
                    .buckets(vec![0.0001, 0.001, 0.01, 0.1, 1.0])
            ).unwrap(),
            message_size: Histogram::with_opts(
                HistogramOpts::new("ipc_message_size_bytes", "Message size")
                    .buckets(vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0])
            ).unwrap(),
            queue_depth: Histogram::with_opts(
                HistogramOpts::new("ipc_queue_depth", "Queue depth")
                    .buckets(vec![1.0, 10.0, 50.0, 100.0, 500.0, 1000.0])
            ).unwrap(),
        }
    }
    
    pub fn register_all(&self) -> Result<()> {
        prometheus::register(Box::new(self.requests_total.clone()))?;
        prometheus::register(Box::new(self.errors_total.clone()))?;
        prometheus::register(Box::new(self.bytes_received.clone()))?;
        prometheus::register(Box::new(self.bytes_sent.clone()))?;
        
        prometheus::register(Box::new(self.active_connections.clone()))?;
        prometheus::register(Box::new(self.memory_usage_bytes.clone()))?;
        prometheus::register(Box::new(self.buffer_pool_free.clone()))?;
        prometheus::register(Box::new(self.cache_hit_ratio.clone()))?;
        
        prometheus::register(Box::new(self.request_duration.clone()))?;
        prometheus::register(Box::new(self.message_size.clone()))?;
        prometheus::register(Box::new(self.queue_depth.clone()))?;
        
        Ok(())
    }
}

/// Health monitoring service
pub struct HealthMonitor {
    config: Arc<IpcConfig>,
    start_time: Instant,
    metrics: Arc<MonitoringMetrics>,
    last_check: Arc<RwLock<Instant>>,
    health_status: Arc<RwLock<HealthResponse>>,
}

impl HealthMonitor {
    pub fn new(config: Arc<IpcConfig>) -> Self {
        let metrics = Arc::new(MonitoringMetrics::new());
        metrics.register_all().unwrap();
        
        Self {
            config,
            start_time: Instant::now(),
            metrics,
            last_check: Arc::new(RwLock::new(Instant::now())),
            health_status: Arc::new(RwLock::new(Self::default_health())),
        }
    }
    
    fn default_health() -> HealthResponse {
        HealthResponse {
            status: "healthy".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            uptime_seconds: 0,
            connections: ConnectionHealth {
                active: 0,
                total: 0,
                max_allowed: 1000,
                utilization_percent: 0.0,
            },
            memory: MemoryHealth {
                used_mb: 0.0,
                limit_mb: 3.0,
                buffer_pool_size: 100,
                cache_size_mb: 0.0,
            },
            providers: ProvidersHealth {
                active_providers: vec![],
                healthy: 0,
                unhealthy: 0,
                circuit_breakers_open: 0,
            },
            performance: PerformanceHealth {
                avg_latency_us: 0.0,
                p99_latency_us: 0.0,
                throughput_msg_sec: 0.0,
                error_rate_percent: 0.0,
            },
        }
    }
    
    /// Update health status
    pub async fn update_health(&self, active_connections: usize, memory_mb: f64) {
        let mut health = self.health_status.write().await;
        
        health.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        health.uptime_seconds = self.start_time.elapsed().as_secs();
        
        // Update connection health
        health.connections.active = active_connections;
        health.connections.total = active_connections; // TODO: track total
        health.connections.utilization_percent = 
            (active_connections as f64 / health.connections.max_allowed as f64) * 100.0;
        
        // Update memory health
        health.memory.used_mb = memory_mb;
        
        // Determine overall status
        if health.connections.utilization_percent > 90.0 {
            health.status = "degraded".to_string();
        } else if health.memory.used_mb > 2.5 {
            health.status = "degraded".to_string();
        } else {
            health.status = "healthy".to_string();
        }
        
        // Update metrics
        self.metrics.active_connections.set(active_connections as f64);
        self.metrics.memory_usage_bytes.set(memory_mb * 1024.0 * 1024.0);
        
        *self.last_check.write().await = Instant::now();
    }
    
    /// Get current health status
    pub async fn get_health(&self) -> HealthResponse {
        self.health_status.read().await.clone()
    }
    
    /// Start health monitoring server
    pub async fn start_server(self: Arc<Self>) -> Result<()> {
        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/health/live", get(liveness_handler))
            .route("/health/ready", get(readiness_handler))
            .route("/metrics", get(metrics_handler))
            .with_state(self.clone());
        
        let addr = format!("0.0.0.0:{}", self.config.monitoring.health_check_port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        
        tracing::info!("Health monitoring server listening on {}", addr);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}

/// Health check endpoint handler
async fn health_handler(State(monitor): State<Arc<HealthMonitor>>) -> Json<HealthResponse> {
    Json(monitor.get_health().await)
}

/// Kubernetes liveness probe
async fn liveness_handler(State(_monitor): State<Arc<HealthMonitor>>) -> StatusCode {
    // Always return OK if the server is running
    StatusCode::OK
}

/// Kubernetes readiness probe
async fn readiness_handler(State(monitor): State<Arc<HealthMonitor>>) -> StatusCode {
    let health = monitor.get_health().await;
    
    if health.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

/// Prometheus metrics endpoint
async fn metrics_handler() -> Result<String, StatusCode> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    
    encoder.encode(&metric_families, &mut buffer)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    String::from_utf8(buffer).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Grafana dashboard generator
pub fn generate_grafana_dashboard() -> serde_json::Value {
    serde_json::json!({
        "dashboard": {
            "title": "Lapce AI IPC Monitoring",
            "panels": [
                {
                    "title": "Request Rate",
                    "type": "graph",
                    "targets": [
                        {
                            "expr": "rate(ipc_requests_total[5m])"
                        }
                    ]
                },
                {
                    "title": "Active Connections",
                    "type": "graph",
                    "targets": [
                        {
                            "expr": "ipc_active_connections"
                        }
                    ]
                },
                {
                    "title": "Memory Usage",
                    "type": "graph",
                    "targets": [
                        {
                            "expr": "ipc_memory_bytes / 1024 / 1024"
                        }
                    ]
                },
                {
                    "title": "Request Latency",
                    "type": "graph",
                    "targets": [
                        {
                            "expr": "histogram_quantile(0.99, rate(ipc_request_duration_seconds_bucket[5m]))"
                        }
                    ]
                },
                {
                    "title": "Error Rate",
                    "type": "graph",
                    "targets": [
                        {
                            "expr": "rate(ipc_errors_total[5m])"
                        }
                    ]
                },
                {
                    "title": "Message Size Distribution",
                    "type": "heatmap",
                    "targets": [
                        {
                            "expr": "rate(ipc_message_size_bytes_bucket[5m])"
                        }
                    ]
                }
            ]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_health_monitor() {
        let config = Arc::new(IpcConfig::default());
        let monitor = Arc::new(HealthMonitor::new(config));
        
        // Update health
        monitor.update_health(10, 1.5).await;
        
        // Get health
        let health = monitor.get_health().await;
        assert_eq!(health.status, "healthy");
        assert_eq!(health.connections.active, 10);
        assert_eq!(health.memory.used_mb, 1.5);
    }
    
    #[test]
    fn test_grafana_dashboard() {
        let dashboard = generate_grafana_dashboard();
        assert!(dashboard["dashboard"]["title"].is_string());
        assert!(dashboard["dashboard"]["panels"].is_array());
    }
}
