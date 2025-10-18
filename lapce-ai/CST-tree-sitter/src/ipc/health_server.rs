//! Health and metrics server for CST-tree-sitter
//! Provides /healthz, /readyz, and /metrics endpoints

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use prometheus::{Encoder, TextEncoder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    timestamp: String,
    version: String,
    uptime_seconds: u64,
}

/// Readiness check response
#[derive(Debug, Serialize, Deserialize)]
struct ReadinessResponse {
    ready: bool,
    checks: Vec<ReadinessCheck>,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReadinessCheck {
    name: String,
    status: String,
    message: Option<String>,
}

/// Application state for health checks
#[derive(Clone)]
pub struct AppState {
    pub start_time: std::time::Instant,
    pub cache: Arc<crate::Phase4Cache>,
}

/// Health check handler - /healthz
async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: format!("{:?}", std::time::SystemTime::now()),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
    };
    
    (StatusCode::OK, axum::Json(response))
}

/// Readiness check handler - /readyz
async fn readiness_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut checks = vec![];
    let mut all_ready = true;
    
    // Check cache readiness
    let stats = state.cache.stats();
    let total_entries = stats.hot_entries + stats.warm_entries + stats.cold_entries + stats.frozen_entries;
    
    let cache_check = if total_entries > 0 {
        ReadinessCheck {
            name: "cache".to_string(),
            status: "ready".to_string(),
            message: Some(format!("{} entries cached (hot:{}, warm:{}, cold:{}, frozen:{})", 
                total_entries, stats.hot_entries, stats.warm_entries, stats.cold_entries, stats.frozen_entries)),
        }
    } else {
        ReadinessCheck {
            name: "cache".to_string(),
            status: "ready".to_string(),
            message: Some("Cache initialized".to_string()),
        }
    };
    checks.push(cache_check);
    
    // Check storage directory
    if let Some(storage_dir) = state.cache.storage_dir().to_str() {
        let storage_check = if std::path::Path::new(storage_dir).exists() {
            ReadinessCheck {
                name: "storage".to_string(),
                status: "ready".to_string(),
                message: Some(format!("Storage directory: {}", storage_dir)),
            }
        } else {
            all_ready = false;
            ReadinessCheck {
                name: "storage".to_string(),
                status: "not_ready".to_string(),
                message: Some("Storage directory not accessible".to_string()),
            }
        };
        checks.push(storage_check);
    }
    
    let response = ReadinessResponse {
        ready: all_ready,
        checks,
        timestamp: format!("{:?}", std::time::SystemTime::now()),
    };
    
    let status = if all_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    
    (status, axum::Json(response))
}

/// Prometheus metrics handler - /metrics
async fn metrics_handler() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    
    let mut buffer = vec![];
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(()) => {
            let body = String::from_utf8(buffer).unwrap_or_else(|_| "Error encoding metrics".to_string());
            (
                StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4")],
                body,
            ).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error gathering metrics: {}", e),
            ).into_response()
        }
    }
}

/// Root handler
async fn root_handler() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>CST-tree-sitter Health Server</title>
    <style>
        body { font-family: sans-serif; margin: 40px; }
        h1 { color: #333; }
        ul { line-height: 1.8; }
        a { color: #0066cc; text-decoration: none; }
        a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <h1>CST-tree-sitter Health & Metrics Server</h1>
    <ul>
        <li><a href="/healthz">Health Check (/healthz)</a> - Basic health status</li>
        <li><a href="/readyz">Readiness Check (/readyz)</a> - Detailed readiness status</li>
        <li><a href="/metrics">Prometheus Metrics (/metrics)</a> - Cache performance metrics</li>
    </ul>
    <hr>
    <p>Version: 0.1.0</p>
</body>
</html>
    "#)
}

/// Start the health/metrics server
pub async fn start_server(
    addr: &str,
    cache: Arc<crate::Phase4Cache>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        start_time: std::time::Instant::now(),
        cache,
    };
    
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/healthz", get(health_handler))
        .route("/readyz", get(readiness_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state);
    
    let listener = TcpListener::bind(addr).await?;
    println!("Health/metrics server listening on: http://{}", addr);
    
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_health_server_startup() {
        let config = crate::Phase4Config {
            memory_budget_mb: 10,
            hot_tier_ratio: 0.4,
            warm_tier_ratio: 0.3,
            segment_size: 256 * 1024,
            storage_dir: tempdir().unwrap().path().to_path_buf(),
            enable_compression: true,
            test_mode: true,
        };
        
        let cache = Arc::new(crate::Phase4Cache::new(config).unwrap());
        
        // Try to bind to an ephemeral port
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            start_server("127.0.0.1:0", cache),
        ).await;
        
        // Server should start but timeout (since it runs forever)
        assert!(result.is_err());
    }
}
