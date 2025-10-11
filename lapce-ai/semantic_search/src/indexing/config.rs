// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Configuration for incremental indexing (CST-B06)
//!
//! Provides unified config with safe defaults for:
//! - Cache sizing and eviction
//! - Async indexer concurrency
//! - Persistence and segmentation
//! - Performance tuning

use std::path::PathBuf;
use std::time::Duration;
use serde::{Serialize, Deserialize};

/// Master configuration for incremental indexing system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalIndexingConfig {
    /// Cache configuration
    pub cache: CacheConfig,
    
    /// Async indexer configuration
    pub async_indexer: AsyncIndexerConfig,
    
    /// Persistence configuration
    pub persistence: PersistenceConfig,
    
    /// Performance tuning
    pub performance: PerformanceConfig,
    
    /// Enable incremental indexing
    pub enabled: bool,
}

impl Default for IncrementalIndexingConfig {
    fn default() -> Self {
        Self {
            cache: CacheConfig::default(),
            async_indexer: AsyncIndexerConfig::default(),
            persistence: PersistenceConfig::default(),
            performance: PerformanceConfig::default(),
            enabled: true,
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum cache size in MB
    pub max_size_mb: usize,
    
    /// Enable LRU eviction
    pub enable_lru: bool,
    
    /// Eviction threshold (% of max size)
    pub eviction_threshold: f64,
    
    /// Eviction batch size (% to evict)
    pub eviction_batch_pct: f64,
    
    /// Cache TTL in seconds (0 = no expiration)
    pub ttl_seconds: u64,
    
    /// Enable compression
    pub enable_compression: bool,
    
    /// Compression level (1-9)
    pub compression_level: u32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size_mb: 100,           // 100 MB default
            enable_lru: true,
            eviction_threshold: 0.9,     // Evict at 90% full
            eviction_batch_pct: 0.2,     // Evict 20% at a time
            ttl_seconds: 0,              // No expiration
            enable_compression: false,   // Off by default for speed
            compression_level: 3,        // Balanced compression
        }
    }
}

impl CacheConfig {
    /// Production config: larger cache, compression enabled
    pub fn production() -> Self {
        Self {
            max_size_mb: 500,
            enable_compression: true,
            compression_level: 6,
            ..Default::default()
        }
    }
    
    /// Development config: smaller cache, no compression
    pub fn development() -> Self {
        Self {
            max_size_mb: 50,
            enable_compression: false,
            ..Default::default()
        }
    }
    
    /// Memory-constrained config
    pub fn low_memory() -> Self {
        Self {
            max_size_mb: 25,
            enable_compression: true,
            compression_level: 9,
            eviction_threshold: 0.8,
            eviction_batch_pct: 0.3,
            ..Default::default()
        }
    }
}

/// Async indexer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncIndexerConfig {
    /// Maximum concurrent indexing tasks
    pub max_concurrent_tasks: usize,
    
    /// Timeout for single file (seconds)
    pub file_timeout_secs: u64,
    
    /// Timeout for embedding generation (seconds)
    pub embedding_timeout_secs: u64,
    
    /// Queue capacity (back-pressure threshold)
    pub queue_capacity: usize,
    
    /// Enable task prioritization
    pub enable_prioritization: bool,
    
    /// Graceful shutdown timeout (seconds)
    pub shutdown_timeout_secs: u64,
}

impl Default for AsyncIndexerConfig {
    fn default() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            max_concurrent_tasks: cpus.max(2),
            file_timeout_secs: 30,
            embedding_timeout_secs: 10,
            queue_capacity: 1000,
            enable_prioritization: true,
            shutdown_timeout_secs: 30,
        }
    }
}

impl AsyncIndexerConfig {
    /// High-throughput config
    pub fn high_throughput() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            max_concurrent_tasks: cpus * 2,
            queue_capacity: 5000,
            ..Default::default()
        }
    }
    
    /// Low-latency config
    pub fn low_latency() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            max_concurrent_tasks: cpus,
            queue_capacity: 100,
            file_timeout_secs: 10,
            ..Default::default()
        }
    }
}

/// Persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Enable persistent cache
    pub enabled: bool,
    
    /// Cache directory
    pub cache_dir: PathBuf,
    
    /// Enable segmented storage
    pub enable_segmentation: bool,
    
    /// Segment size in KB
    pub segment_size_kb: usize,
    
    /// Enable write-ahead logging
    pub enable_wal: bool,
    
    /// Sync to disk interval (seconds)
    pub sync_interval_secs: u64,
    
    /// Enable crash recovery
    pub enable_recovery: bool,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_dir: std::env::temp_dir().join("semantic_search_cache"),
            enable_segmentation: true,
            segment_size_kb: 256,
            enable_wal: false,              // WAL off by default
            sync_interval_secs: 300,        // Sync every 5 minutes
            enable_recovery: true,
        }
    }
}

impl PersistenceConfig {
    /// In-memory only (no persistence)
    pub fn memory_only() -> Self {
        Self {
            enabled: false,
            enable_wal: false,
            ..Default::default()
        }
    }
    
    /// High-durability config
    pub fn high_durability() -> Self {
        Self {
            enabled: true,
            enable_wal: true,
            sync_interval_secs: 60,         // Sync every minute
            enable_recovery: true,
            ..Default::default()
        }
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable parallel node processing
    pub enable_parallel_nodes: bool,
    
    /// Minimum nodes for parallel processing
    pub parallel_threshold: usize,
    
    /// Enable batch embedding
    pub enable_batch_embedding: bool,
    
    /// Batch size for embeddings
    pub embedding_batch_size: usize,
    
    /// Enable prefetching
    pub enable_prefetch: bool,
    
    /// Prefetch buffer size
    pub prefetch_buffer_size: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_parallel_nodes: true,
            parallel_threshold: 50,          // Parallelize if >50 nodes
            enable_batch_embedding: true,
            embedding_batch_size: 32,
            enable_prefetch: false,          // Conservative default
            prefetch_buffer_size: 10,
        }
    }
}

