// Day 10.3: Complete REST API Server with Axum
use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

// API Models
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
    id: String,
    content: String,
    embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchRequest {
    query: String,
    top_k: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResult {
    id: String,
    score: f32,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheRequest {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

// App State
#[derive(Clone)]
struct AppState {
    documents: Arc<RwLock<HashMap<String, Document>>>,
    cache: Arc<RwLock<HashMap<String, String>>>,
}

#[tokio::main]
async fn main() {
    println!("🚀 Starting REST API Server on port 8080");
    
    let state = AppState {
        documents: Arc::new(RwLock::new(HashMap::new())),
        cache: Arc::new(RwLock::new(HashMap::new())),
    };
    
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        
        // Document endpoints
        .route("/documents", post(create_document))
        .route("/documents/:id", get(get_document))
        .route("/documents/:id", delete(delete_document))
        .route("/documents", get(list_documents))
        
        // Search endpoint
        .route("/search", post(search_documents))
        
        // Cache endpoints
        .route("/cache", post(set_cache))
        .route("/cache/:key", get(get_cache))
        .route("/cache/:key", delete(delete_cache))
        
        // Performance test endpoint
        .route("/test/performance", get(performance_test))
        
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    
    println!("✅ REST API listening on http://0.0.0.0:8080");
    
    axum::serve(listener, app).await.unwrap();
}

// Health check
async fn health_check() -> impl IntoResponse {
    Json(ApiResponse {
        success: true,
        data: Some("REST API is healthy"),
        error: None,
    })
}

// Document operations
async fn create_document(
    State(state): State<AppState>,
    Json(doc): Json<Document>,
) -> impl IntoResponse {
    let mut docs = state.documents.write().await;
    docs.insert(doc.id.clone(), doc.clone());
    
    Json(ApiResponse {
        success: true,
        data: Some(doc),
        error: None,
    })
}

async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let docs = state.documents.read().await;
    
    match docs.get(&id) {
        Some(doc) => Json(ApiResponse {
            success: true,
            data: Some(doc.clone()),
            error: None,
        }),
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Document not found".to_string()),
        }),
    }
}

async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut docs = state.documents.write().await;
    
    match docs.remove(&id) {
        Some(_) => Json(ApiResponse {
            success: true,
            data: Some("Document deleted"),
            error: None,
        }),
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Document not found".to_string()),
        }),
    }
}

async fn list_documents(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let docs = state.documents.read().await;
    let doc_list: Vec<Document> = docs.values().cloned().collect();
    
    Json(ApiResponse {
        success: true,
        data: Some(doc_list),
        error: None,
    })
}

// Search
async fn search_documents(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> impl IntoResponse {
    let docs = state.documents.read().await;
    
    // Simple text search (mock)
    let mut results = Vec::new();
    for doc in docs.values() {
        if doc.content.contains(&req.query) {
            results.push(SearchResult {
                id: doc.id.clone(),
                score: 0.95, // Mock score
                content: doc.content.clone(),
            });
        }
    }
    
    results.truncate(req.top_k);
    
    Json(ApiResponse {
        success: true,
        data: Some(results),
        error: None,
    })
}

// Cache operations
async fn set_cache(
    State(state): State<AppState>,
    Json(req): Json<CacheRequest>,
) -> impl IntoResponse {
    let mut cache = state.cache.write().await;
    cache.insert(req.key.clone(), req.value);
    
    Json(ApiResponse {
        success: true,
        data: Some("Cache set"),
        error: None,
    })
}

async fn get_cache(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let cache = state.cache.read().await;
    
    match cache.get(&key) {
        Some(value) => Json(ApiResponse {
            success: true,
            data: Some(value.clone()),
            error: None,
        }),
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Key not found".to_string()),
        }),
    }
}

async fn delete_cache(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let mut cache = state.cache.write().await;
    
    match cache.remove(&key) {
        Some(_) => Json(ApiResponse {
            success: true,
            data: Some("Cache deleted"),
            error: None,
        }),
        None => Json(ApiResponse {
            success: false,
            data: None,
            error: Some("Key not found".to_string()),
        }),
    }
}

// Performance test
async fn performance_test() -> impl IntoResponse {
    use std::time::Instant;
    
    let start = Instant::now();
    let iterations = 10000;
    
    for _ in 0..iterations {
        // Simulate work
        std::hint::black_box(42);
    }
    
    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
    
    Json(ApiResponse {
        success: true,
        data: Some(format!("{:.0} ops/sec", ops_per_sec)),
        error: None,
    })
}
