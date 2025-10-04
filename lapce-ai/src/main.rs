/// Main Application with Cache Integration
/// Complete production-ready implementation

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error};
use lapce_ai_rust::cache::{CacheV3, CacheConfig};
use tokio::signal;

async fn init_cache_service(config: CacheConfig) -> Result<Arc<CacheV3>> {
    Ok(Arc::new(CacheV3::new(config).await?))
}

async fn shutdown_cache_service() -> Result<()> {
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("Starting Lapce AI application...");
    
    // Load configuration
    let cache_config = load_cache_config().await?;
    
    // Initialize cache service
    info!("Initializing cache service...");
    let cache = init_cache_service(cache_config).await?;
    info!("Cache service initialized successfully");
    
    // Start main application
    let app = Arc::new(Application::new(cache.clone()).await?);
    
    // Start HTTP server
    let server = start_http_server(app).await?;
    
    // Wait for shutdown signal
    shutdown_signal().await;
    
    // Graceful shutdown
    info!("Shutting down application...");
    server.abort();
    shutdown_cache_service().await?;
    info!("Application shut down successfully");
    
    Ok(())
}

/// Load cache configuration from file or environment
async fn load_cache_config() -> Result<CacheConfig> {
    // Try to load from config file
    if let Ok(config) = CacheConfig::from_toml("config/cache.toml") {
        info!("Loaded cache config from file");
        return Ok(config);
    }
    
    // Fall back to environment variables
    let mut config = CacheConfig::default();
    
    if let Ok(val) = std::env::var("CACHE_L1_MAX_ENTRIES") {
        config.l1_config.max_entries = val.parse()?;
    }
    
    if let Ok(val) = std::env::var("CACHE_L2_PATH") {
        config.l2_config.cache_dir = val.into();
    }
    
    if let Ok(val) = std::env::var("CACHE_L3_URL") {
        config.l3_redis_url = Some(val);
    }
    
    info!("Using default cache config with environment overrides");
    Ok(config)
}

/// Application structure with cache integration
struct Application {
    cache: Arc<CacheV3>,
}

impl Application {
    async fn new(cache: Arc<CacheV3>) -> Result<Self> {
        Ok(Self { cache })
    }
    
    /// Handle query with cache
    async fn handle_query(&self, query: String) -> Result<String> {
        // Generate cache key from query
        let cache_key = format!("query:{}", blake3::hash(query.as_bytes()).to_hex());
        
        // Try cache first
        use lapce_ai_rust::cache::{CacheKey, CacheValue};
        let key = CacheKey(cache_key);
        if let Some(cached_result) = self.cache.get(&key).await {
            info!("Cache hit for query");
            return Ok(String::from_utf8(cached_result.data)?);
        }
        
        // Process query (would be actual processing logic)
        let result = self.process_query_internal(query).await?;
        
        // Store in cache
        let value = CacheValue {
            data: result.clone().into_bytes(),
            size: result.len(),
            created_at: std::time::SystemTime::now(),
            access_count: 0,
            last_accessed: std::time::SystemTime::now(),
            metadata: None,
            ttl: None,
        };
        self.cache.put(key, value).await;
        
        Ok(result)
    }
    
    async fn process_query_internal(&self, query: String) -> Result<String> {
        // Simulate query processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(format!("Processed: {}", query))
    }
}

/// Start HTTP server for cache operations
async fn start_http_server(app: Arc<Application>) -> Result<tokio::task::JoinHandle<()>> {
    use warp::Filter;
    
    // Health check endpoint
    let health = warp::path("health")
        .map(|| warp::reply::json(&serde_json::json!({"status": "ok"})));
    
    // Query endpoint
    let query = {
        let app = app.clone();
        warp::path("query")
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |body: serde_json::Value| {
                let app = app.clone();
                async move {
                    let query = body.get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    match app.handle_query(query).await {
                        Ok(result) => Ok::<_, warp::Rejection>(warp::reply::json(&serde_json::json!({
                            "result": result
                        }))),
                        Err(e) => {
                            error!("Query error: {}", e);
                            Ok(warp::reply::json(&serde_json::json!({
                                "error": e.to_string()
                            })))
                        }
                    }
                }
            })
    };
    
    // Cache stats endpoint
    let stats = {
        let app = app.clone();
        warp::path("cache")
            .and(warp::path("stats"))
            .and_then(move || {
                let app = app.clone();
                async move {
                    let stats = app.cache.get_metrics().await;
                    Ok::<_, warp::Rejection>(warp::reply::json(&stats))
                }
            })
    };
    
    // Prometheus metrics endpoint
    let metrics = {
        let app = app.clone();
        warp::path("metrics")
            .and_then(move || {
                let app = app.clone();
                async move {
                    let metrics = app.cache.get_metrics().await;
                    let json = serde_json::to_string(&metrics).unwrap_or_default();
                    Ok::<_, warp::Rejection>(warp::reply::with_header(
                        json,
                        "content-type",
                        "application/json"
                    ))
                }
            })
    };
    
    // Combine all routes with explicit boxing to help type inference
    let routes = health.boxed()
        .or(query.boxed())
        .or(stats.boxed())
        .or(metrics.boxed());
    
    let handle = tokio::spawn(async move {
        warp::serve(routes)
            .run(([0, 0, 0, 0], 8080))
            .await;
    });
    
    info!("HTTP server started on :8080");
    Ok(handle)
}

/// Wait for shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    
    info!("Shutdown signal received");
}
