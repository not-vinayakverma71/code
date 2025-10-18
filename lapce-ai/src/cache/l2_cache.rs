/// L2Cache - EXACT implementation from docs lines 57-60, 208-269
use std::sync::Arc;
use anyhow::Result;
use sled::Tree;
use uuid::Uuid;

use super::{
    compression_strategy::CompressionStrategy,
    cache_metrics::CacheMetrics,
    types::{CacheKey, CacheValue, L2Config, CompressionType},
};
use crate::global_sled::GLOBAL_SLED_DB;

pub struct L2Cache {
    pub tree: Tree,
    pub compression: CompressionStrategy,
    pub max_size: usize,
    pub metrics: Arc<CacheMetrics>,
}

impl L2Cache {
    pub async fn new(config: L2Config, metrics: Arc<CacheMetrics>) -> Result<Self> {
        // Use global sled with unique tree
        let tree_name = format!("l2_cache_{}_{}",
            config.cache_dir.to_string_lossy().replace('/', "_"),
            Uuid::new_v4()
        );
        let tree = GLOBAL_SLED_DB.open_tree(&tree_name)?;
        
        // Configure compression
        let compression = match config.compression {
            CompressionType::None => CompressionStrategy::None,
            CompressionType::Lz4 => CompressionStrategy::Lz4,
            CompressionType::Zstd => CompressionStrategy::Zstd,
        };
        
        Ok(Self {
            tree,
            compression,
            max_size: config.max_size,
            metrics,
        })
    }
    
    pub async fn get(&self, key: &CacheKey) -> Result<Option<CacheValue>> {
        let key_bytes = bincode::serialize(key)?;
        
        if let Some(compressed) = self.tree.get(key_bytes)? {
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
        // Check size limits (use entry count as proxy)
        if self.tree.len() > self.max_size / 1024 {
            self.evict_lru().await?;
        }
        
        let key_bytes = bincode::serialize(&key)?;
        let value_bytes = bincode::serialize(&value)?;
        let compressed = self.compression.compress(&value_bytes)?;
        
        self.tree.insert(key_bytes, compressed)?;
        self.tree.flush_async().await?;
        
        Ok(())
    }
    
    async fn evict_lru(&self) -> Result<()> {
        // Simple LRU eviction
        let target_entries = (self.max_size / 1024) * 80 / 100; // Evict to 80% capacity
        
        while self.tree.len() > target_entries {
            // Get oldest entry (sled maintains insertion order)
            if let Some(Ok((key, _))) = self.tree.iter().next() {
                self.tree.remove(key)?;
                self.metrics.record_eviction();
            } else {
                break;
            }
        }
        
        Ok(())
    }
}