impl PerformanceConfig {
    /// Maximum performance
    pub fn max_performance() -> Self {
        Self {
            enable_parallel_nodes: true,
            parallel_threshold: 20,
            enable_batch_embedding: true,
            embedding_batch_size: 64,
            enable_prefetch: true,
            prefetch_buffer_size: 50,
        }
    }
    
    /// Conservative config (lower CPU usage)
    pub fn conservative() -> Self {
        Self {
            enable_parallel_nodes: false,
            enable_batch_embedding: true,
            embedding_batch_size: 16,
            enable_prefetch: false,
            ..Default::default()
        }
    }
}

impl IncrementalIndexingConfig {
    /// Load config from TOML file (requires toml crate)
    #[cfg(feature = "config_toml")]
    pub fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }
    
    /// Save config to TOML file (requires toml crate)
    #[cfg(feature = "config_toml")]
    pub fn to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Cache validation
        if self.cache.max_size_mb == 0 {
            return Err("Cache max_size_mb must be > 0".to_string());
        }
        if self.cache.eviction_threshold < 0.5 || self.cache.eviction_threshold > 1.0 {
            return Err("Cache eviction_threshold must be between 0.5 and 1.0".to_string());
        }
        if self.cache.compression_level < 1 || self.cache.compression_level > 9 {
            return Err("Cache compression_level must be between 1 and 9".to_string());
        }
        
        // Async indexer validation
        if self.async_indexer.max_concurrent_tasks == 0 {
            return Err("max_concurrent_tasks must be > 0".to_string());
        }
        if self.async_indexer.queue_capacity == 0 {
            return Err("queue_capacity must be > 0".to_string());
        }
        
        // Persistence validation
        if self.persistence.segment_size_kb == 0 {
            return Err("segment_size_kb must be > 0".to_string());
        }
        
        // Performance validation
        if self.performance.embedding_batch_size == 0 {
            return Err("embedding_batch_size must be > 0".to_string());
        }
        
        Ok(())
    }
    
    /// Production preset
    pub fn production() -> Self {
        Self {
            cache: CacheConfig::production(),
            async_indexer: AsyncIndexerConfig::high_throughput(),
            persistence: PersistenceConfig::high_durability(),
            performance: PerformanceConfig::max_performance(),
            enabled: true,
        }
    }
    
    /// Development preset
    pub fn development() -> Self {
        Self {
            cache: CacheConfig::development(),
            async_indexer: AsyncIndexerConfig::default(),
            persistence: PersistenceConfig::memory_only(),
            performance: PerformanceConfig::default(),
            enabled: true,
        }
    }
    
    /// Low-resource preset
    pub fn low_resource() -> Self {
        Self {
            cache: CacheConfig::low_memory(),
            async_indexer: AsyncIndexerConfig::default(),
            persistence: PersistenceConfig::memory_only(),
            performance: PerformanceConfig::conservative(),
            enabled: true,
        }
    }
    
    /// Get duration values
    pub fn file_timeout(&self) -> Duration {
        Duration::from_secs(self.async_indexer.file_timeout_secs)
    }
    
    pub fn embedding_timeout(&self) -> Duration {
        Duration::from_secs(self.async_indexer.embedding_timeout_secs)
    }
    
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.async_indexer.shutdown_timeout_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_default_config_valid() {
        let config = IncrementalIndexingConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_production_config_valid() {
        let config = IncrementalIndexingConfig::production();
        assert!(config.validate().is_ok());
        assert!(config.cache.max_size_mb > 100);
        assert!(config.cache.enable_compression);
    }
    
    #[test]
    fn test_development_config_valid() {
        let config = IncrementalIndexingConfig::development();
        assert!(config.validate().is_ok());
        assert!(!config.persistence.enabled);
        assert!(!config.cache.enable_compression);
    }
    
    #[test]
    fn test_low_resource_config() {
        let config = IncrementalIndexingConfig::low_resource();
        assert!(config.validate().is_ok());
        assert!(config.cache.max_size_mb < 50);
        assert!(!config.performance.enable_parallel_nodes);
    }
    
    #[test]
    fn test_config_validation_errors() {
        let mut config = IncrementalIndexingConfig::default();
        
        // Invalid cache size
        config.cache.max_size_mb = 0;
        assert!(config.validate().is_err());
        config.cache.max_size_mb = 100;
        
        // Invalid eviction threshold
        config.cache.eviction_threshold = 1.5;
        assert!(config.validate().is_err());
        config.cache.eviction_threshold = 0.9;
        
        // Invalid compression level
        config.cache.compression_level = 10;
        assert!(config.validate().is_err());
        config.cache.compression_level = 3;
        
        // Now should be valid
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_cloning() {
        let config = IncrementalIndexingConfig::default();
        let cloned = config.clone();
        assert_eq!(config.cache.max_size_mb, cloned.cache.max_size_mb);
        assert_eq!(config.enabled, cloned.enabled);
    }
    
    #[test]
    fn test_duration_conversion() {
        let config = IncrementalIndexingConfig::default();
        assert_eq!(config.file_timeout(), Duration::from_secs(30));
        assert_eq!(config.embedding_timeout(), Duration::from_secs(10));
    }
}
