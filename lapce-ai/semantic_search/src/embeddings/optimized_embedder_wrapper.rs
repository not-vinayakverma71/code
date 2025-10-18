// Optimized Embedder Wrapper with HierarchicalCache + MmapStorage
// Provides transparent caching layer for any IEmbedder implementation

use crate::error::{Error, Result};
use crate::embeddings::embedder_interface::{IEmbedder, EmbeddingResponse, EmbedderInfo};
use crate::storage::hierarchical_cache::{HierarchicalCache, CacheConfig};
use crate::storage::mmap_storage::ConcurrentMmapStorage;
use crate::embeddings::zstd_compression::{ZstdCompressor, CompressionConfig};

use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tokio::sync::RwLock;
use async_trait::async_trait;
use tracing::{info, debug, warn};
use sha2::{Sha256, Digest};

/// Configuration for the optimized embedder
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    pub cache_dir: PathBuf,
    pub enable_l1_cache: bool,
    pub enable_l2_cache: bool,
    pub enable_l3_mmap: bool,
    pub l1_max_size_mb: f64,
    pub l2_max_size_mb: f64,
    pub l3_max_size_mb: f64,
    pub compression_level: i32,
    pub enable_stats: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from(".embeddings_cache"),
            enable_l1_cache: true,
            enable_l2_cache: true,
            enable_l3_mmap: true,
            l1_max_size_mb: 2.0,
            l2_max_size_mb: 5.0,
            l3_max_size_mb: 100.0,
            compression_level: 3,
            enable_stats: true,
        }
    }
}

/// Statistics for the optimized embedder
#[derive(Debug, Clone, Default)]
pub struct OptimizerStats {
    pub total_requests: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub api_calls_saved: usize,
    pub compression_bytes_saved: usize,
    pub average_compression_ratio: f32,
}

/// Optimized wrapper that adds caching and compression to any embedder
pub struct OptimizedEmbedderWrapper {
    base_embedder: Arc<dyn IEmbedder>,
    cache: Arc<HierarchicalCache>,
    mmap_storage: Arc<ConcurrentMmapStorage>,
    compressor: Arc<RwLock<ZstdCompressor>>,
    stats: Arc<RwLock<OptimizerStats>>,
    config: OptimizerConfig,
    model_id: String,
}

impl OptimizedEmbedderWrapper {
    /// Create new optimized wrapper around a base embedder
    pub fn new(
        base_embedder: Arc<dyn IEmbedder>,
        config: OptimizerConfig,
        model_id: String,
    ) -> Result<Self> {
        // Create cache directory
        std::fs::create_dir_all(&config.cache_dir).map_err(|e| Error::Runtime {
            message: format!("Failed to create cache directory: {}", e)
        })?;

        // Initialize hierarchical cache
        let cache_config = CacheConfig {
            l1_max_size_mb: config.l1_max_size_mb,
            l1_max_entries: 100,
            l2_max_size_mb: config.l2_max_size_mb,
            l2_max_entries: 500,
            l3_max_size_mb: config.l3_max_size_mb,
            promotion_threshold: 3,
            demotion_timeout: std::time::Duration::from_secs(300),
            bloom_filter_size: 10000,
            enable_statistics: config.enable_stats,
        };

        let cache = Arc::new(HierarchicalCache::new(cache_config, &config.cache_dir)?);

        // Initialize memory-mapped storage for L3
        let mmap_storage = Arc::new(ConcurrentMmapStorage::new(
            &config.cache_dir.join("mmap"),
            (config.l3_max_size_mb * 1024.0 * 1024.0) as u64,
        )?);

        // Initialize compressor
        let compression_config = CompressionConfig {
            compression_level: config.compression_level,
            enable_dictionary: true,
            enable_checksum: true,
            chunk_size: 100,
        };
        let compressor = Arc::new(RwLock::new(ZstdCompressor::new(compression_config)));

        info!(
            "Initialized OptimizedEmbedderWrapper for model '{}' with cache at {:?}",
            model_id, config.cache_dir
        );

        Ok(Self {
            base_embedder,
            cache,
            mmap_storage,
            compressor,
            stats: Arc::new(RwLock::new(OptimizerStats::default())),
            config,
            model_id,
        })
    }

