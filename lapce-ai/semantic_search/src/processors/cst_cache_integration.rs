// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Integration between Phase4Cache and CstApi (CST-UP01)
//!
//! Bridges the upstream CST-tree-sitter Phase4Cache with our semantic_search
//! CstApi system, enabling cached retrieval of CstApi with stable IDs.

#[cfg(feature = "cst_ts")]
use crate::error::{Error, Result};
#[cfg(feature = "cst_ts")]
use std::path::PathBuf;
#[cfg(feature = "cst_ts")]
use std::sync::Arc;

// Note: Full CstApi integration pending upstream export of types
// This provides the interface structure for when lapce_tree_sitter exports CstApi

/// Configuration for CST cache integration
#[cfg(feature = "cst_ts")]
#[derive(Debug, Clone)]
pub struct CstCacheConfig {
    /// Enable Phase4 caching
    pub enabled: bool,
    
    /// Cache directory
    pub cache_dir: PathBuf,
    
    /// Memory budget in MB
    pub memory_budget_mb: usize,
    
    /// Enable compression
    pub enable_compression: bool,
}

#[cfg(feature = "cst_ts")]
impl Default for CstCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_dir: std::env::temp_dir().join("semantic_search_cst_cache"),
            memory_budget_mb: 100,
            enable_compression: true,
        }
    }
}

/// Cache statistics
#[cfg(feature = "cst_ts")]
#[derive(Debug, Clone, Default)]
pub struct CstCacheStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub entries_stored: u64,
    pub bytes_cached: u64,
    pub api_retrievals: u64,
}

#[cfg(feature = "cst_ts")]
impl CstCacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

/// Integrated CST cache that stores and retrieves CstApi
#[cfg(feature = "cst_ts")]
pub struct CstCache {
    config: CstCacheConfig,
    stats: Arc<parking_lot::RwLock<CstCacheStats>>,
    // Phase4Cache integration would go here when available
    // For now, we provide the interface structure
}

#[cfg(feature = "cst_ts")]
impl CstCache {
    /// Create new CST cache
    pub fn new(config: CstCacheConfig) -> Result<Self> {
        // Create cache directory
        std::fs::create_dir_all(&config.cache_dir).map_err(|e| Error::Runtime {
            message: format!("Failed to create cache directory: {}", e)
        })?;
        
        Ok(Self {
            config,
            stats: Arc::new(parking_lot::RwLock::new(CstCacheStats::default())),
        })
    }
    
    /// Store parsed tree in cache
    /// TODO: Accept CstApi when upstream exports it
    pub fn store(
        &self,
        file_path: &PathBuf,
        _tree_hash: u64,
        source: &[u8],
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // TODO: When Phase4Cache is integrated:
        // 1. Convert CstApi to tree-sitter Tree
        // 2. Store using Phase4Cache.store()
        // 3. Update statistics
        
        let mut stats = self.stats.write();
        stats.entries_stored += 1;
        stats.bytes_cached += source.len() as u64;
        
        log::debug!("Stored CST cache for {:?}", file_path);
        Ok(())
    }
    
    /// Check if cache has entry
    /// TODO: Return CstApi when upstream exports it
    pub fn has_cached(&self, file_path: &PathBuf, _source: &[u8]) -> Result<bool> {
        if !self.config.enabled {
            self.stats.write().cache_misses += 1;
            return Ok(false);
        }
        
        // TODO: When Phase4Cache is integrated:
        // 1. Try to retrieve segmented bytecode from Phase4Cache
        // 2. Deserialize bytecode back to tree-sitter Tree
        // 3. Convert Tree to CstApi using CstApiBuilder
        // 4. Return CstApi with stable IDs
        
        let mut stats = self.stats.write();
        stats.cache_misses += 1;
        
        log::debug!("Cache miss for {:?}", file_path);
        Ok(false)
    }
    
