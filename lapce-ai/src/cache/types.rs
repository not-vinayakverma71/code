/// Core types for cache system - EXACT from docs lines 34-49

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use parking_lot::RwLock;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct CacheKey(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheValue {
    pub data: Vec<u8>,
    pub size: usize,
    pub created_at: SystemTime,
    pub access_count: u32,
    pub last_accessed: SystemTime,
    pub metadata: Option<HashMap<String, String>>,
    pub ttl: Option<u64>,
}

impl From<QueryResult> for CacheValue {
    fn from(result: QueryResult) -> Self {
        CacheValue::new(result.data)
    }
}

impl CacheValue {
    pub fn size(&self) -> usize {
        self.size
    }
    
    pub fn new(data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            data,
            size,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: None,
            ttl: None,
        }
    }
    
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheLevel {
    L1,
    L2,
    L3,
}

#[derive(Debug, Clone)]
pub struct L1Config {
    pub max_entries: u64,
    pub ttl: std::time::Duration,
    pub idle_time: std::time::Duration,
    pub bloom_size: usize,
    pub bloom_fp_rate: f64,
}

impl Default for L1Config {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            ttl: std::time::Duration::from_secs(3600),
            idle_time: std::time::Duration::from_secs(600),
            bloom_size: 100000,
            bloom_fp_rate: 0.01,
        }
    }
}

#[derive(Debug, Clone)]
pub struct L2Config {
    pub max_size: usize,
    pub compression: CompressionType,
    pub cache_dir: PathBuf,
}

impl Default for L2Config {
    fn default() -> Self {
        Self {
            max_size: 100 * 1024 * 1024, // 100MB
            compression: CompressionType::Lz4,
            cache_dir: PathBuf::from("/tmp/cache"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub l1_config: L1Config,
    pub l2_config: L2Config,
    pub l3_redis_url: Option<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_config: L1Config::default(),
            l2_config: L2Config::default(),
            l3_redis_url: None,
        }
    }
}

impl CacheConfig {
    pub fn from_toml(_path: &str) -> anyhow::Result<Self> {
        Ok(Self::default())
    }
    
    // Compatibility accessors for old field names
    pub fn l1_max_entries(&self) -> u64 {
        self.l1_config.max_entries
    }
    
    pub fn l2_path(&self) -> &PathBuf {
        &self.l2_config.cache_dir
    }
    
    pub fn l3_enabled(&self) -> bool {
        self.l3_redis_url.is_some()
    }
    
    pub fn l3_url(&self) -> Option<&str> {
        self.l3_redis_url.as_deref()
    }
    
    pub fn metrics_enabled(&self) -> bool {
        true
    }
}

impl TryFrom<CacheValue> for QueryResult {
    type Error = ();
    
    fn try_from(value: CacheValue) -> Result<Self, Self::Error> {
        Ok(QueryResult { data: value.data })
    }
}