    /// Generate cache key for text
    fn generate_cache_key(&self, text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.model_id.as_bytes());
        hasher.update(b":");
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check all cache tiers for embedding
    async fn check_cache(&self, key: &str) -> Option<Vec<f32>> {
        // Try hierarchical cache first (L1/L2) - not async
        if let Ok(Some(embedding)) = self.cache.get(key) {
            debug!("Cache hit for key: {}", key);
            return Some(embedding.to_vec());
        }

        // Try L3 mmap storage
        if self.config.enable_l3_mmap && self.mmap_storage.contains(key) {
            if let Ok(embedding) = self.mmap_storage.get(key) {
                debug!("L3 mmap hit for key: {}", key);
                // Promote to L2 cache for faster future access
                let _ = self.cache.put(key, embedding.clone());
                return Some(embedding);
            }
        }

        None
    }

    /// Store embedding in cache
    async fn store_in_cache(&self, key: &str, embedding: &[f32]) -> Result<()> {
        // Store in hierarchical cache (handles L1/L2 automatically) - not async
        self.cache.put(key, embedding.to_vec())?;

        // Also store compressed version in mmap for L3
        if self.config.enable_l3_mmap {
            let mut compressor = self.compressor.write().await;
            let compressed = compressor.compress_embedding(embedding, key)?;
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.compression_bytes_saved += compressed.original_size - compressed.compressed_size;
            stats.average_compression_ratio = 
                (stats.average_compression_ratio + compressed.compression_ratio) / 2.0;
            drop(stats);

            // Store in mmap (not async)
            self.mmap_storage.store(key, embedding)?;
        }

        Ok(())
    }

    /// Get statistics report
    pub async fn get_stats_report(&self) -> String {
        let stats = self.stats.read().await;
        let cache_stats = self.cache.get_stats();
        
        format!(
            "OptimizedEmbedderWrapper Statistics:\n\
             ├─ Total Requests: {}\n\
             ├─ Cache Hits: {} ({:.1}%)\n\
             ├─ Cache Misses: {}\n\
             ├─ API Calls Saved: {}\n\
             ├─ Compression Bytes Saved: {} KB\n\
             ├─ Average Compression Ratio: {:.2}x\n\
             ├─ L1 Hit Rate: {:.1}%\n\
             └─ Overall Cache Hit Rate: {:.1}%",
            stats.total_requests,
            stats.cache_hits,
            if stats.total_requests > 0 {
                (stats.cache_hits as f64 / stats.total_requests as f64) * 100.0
            } else { 0.0 },
            stats.cache_misses,
            stats.api_calls_saved,
            stats.compression_bytes_saved / 1024,
            stats.average_compression_ratio,
            cache_stats.l1_hit_rate() * 100.0,
            cache_stats.overall_hit_rate() * 100.0
        )
    }

    /// Print statistics to console
    pub async fn print_stats_report(&self) {
        println!("{}", self.get_stats_report().await);
    }
}

#[async_trait]
impl IEmbedder for OptimizedEmbedderWrapper {
    async fn create_embeddings(
        &self,
        texts: Vec<String>,
        model: Option<&str>,
    ) -> Result<EmbeddingResponse> {
        let mut embeddings = Vec::new();
        let mut texts_to_embed = Vec::new();
        let mut text_indices = Vec::new();
        
        // Update request count
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += texts.len();
        }

        // Check cache for each text
        for (idx, text) in texts.iter().enumerate() {
            let cache_key = self.generate_cache_key(text);
            
            if let Some(cached_embedding) = self.check_cache(&cache_key).await {
                // Cache hit
                embeddings.push((idx, cached_embedding));
                
                let mut stats = self.stats.write().await;
                stats.cache_hits += 1;
                stats.api_calls_saved += 1;
            } else {
                // Cache miss - need to generate
                texts_to_embed.push(text.clone());
                text_indices.push(idx);
                
                let mut stats = self.stats.write().await;
                stats.cache_misses += 1;
            }
        }

