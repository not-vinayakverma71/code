// Tool Cache module for MCP tools - clean implementation
use std::time::{Duration, Instant, SystemTime};
use std::sync::Arc;
use std::path::PathBuf;
use dashmap::DashMap;
use serde_json::Value;

pub struct ToolCache {
    cache: Arc<DashMap<String, CachedResult>>,
    ttl: Duration,
    max_size: usize,
    metrics: Arc<CacheMetrics>,
}

#[derive(Clone)]
struct CachedResult {
    value: Value,
    timestamp: Instant,
}

struct CacheMetrics {
    // Add metrics fields here
}

impl Default for CacheMetrics {
    fn default() -> Self {
        CacheMetrics {
            // Initialize metrics fields here
        }
    }
}

// FileCache for filesystem operations
pub struct FileCache {
    cache: Arc<DashMap<PathBuf, FileContent>>,
}

#[derive(Clone)]
pub struct FileContent {
    pub content: String,
    pub modified: SystemTime,
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }
    
    pub async fn get(&self, path: &PathBuf) -> Option<FileContent> {
        self.cache.get(path).map(|entry| entry.value().clone())
    }
    
    pub async fn put(&self, path: PathBuf, content: FileContent) {
        self.cache.insert(path, content);
    }
    
    pub async fn invalidate(&self, path: &PathBuf) {
        self.cache.remove(path);
    }
}

impl ToolCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            ttl,
            max_size,
            metrics: Arc::new(CacheMetrics::default()),
        }
    }
    
    pub fn get(&self, key: &str) -> Option<Value> {
        if let Some(entry) = self.cache.get(key) {
            if entry.timestamp.elapsed() < self.ttl {
                return Some(entry.value.clone());
            }
            self.cache.remove(key);
        }
        None
    }
    
    pub fn set(&self, key: String, value: Value) {
        self.cache.insert(key, CachedResult {
            value,
            timestamp: Instant::now(),
        });
    }
    
    pub async fn put(&self, key: String, value: Value) {
        self.set(key, value);
    }
}