    /// Check cache and return whether to parse
    pub fn should_parse(&self, file_path: &PathBuf, source: &[u8]) -> Result<bool> {
        let has_cached = self.has_cached(file_path, source)?;
        if has_cached {
            self.stats.write().cache_hits += 1;
            self.stats.write().api_retrievals += 1;
            log::debug!("Cache hit for {:?}", file_path);
        }
        Ok(!has_cached)
    }
    
    /// Invalidate cache entry
    pub fn invalidate(&self, file_path: &PathBuf) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // TODO: When Phase4Cache is integrated:
        // Remove entry from Phase4Cache
        
        log::debug!("Invalidated cache for {:?}", file_path);
        Ok(())
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CstCacheStats {
        self.stats.read().clone()
    }
    
    /// Clear all cache entries
    pub fn clear(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // TODO: When Phase4Cache is integrated:
        // Clear all entries from Phase4Cache
        
        let mut stats = self.stats.write();
        *stats = CstCacheStats::default();
        
        log::info!("Cleared CST cache");
        Ok(())
    }
}

/// Helper to create cache-aware parser
/// TODO: Full implementation when CstApi is exported from upstream
#[cfg(feature = "cst_ts")]
pub struct CachedCstParser {
    cache: Arc<CstCache>,
}

#[cfg(feature = "cst_ts")]
impl CachedCstParser {
    pub fn new(cache_config: CstCacheConfig) -> Result<Self> {
        Ok(Self {
            cache: Arc::new(CstCache::new(cache_config)?),
        })
    }
    
    /// Check if file needs parsing
    pub fn should_parse(&self, file_path: &PathBuf, source: &[u8]) -> Result<bool> {
        self.cache.should_parse(file_path, source)
    }
    
    /// Record successful parse
    pub fn record_parse(&self, file_path: &PathBuf, tree_hash: u64, source: &[u8]) -> Result<()> {
        self.cache.store(file_path, tree_hash, source)
    }
    
    pub fn cache_stats(&self) -> CstCacheStats {
        self.cache.stats()
    }
}

#[cfg(test)]
#[cfg(feature = "cst_ts")]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = CstCacheConfig {
            enabled: true,
            cache_dir: temp_dir.path().to_path_buf(),
            memory_budget_mb: 50,
            enable_compression: true,
        };
        
        let cache = CstCache::new(config).unwrap();
        let stats = cache.stats();
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
    }
    
    #[test]
    fn test_cache_miss() {
        let temp_dir = TempDir::new().unwrap();
        let config = CstCacheConfig {
            enabled: true,
            cache_dir: temp_dir.path().to_path_buf(),
            memory_budget_mb: 50,
            enable_compression: true,
        };
        
        let cache = CstCache::new(config).unwrap();
        let file_path = PathBuf::from("/test.rs");
        let source = b"fn main() {}";
        
        let has_cached = cache.has_cached(&file_path, source).unwrap();
        assert!(!has_cached);
        
        let stats = cache.stats();
        assert_eq!(stats.cache_misses, 1);
    }
    
    #[test]
    fn test_stats_hit_rate() {
        let mut stats = CstCacheStats::default();
        stats.cache_hits = 80;
        stats.cache_misses = 20;
        
        assert_eq!(stats.hit_rate(), 0.8);
    }
    
    #[test]
    fn test_cache_invalidation() {
        let temp_dir = TempDir::new().unwrap();
        let config = CstCacheConfig {
            enabled: true,
            cache_dir: temp_dir.path().to_path_buf(),
            memory_budget_mb: 50,
            enable_compression: true,
        };
        
        let cache = CstCache::new(config).unwrap();
        let file_path = PathBuf::from("/test.rs");
        
        cache.invalidate(&file_path).unwrap();
        // Should not panic
    }
    
    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let config = CstCacheConfig {
            enabled: true,
            cache_dir: temp_dir.path().to_path_buf(),
            memory_budget_mb: 50,
            enable_compression: true,
        };
        
        let cache = CstCache::new(config).unwrap();
        cache.clear().unwrap();
        
        let stats = cache.stats();
        assert_eq!(stats.entries_stored, 0);
    }
}