        // Generate embeddings for cache misses
        if !texts_to_embed.is_empty() {
            debug!(
                "Generating {} embeddings (cached: {})",
                texts_to_embed.len(),
                embeddings.len()
            );

            let response = self.base_embedder
                .create_embeddings(texts_to_embed.clone(), model)
                .await?;

            // Store new embeddings in cache
            for (text_idx, embedding) in text_indices.iter().zip(response.embeddings.iter()) {
                let text = &texts[*text_idx];
                let cache_key = self.generate_cache_key(text);
                
                // Store in cache (async, don't block)
                let cache_key_clone = cache_key.clone();
                let embedding_clone = embedding.clone();
                let self_clone = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = self_clone.store_in_cache(&cache_key_clone, &embedding_clone).await {
                        warn!("Failed to cache embedding: {}", e);
                    }
                });

                embeddings.push((*text_idx, embedding.clone()));
            }
        }

        // Sort by original index to maintain order
        embeddings.sort_by_key(|(idx, _)| *idx);
        let final_embeddings: Vec<Vec<f32>> = embeddings.into_iter()
            .map(|(_, emb)| emb)
            .collect();

        Ok(EmbeddingResponse {
            embeddings: final_embeddings,
            usage: None,
        })
    }

    async fn validate_configuration(&self) -> Result<(bool, Option<String>)> {
        // Validate base embedder
        let (valid, message) = self.base_embedder.validate_configuration().await?;
        
        if valid {
            let stats_report = self.get_stats_report().await;
            Ok((true, Some(format!(
                "{}\n\nOptimization Layer:\n{}",
                message.unwrap_or_default(),
                stats_report
            ))))
        } else {
            Ok((valid, message))
        }
    }

    fn embedder_info(&self) -> EmbedderInfo {
        self.base_embedder.embedder_info()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Implement Clone for Arc sharing
impl Clone for OptimizedEmbedderWrapper {
    fn clone(&self) -> Self {
        Self {
            base_embedder: self.base_embedder.clone(),
            cache: self.cache.clone(),
            mmap_storage: self.mmap_storage.clone(),
            compressor: self.compressor.clone(),
            stats: self.stats.clone(),
            config: self.config.clone(),
            model_id: self.model_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::embedder_interface::AvailableEmbedders;

    // Mock embedder for testing
    struct MockEmbedder {
        call_count: Arc<RwLock<usize>>,
    }

    #[async_trait]
    impl IEmbedder for MockEmbedder {
        async fn create_embeddings(
            &self,
            texts: Vec<String>,
            _model: Option<&str>,
        ) -> Result<EmbeddingResponse> {
            let mut count = self.call_count.write().await;
            *count += 1;
            
            // Generate fake embeddings
            let embeddings = texts.iter()
                .map(|_| vec![0.1; 384])
                .collect();
            
            Ok(EmbeddingResponse {
                embeddings,
                usage: None,
            })
        }

        async fn validate_configuration(&self) -> Result<(bool, Option<String>)> {
            Ok((true, Some("Mock embedder ready".to_string())))
        }

        fn embedder_info(&self) -> EmbedderInfo {
            EmbedderInfo {
                name: AvailableEmbedders::OpenAi,
            }
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[tokio::test]
    async fn test_cache_hits() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = OptimizerConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mock_embedder = Arc::new(MockEmbedder {
            call_count: Arc::new(RwLock::new(0)),
        });

        let wrapper = OptimizedEmbedderWrapper::new(
            mock_embedder.clone(),
            config,
            "test-model".to_string(),
        ).unwrap();

        // First call - should hit the base embedder
        let texts = vec!["test1".to_string(), "test2".to_string()];
        let _ = wrapper.create_embeddings(texts.clone(), None).await.unwrap();
        
        // Wait for cache to be populated
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Second call with same texts - should hit cache
        let _ = wrapper.create_embeddings(texts, None).await.unwrap();

        // Check that base embedder was only called once
        let call_count = *mock_embedder.call_count.read().await;
        assert_eq!(call_count, 1);

        // Check stats
        let stats = wrapper.stats.read().await;
        assert_eq!(stats.total_requests, 4); // 2 texts * 2 calls
        assert_eq!(stats.cache_hits, 2); // Second call should be all hits
        assert_eq!(stats.cache_misses, 2); // First call should be all misses
    }
}
