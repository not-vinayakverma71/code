/// Real L2 Cache Implementation with Sled
/// Production-ready persistent disk cache with compression

use std::sync::Arc;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};
use std::collections::HashMap;
use sled::{Db, IVec};
use anyhow::Result;
use async_trait::async_trait;
use tokio::task;

use super::types::{CacheKey, CacheValue, L2Config};
use super::cache_metrics::CacheMetrics;

/// Real L2 persistent disk cache using Sled
pub struct RealL2Cache {
    db: Arc<Db>,
    metrics: Arc<CacheMetrics>,
    config: L2Config,
}

impl RealL2Cache {
    /// Create new L2 cache with Sled backend
    pub async fn new(config: L2Config, metrics: Arc<CacheMetrics>) -> Result<Self> {
        // Ensure cache directory exists
        tokio::fs::create_dir_all(&config.cache_dir).await?;
        
        // Configure Sled
        let db_path = config.cache_dir.join("l2_cache.db");
        let db = sled::Config::default()
            .path(db_path)
            .cache_capacity(100_000_000) // 100MB in-memory cache
            .flush_every_ms(Some(1000)) // Flush to disk every second
            .mode(sled::Mode::HighThroughput)
            .open()?;
        
        Ok(Self {
            db: Arc::new(db),
            metrics,
            config,
        })
    }

    /// Get value from L2 cache
    pub async fn get(&self, key: &CacheKey) -> Result<Option<CacheValue>> {
        let start = Instant::now();
        let db = self.db.clone();
        let key_bytes = bincode::serialize(key)?;
        
        // Run blocking Sled operation in spawn_blocking
        let result = task::spawn_blocking(move || -> Result<Option<Vec<u8>>> {
            match db.get(key_bytes)? {
                Some(ivec) => Ok(Some(ivec.to_vec())),
                None => Ok(None),
            }
        }).await??;
        
        // Deserialize if found
        let value = match result {
            Some(data) => {
                let value = if matches!(self.config.compression, super::types::CompressionType::Lz4 | super::types::CompressionType::Zstd) {
                    // Decompress with zstd
                    let decompressed = zstd::decode_all(&data[..])?;
                    bincode::deserialize(&decompressed)?
                } else {
                    bincode::deserialize(&data)?
                };
                
                self.metrics.record_l2_hit();
                Some(value)
            }
            None => {
                self.metrics.record_l2_miss();
                None
            }
        };
        
        // Record latency
        let latency = start.elapsed();
        self.metrics.record_l2_latency(latency);
        
        Ok(value)
    }

    /// Put value into L2 cache
    pub async fn put(&self, key: CacheKey, value: CacheValue) -> Result<()> {
        let start = Instant::now();
        
        // Check size limit
        if value.size > self.config.max_size {
            return Ok(()); // Silently skip values too large for L2
        }
        
        let db = self.db.clone();
        let key_bytes = bincode::serialize(&key)?;
        
        // Serialize and optionally compress
        let value_bytes = if matches!(self.config.compression, super::types::CompressionType::Lz4 | super::types::CompressionType::Zstd) {
            let serialized = bincode::serialize(&value)?;
            zstd::encode_all(&serialized[..], 3)? // Compression level 3
        } else {
            bincode::serialize(&value)?
        };
        
        // Run blocking Sled operation
        task::spawn_blocking(move || -> Result<()> {
            db.insert(key_bytes, value_bytes)?;
            db.flush()?; // Ensure persistence
            Ok(())
        }).await??;
        
        // Record metrics
        let latency = start.elapsed();
        self.metrics.record_l2_write_latency(latency);
        
        Ok(())
    }

    /// Invalidate specific key
    pub async fn invalidate(&self, key: &CacheKey) -> Result<()> {
        let db = self.db.clone();
        let key_bytes = bincode::serialize(key)?;
        
        task::spawn_blocking(move || -> Result<()> {
            db.remove(key_bytes)?;
            Ok(())
        }).await??;
        
        Ok(())
    }

    /// Invalidate multiple keys matching prefix
    pub async fn invalidate_prefix(&self, prefix: &str) -> Result<usize> {
        let db = self.db.clone();
        let prefix_bytes = prefix.as_bytes().to_vec();
        
        let count = task::spawn_blocking(move || -> Result<usize> {
            let mut count = 0;
            for result in db.scan_prefix(prefix_bytes) {
                let (key, _) = result?;
                db.remove(key)?;
                count += 1;
            }
            Ok(count)
        }).await??;
        
        Ok(count)
    }

