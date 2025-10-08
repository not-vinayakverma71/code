//! Health and metrics server for monitoring CST operations

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use prometheus::{
    register_counter_vec, register_histogram_vec, register_gauge_vec,
    CounterVec, HistogramVec, GaugeVec, TextEncoder, Encoder,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Metrics collected by the server
pub struct Metrics {
    pub cache_hits: CounterVec,
    pub cache_misses: CounterVec,
    pub parse_duration: HistogramVec,
    pub bytecode_size: HistogramVec,
    pub memory_usage: GaugeVec,
    pub active_parsers: GaugeVec,
    pub segment_loads: CounterVec,
    pub bytes_written: CounterVec,
    pub verify_duration: HistogramVec,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            cache_hits: register_counter_vec!(
                "cst_cache_hits_total",
                "Total number of cache hits",
                &["language", "tier"]
            ).unwrap(),
            
            cache_misses: register_counter_vec!(
                "cst_cache_misses_total",
                "Total number of cache misses",
                &["language", "tier"]
            ).unwrap(),
            
            parse_duration: register_histogram_vec!(
                "cst_parse_duration_seconds",
                "Parse duration in seconds",
                &["language"],
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
            ).unwrap(),
            
            bytecode_size: register_histogram_vec!(
                "cst_bytecode_size_bytes",
                "Bytecode size in bytes",
                &["language"],
                vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0]
            ).unwrap(),
            
            memory_usage: register_gauge_vec!(
                "cst_memory_usage_bytes",
                "Memory usage in bytes",
                &["component"]
            ).unwrap(),
            
            active_parsers: register_gauge_vec!(
                "cst_active_parsers",
                "Number of active parsers",
                &["language"]
            ).unwrap(),
            
            segment_loads: register_counter_vec!(
                "cst_segment_loads_total",
                "Total number of segment loads",
                &["tier"]
            ).unwrap(),
            
            bytes_written: register_counter_vec!(
                "cst_bytes_written_total",
                "Total bytes written",
                &["tier"]
            ).unwrap(),
            
            verify_duration: register_histogram_vec!(
                "cst_verify_duration_seconds",
                "Verification duration in seconds",
                &["language"],
                vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
            ).unwrap(),
        }
    }
}

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Cache statistics
#[derive(Serialize, Deserialize, Clone)]
pub struct CacheStats {
    pub total_hits: u64,
    pub total_misses: u64,
    pub hit_ratio: f64,
    pub total_bytes: u64,
    pub total_entries: u64,
    pub evictions: u64,
}

/// Application state
pub struct AppState {
    pub metrics: Arc<Metrics>,
    pub cache_stats: Arc<RwLock<CacheStats>>,
    pub start_time: SystemTime,
}

/// Create the metrics server router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/stats", get(stats_handler))
        .route("/stats", post(update_stats_handler))
        .with_state(Arc::new(state))
}

/// Health check endpoint
async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().unwrap_or_default().as_secs();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
    })
}

/// Prometheus metrics endpoint
async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    String::from_utf8(buffer).unwrap()
}

/// Get cache statistics
async fn stats_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let stats = state.cache_stats.read().await;
    Json(stats.clone())
}

/// Update cache statistics
async fn update_stats_handler(
    State(state): State<Arc<AppState>>,
    Json(new_stats): Json<CacheStats>,
) -> impl IntoResponse {
    let mut stats = state.cache_stats.write().await;
    *stats = new_stats;
    StatusCode::OK
}

/// Start the metrics server
pub async fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        metrics: Arc::new(Metrics::new()),
        cache_stats: Arc::new(RwLock::new(CacheStats {
            total_hits: 0,
            total_misses: 0,
            hit_ratio: 0.0,
            total_bytes: 0,
            total_entries: 0,
            evictions: 0,
        })),
        start_time: SystemTime::now(),
    };
    
    let app = create_router(state);
    let addr = format!("0.0.0.0:{}", port);
    
    println!("Metrics server listening on http://{}", addr);
    
    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new();
        
        // Record some metrics
        metrics.cache_hits.with_label_values(&["rust", "hot"]).inc();
        metrics.cache_misses.with_label_values(&["python", "cold"]).inc();
        metrics.parse_duration.with_label_values(&["rust"]).observe(0.025);
        metrics.bytecode_size.with_label_values(&["python"]).observe(5000.0);
        
        // Verify they were recorded
        let families = prometheus::gather();
        assert!(!families.is_empty());
    }
}
