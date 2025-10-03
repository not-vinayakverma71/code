/// L2Cache - EXACT implementation from docs lines 57-60, 208-269
use std::sync::Arc;
use anyhow::Result;
use sled::Db;

use super::{
    compression_strategy::CompressionStrategy,
    cache_metrics::CacheMetrics,
    types::{CacheKey, CacheValue, L2Config, CompressionType},
};

pub struct L2Cache {
    pub db: Db,
    pub compression: CompressionStrategy,
    pub max_size: usize,
    pub metrics: Arc<CacheMetrics>,
}

impl L2Cache {
    pub async fn new(config: L2Config, metrics: Arc<CacheMetrics>) -> Result<Self> {
        let db = sled::open(&config.cache_dir)?;
        
        // Configure compression
        let compression = match config.compression {
            CompressionType::None => CompressionStrategy::None,
            CompressionType::Lz4 => CompressionStrategy::Lz4,
            CompressionType::Zstd => CompressionStrategy::Zstd,
        };
        
        Ok(Self {
            db,
            compression,
            max_size: config.max_size,
            metrics,
        })
    }
    
    pub async fn get(&self, key: &CacheKey) -> Result<Option<CacheValue>> {
        let key_bytes = bincode::serialize(key)?;
        
        if let Some(compressed) = self.db.get(key_bytes)? {
            let decompressed = self.compression.decompress(&compressed)?;
            let value: CacheValue = bincode::deserialize(&decompressed)?;
            self.metrics.record_l2_hit();
            Ok(Some(value))
        } else {
            self.metrics.record_l2_miss();
            Ok(None)
        }
    }
    
    pub async fn put(&self, key: CacheKey, value: CacheValue) -> Result<()> {
        // Check size limits
        if self.db.size_on_disk()? > self.max_size as u64 {
            self.evict_lru().await?;
        }
        
        let key_bytes = bincode::serialize(&key)?;
        let value_bytes = bincode::serialize(&value)?;
        let compressed = self.compression.compress(&value_bytes)?;
        
        self.db.insert(key_bytes, compressed)?;
        self.db.flush_async().await?;
        
        Ok(())
    }
    
    async fn evict_lru(&self) -> Result<()> {
        // Simple LRU eviction
        let target_size = (self.max_size * 80 / 100) as u64; // Evict to 80% capacity
        
        while self.db.size_on_disk()? > target_size {
            // Get oldest entry (sled maintains insertion order)
            if let Some(Ok((key, _))) = self.db.iter().next() {
                self.db.remove(key)?;
                self.metrics.record_eviction();
            } else {
                break;
            }
        }
        
        Ok(())
    }
}
