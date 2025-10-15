/// Working Cache System - L1 (HashMap), L2 (Sled), L3 (Redis optional)
use std::collections::HashMap;
use sled::Tree;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::global_sled::GLOBAL_SLED_DB;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub timestamp: u64,
}

pub struct WorkingCacheSystem {
    l1: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    l2_tree: Tree,  // Tree within global sled DB
    tree_name: String,  // Unique tree identifier
    // L3 Redis disabled for now (optional)
}

impl WorkingCacheSystem {
    pub async fn new() -> Result<Self> {
        // L1: Simple HashMap cache
        let l1 = Arc::new(RwLock::new(HashMap::new()));
        
        // L2: Use unique tree in global sled DB
        let tree_name = format!("cache_{}", Uuid::new_v4());
        let l2_tree = GLOBAL_SLED_DB.open_tree(&tree_name)?;
        
        Ok(Self {
            l1,
            l2_tree,
            tree_name,
        })
    }
    
    /// Get from cache (checks L1 -> L2 -> L3)
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        // Check L1
        if let Some(value) = self.l1.read().await.get(key) {
            return Some(value.clone());
        }
        
        // Check L2 (tree)
        if let Ok(Some(value)) = self.l2_tree.get(key.as_bytes()) {
            let value = value.to_vec();
            // Promote to L1
            self.l1.write().await.insert(key.to_string(), value.clone());
            return Some(value);
        }
        
        // L3 disabled for now
        None
    }
    
    /// Set value in cache
    pub async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        // Set in L1
        self.l1.write().await.insert(key.to_string(), value.clone());
        
        // Set in L2 (tree)
        self.l2_tree.insert(key.as_bytes(), value)?;
        
        // L3 disabled
        
        Ok(())
    }
    
    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let l1_size = self.l1.read().await.len() as u64;
        let l2_size = self.l2_tree.len();
        
        CacheStats {
            l1_entries: l1_size,
            l2_entries: l2_size,
            l3_connected: false, // L3 disabled
        }
    }
    
    /// Clear all caches
    pub async fn clear(&self) -> Result<()> {
        self.l1.write().await.clear();
        self.l2_tree.clear()?;
        
        // L3 disabled
        
        Ok(())
    }
}

// NOTE: No Drop implementation!
// Trees persist in global DB.
// Global DB cleanup happens on process exit - safe with sled's background threads.

#[derive(Debug)]
pub struct CacheStats {
    pub l1_entries: u64,
    pub l2_entries: usize,
    pub l3_connected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_system() {
        let cache = WorkingCacheSystem::new().await.unwrap();
        
        // Test set
        cache.set("test_key", vec![1, 2, 3, 4]).await.unwrap();
        
        // Test get
        let value = cache.get("test_key").await.unwrap();
        assert_eq!(value, vec![1, 2, 3, 4]);
        
        // Test stats
        let stats = cache.stats().await;
        assert!(stats.l1_entries > 0);
    }
}
