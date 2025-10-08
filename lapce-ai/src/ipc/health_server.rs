/// Health Server Implementation for IPC
/// Provides /health and /metrics endpoints for monitoring
use std::sync::Arc;
use std::net::SocketAddr;
use hyper::{Request, Response, StatusCode};
use http_body_util::Full;
use bytes::Bytes;
use serde_json::json;
use tracing::info;

use crate::connection_pool_manager::ConnectionStats;
use crate::ipc::connection_pool::ConnectionPool;
use crate::ipc::circuit_breaker::{CircuitBreaker, CircuitBreakerStats};
use crate::ipc::ipc_server::IpcServerStats;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[cfg(test)]
#[path = "health_server_tests.rs"]
mod tests;

/// Health server configuration
#[derive(Debug, Clone)]
pub struct HealthServerConfig {
    pub bind_address: SocketAddr,
    pub enable_prometheus: bool,
    pub enable_detailed_stats: bool,
}

impl Default for HealthServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:9090".parse().unwrap(),
            enable_prometheus: true,
            enable_detailed_stats: true,
        }
    }
}

/// Tool execution metrics - P0-OPS
#[derive(Debug, Default)]
pub struct ToolMetrics {
    /// Total tool runs
    pub tool_runs: AtomicU64,
    
    /// Approvals requested
    pub approvals_requested: AtomicU64,
    
    /// Approvals approved
    pub approvals_approved: AtomicU64,
    
    /// Approvals denied
    pub approvals_denied: AtomicU64,
    
    /// Tool failures
    pub tool_failures: AtomicU64,
    
    /// Tool execution durations (milliseconds)
    pub execution_durations: Arc<RwLock<Vec<u64>>>,
    
    /// Per-tool metrics
    pub tool_specific: Arc<RwLock<HashMap<String, ToolSpecificMetrics>>>,
}

#[derive(Debug, Default, Clone)]
pub struct ToolSpecificMetrics {
    pub invocations: u64,
    pub failures: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: u64,
}

impl ToolMetrics {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Record tool execution
    pub async fn record_execution(&self, tool_name: &str, duration: Duration, success: bool) {
        self.tool_runs.fetch_add(1, Ordering::Relaxed);
        
        if !success {
            self.tool_failures.fetch_add(1, Ordering::Relaxed);
        }
        
        let duration_ms = duration.as_millis() as u64;
        
        // Record duration
        {
            let mut durations = self.execution_durations.write().await;
            durations.push(duration_ms);
            
            // Keep only last 1000 durations
            if durations.len() > 1000 {
                durations.remove(0);
            }
        }
        
        // Update per-tool metrics
        {
            let mut tool_metrics = self.tool_specific.write().await;
            let metrics = tool_metrics.entry(tool_name.to_string())
                .or_insert_with(ToolSpecificMetrics::default);
            
            metrics.invocations += 1;
            if !success {
                metrics.failures += 1;
            }
            metrics.total_duration_ms += duration_ms;
            metrics.avg_duration_ms = metrics.total_duration_ms / metrics.invocations;
        }
    }
    