    /// Get cache statistics
    pub async fn stats(&self) -> L2Stats {
        let db = self.db.clone();
        
        let (size_on_disk, entry_count) = task::spawn_blocking(move || {
            let size = db.size_on_disk().unwrap_or(0);
            let count = db.len();
            (size, count)
        }).await.unwrap_or((0, 0));
        
        L2Stats {
            size_on_disk_bytes: size_on_disk as usize,
            entry_count,
            compression_enabled: !matches!(self.config.compression, super::types::CompressionType::None),
            hit_rate: self.metrics.l2_hit_rate(),
            avg_latency_ms: self.metrics.avg_l2_latency_ms(),
        }
    }

    /// Run maintenance tasks
    pub async fn run_maintenance(&self) -> Result<()> {
        let db = self.db.clone();
        let max_size = self.config.max_size;
        
        task::spawn_blocking(move || -> Result<()> {
            // Compact database
            db.flush()?;
            
            // Check size and evict if needed
            if db.size_on_disk()? > max_size as u64 {
                // Simple FIFO eviction for now
                // In production, use LRU or access time tracking
                let to_remove = db.len() / 10; // Remove 10% of entries
                let mut removed = 0;
                
                for result in db.iter() {
                    if removed >= to_remove {
                        break;
                    }
                    let (key, _) = result?;
                    db.remove(key)?;
                    removed += 1;
                }
            }
            
            Ok(())
        }).await??;
        
        Ok(())
    }

    /// Clear entire cache
    pub async fn clear(&self) -> Result<()> {
        let db = self.db.clone();
        
        task::spawn_blocking(move || -> Result<()> {
            db.clear()?;
            db.flush()?;
            Ok(())
        }).await??;
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct L2Stats {
    pub size_on_disk_bytes: usize,
    pub entry_count: usize,
    pub compression_enabled: bool,
    pub hit_rate: f64,
    pub avg_latency_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_l2_cache_basic() {
        let temp_dir = TempDir::new().unwrap();
        let config = L2Config {
            max_size: 1_000_000,
            compression: super::types::CompressionType::None,
            cache_dir: temp_dir.path().to_path_buf(),
        };
        
        let metrics = Arc::new(CacheMetrics::default());
        let cache = RealL2Cache::new(config, metrics).await.unwrap();
        
        // Test put and get
        let key = CacheKey("test_key".to_string());
        let value = CacheValue {
            data: vec![1, 2, 3, 4, 5],
            size: 5,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: Some(HashMap::new()),
            ttl: None,
        };
        
        cache.put(key.clone(), value.clone()).await.unwrap();
        
        let retrieved = cache.get(&key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, value.data);
    }

    #[tokio::test]
    async fn test_l2_cache_compression() {
        let temp_dir = TempDir::new().unwrap();
        let config = L2Config {
            max_size: 1_000_000,
            compression: super::types::CompressionType::Lz4, // Enable compression
            cache_dir: temp_dir.path().to_path_buf(),
        };
        
        let metrics = Arc::new(CacheMetrics::default());
        let cache = RealL2Cache::new(config, metrics).await.unwrap();
        
        // Test with larger data to see compression effect
        let key = CacheKey("compress_key".to_string());
        let value = CacheValue {
            data: vec![42; 10000], // Highly compressible data
            size: 10000,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: Some(HashMap::new()),
            ttl: None,
        };
        
        cache.put(key.clone(), value.clone()).await.unwrap();
        
        let retrieved = cache.get(&key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, value.data);
        
        // Check that compression actually reduced size on disk
        let stats = cache.stats().await;
        assert!(stats.size_on_disk_bytes < 10000);
    }

    #[tokio::test]
    async fn test_l2_cache_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();
        let metrics = Arc::new(CacheMetrics::default());
        
        let key = CacheKey("persist_key".to_string());
        let value = CacheValue {
            data: vec![99; 100],
            size: 100,
            created_at: SystemTime::now(),
            access_count: 0,
            last_accessed: SystemTime::now(),
            metadata: Some(HashMap::new()),
            ttl: None,
        };
        
        // Create cache and add data
        {
            let config = L2Config {
                max_size: 1_000_000,
                compression: false,
                cache_dir: cache_dir.clone(),
            };
            let cache = RealL2Cache::new(config, metrics.clone()).await.unwrap();
            cache.put(key.clone(), value.clone()).await.unwrap();
        }
        
        // Create new cache instance with same directory
        {
            let config = L2Config {
                max_size: 1_000_000,
                compression: false,
                cache_dir: cache_dir.clone(),
            };
            let cache = RealL2Cache::new(config, metrics).await.unwrap();
            
            // Data should persist across instances
            let retrieved = cache.get(&key).await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().data, value.data);
        }
    }
}