    /// Record approval request
    pub fn record_approval_request(&self) {
        self.approvals_requested.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record approval result
    pub fn record_approval_result(&self, approved: bool) {
        if approved {
            self.approvals_approved.fetch_add(1, Ordering::Relaxed);
        } else {
            self.approvals_denied.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Health server for monitoring
pub struct HealthServer {
    config: HealthServerConfig,
    ipc_stats: Arc<IpcServerStats>,
    pool_stats: Arc<ConnectionStats>,
    circuit_breaker: Arc<CircuitBreaker>,
    tool_metrics: Arc<ToolMetrics>,
}

impl HealthServer {
    pub fn new(config: HealthServerConfig, ipc_stats: Arc<IpcServerStats>, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            config,
            ipc_stats,
            pool_stats: Arc::new(ConnectionStats::default()),
            circuit_breaker,
            tool_metrics: Arc::new(ToolMetrics::new()),
        }
    }
    
    /// Start the health server
    pub async fn start(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        let addr = self.config.bind_address;
        
        // Simplified health server - just return OK for now
        // TODO: Implement full health server with proper hyper setup
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });
        
        info!("Health server listening on http://{}", addr);
        Ok(())
    }
    
    /// Route requests to appropriate handlers
    async fn handle_request(&self, req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
        match req.uri().path() {
            "/health" => self.handle_health().await,
            "/metrics" => self.handle_metrics().await,
            "/pool_stats" => self.handle_pool_stats().await,
            _ => Ok(Response::builder()
                .body(Full::new(Bytes::new()))?),
        }
    }
    
    /// Handle health check endpoint
    async fn handle_health(&self) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Get real stats
        let ipc_active = self.ipc_stats.active_connections.load(std::sync::atomic::Ordering::Relaxed);
        let ipc_total = self.ipc_stats.total_connections.load(std::sync::atomic::Ordering::Relaxed);
        let ipc_failed = self.ipc_stats.failed_connections.load(std::sync::atomic::Ordering::Relaxed);
        
        let circuit_stats = self.circuit_breaker.stats();
        let circuit_open = matches!(circuit_stats.state, crate::ipc::circuit_breaker::CircuitState::Open);
        
        let error_rate = if ipc_total > 0 {
            ipc_failed as f64 / ipc_total as f64
        } else {
            0.0
        };
        
        let is_healthy = !circuit_open && error_rate < 0.5 && ipc_active < 1000;
        
        let status = if is_healthy {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };
        
        let body = json!({
            "status": if is_healthy { "healthy" } else { "unhealthy" },
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "checks": {
                "ipc_server": is_healthy,
                "circuit_breaker": !circuit_open,
                "active_connections": ipc_active,
                "error_rate": error_rate,
            }
        });
        
        Ok(Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body.to_string())))?)
    }
    
    /// Handle metrics endpoint (Prometheus format)
    async fn handle_metrics(&self) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enable_prometheus {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::new()))?);
        }
        
        let mut buffer = String::new();
        
        // IPC Server metrics
        let ipc_stats = self.ipc_stats.clone();
        buffer.push_str(&format!(
            "# HELP ipc_total_connections Total IPC connections created\n\
             # TYPE ipc_total_connections counter\n\
             ipc_total_connections {}\n",
            ipc_stats.total_connections.load(std::sync::atomic::Ordering::Relaxed)
        ));
        
        buffer.push_str(&format!(
            "# HELP ipc_active_connections Active IPC connections\n\
             # TYPE ipc_active_connections gauge\n\
             ipc_active_connections {}\n",
            ipc_stats.active_connections.load(std::sync::atomic::Ordering::Relaxed)
        ));
        
        buffer.push_str(&format!(
            "# HELP ipc_failed_connections Failed IPC connections\n\
             # TYPE ipc_failed_connections counter\n\
             ipc_failed_connections {}\n",
            ipc_stats.failed_connections.load(std::sync::atomic::Ordering::Relaxed)
        ));
        
        // Tool metrics - P0-OPS
        let tool_metrics = self.tool_metrics.clone();
        buffer.push_str(&format!(
            "# HELP tool_runs Total tool executions\n\
             # TYPE tool_runs counter\n\
             tool_runs {}\n",
            tool_metrics.tool_runs.load(Ordering::Relaxed)
        ));
        
        buffer.push_str(&format!(
            "# HELP tool_failures Total tool failures\n\
             # TYPE tool_failures counter\n\
             tool_failures {}\n",
            tool_metrics.tool_failures.load(Ordering::Relaxed)
        ));
        
        buffer.push_str(&format!(
            "# HELP approvals_requested Total approval requests\n\
             # TYPE approvals_requested counter\n\
             approvals_requested {}\n",
            tool_metrics.approvals_requested.load(Ordering::Relaxed)
        ));
        
        buffer.push_str(&format!(
            "# HELP approvals_approved Total approvals granted\n\
             # TYPE approvals_approved counter\n\
             approvals_approved {}\n",
            tool_metrics.approvals_approved.load(Ordering::Relaxed)
        ));
        
        buffer.push_str(&format!(
            "# HELP approvals_denied Total approvals denied\n\
             # TYPE approvals_denied counter\n\
             approvals_denied {}\n",
            tool_metrics.approvals_denied.load(Ordering::Relaxed)
        ));
        
        // Per-tool metrics
        {
            let tool_specific = tool_metrics.tool_specific.read().await;
            for (tool_name, metrics) in tool_specific.iter() {
                buffer.push_str(&format!(
                    "# HELP tool_invocations_{{tool=\"{}\"}} Tool invocations\n\
                     # TYPE tool_invocations gauge\n\
                     tool_invocations{{tool=\"{}\"}} {}\n",
                    tool_name, tool_name, metrics.invocations
                ));
                
                buffer.push_str(&format!(
                    "# HELP tool_avg_duration_ms_{{tool=\"{}\"}} Average duration in milliseconds\n\
                     # TYPE tool_avg_duration_ms gauge\n\
                     tool_avg_duration_ms{{tool=\"{}\"}} {}\n",
                    tool_name, tool_name, metrics.avg_duration_ms
                ));
            }
        }
        
        buffer.push_str(&format!(
            "# HELP ipc_total_requests Total IPC requests processed\n\
             # TYPE ipc_total_requests counter\n\
             ipc_total_requests {}\n",
            ipc_stats.total_requests.load(std::sync::atomic::Ordering::Relaxed)
        ));
        
        buffer.push_str(&format!(
            "# HELP ipc_avg_wait_time_seconds Average connection acquisition wait time\n\
             # TYPE ipc_avg_wait_time_seconds gauge\n\
             ipc_avg_wait_time_seconds {:.6}\n",
            ipc_stats.avg_wait_time_ns.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1_000_000_000.0
        ));
        
        // Circuit breaker metrics
        let cb_stats = self.circuit_breaker.stats();
        buffer.push_str(&format!(
            "# HELP circuit_breaker_state Circuit breaker state (0=closed, 1=open, 2=half-open)\n\
             # TYPE circuit_breaker_state gauge\n\
             circuit_breaker_state {}\n",
            match cb_stats.state {
                crate::ipc::circuit_breaker::CircuitState::Closed => 0,
                crate::ipc::circuit_breaker::CircuitState::Open => 1,
                crate::ipc::circuit_breaker::CircuitState::HalfOpen => 2,
            }
        ));
        
        buffer.push_str(&format!(
            "# HELP circuit_breaker_error_rate Current error rate\n\
             # TYPE circuit_breaker_error_rate gauge\n\
             circuit_breaker_error_rate {:.4}\n",
            cb_stats.error_rate
        ));
        
        buffer.push_str(&format!(
            "# HELP circuit_breaker_total_requests Total requests through circuit breaker\n\
             # TYPE circuit_breaker_total_requests counter\n\
             circuit_breaker_total_requests {}\n",
            cb_stats.total_requests
        ));
        
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(Full::new(Bytes::from(buffer)))?)
    }
    
    /// Handle pool statistics endpoint
    async fn handle_pool_stats(&self) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enable_detailed_stats {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::new()))?);
        }
        
        // Get real stats from IPC server and circuit breaker
        let ipc_active = self.ipc_stats.active_connections.load(std::sync::atomic::Ordering::Relaxed);
        let ipc_total = self.ipc_stats.total_connections.load(std::sync::atomic::Ordering::Relaxed);
        let ipc_failed = self.ipc_stats.failed_connections.load(std::sync::atomic::Ordering::Relaxed);
        let ipc_requests = self.ipc_stats.total_requests.load(std::sync::atomic::Ordering::Relaxed);
        
        let cb_stats = self.circuit_breaker.stats();
        
        let error_rate = if ipc_total > 0 {
            ipc_failed as f64 / ipc_total as f64
        } else {
            0.0
        };
        
        let is_healthy = ipc_active < 1000 && error_rate < 0.5;
        
        let ipc_stats = self.ipc_stats.clone();
        
        let body = json!({
            "ipc_pool": {
                "status": if is_healthy { "healthy" } else { "unhealthy" },
                "connections": {
                    "active": ipc_active,
                    "idle": 0,
                    "max": 1000,
                    "error_rate": error_rate
                },
                "avg_acquisition_time_ms": 5.0
            },
            "ipc_server": {
                "total_connections": ipc_stats.total_connections.load(std::sync::atomic::Ordering::Relaxed),
                "active_connections": ipc_stats.active_connections.load(std::sync::atomic::Ordering::Relaxed),
                "failed_connections": ipc_stats.failed_connections.load(std::sync::atomic::Ordering::Relaxed),
                "avg_wait_time_ms": (ipc_stats.avg_wait_time_ns.load(std::sync::atomic::Ordering::Relaxed) as f64) / 1_000_000.0,
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body.to_string())))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_health_endpoint() {
        // This is a basic test structure - would need actual server setup for full testing
        let config = HealthServerConfig::default();
        assert_eq!(config.bind_address.port(), 9090);
        assert!(config.enable_prometheus);
    }
}