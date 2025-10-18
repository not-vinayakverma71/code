// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Cached embedding layer using stable IDs (CST-B05-3)
//!
//! Wraps embedding model with intelligent caching:
//! - Check cache for stable ID before generating embedding
//! - Store newly generated embeddings by stable ID
//! - Reuse embeddings for unchanged nodes in incremental updates

use std::sync::Arc;
use std::path::PathBuf;
use crate::error::{Error, Result};
use crate::indexing::{StableIdEmbeddingCache, CacheEntry, IncrementalDetector, ChangeSet};
use crate::processors::cst_to_ast_pipeline::{CstNode, AstNode};
use std::time::{SystemTime, UNIX_EPOCH};

/// Statistics for cached embedding operations
#[derive(Debug, Default, Clone)]
pub struct EmbeddingStats {
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub embeddings_generated: usize,
    pub embeddings_reused: usize,
}

impl EmbeddingStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

/// Trait for embedding models (abstraction for testing)
pub trait EmbeddingModel: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>>;
}

/// Cached embedder that integrates stable ID cache with embedding model
pub struct CachedEmbedder {
    /// Underlying embedding model
    model: Arc<dyn EmbeddingModel>,
    
    /// Stable ID → embedding cache
    cache: Arc<StableIdEmbeddingCache>,
    
    /// Change detector for incremental updates
    detector: Arc<parking_lot::RwLock<IncrementalDetector>>,
    
    /// Statistics
    stats: parking_lot::RwLock<EmbeddingStats>,
}

impl CachedEmbedder {
    pub fn new(model: Arc<dyn EmbeddingModel>) -> Self {
        Self {
            model,
            cache: Arc::new(StableIdEmbeddingCache::new()),
            detector: Arc::new(parking_lot::RwLock::new(IncrementalDetector::new())),
            stats: parking_lot::RwLock::new(EmbeddingStats::default()),
        }
    }
    
    pub fn with_cache_capacity(model: Arc<dyn EmbeddingModel>, capacity: usize) -> Self {
        Self {
            model,
            cache: Arc::new(StableIdEmbeddingCache::with_capacity(capacity)),
            detector: Arc::new(parking_lot::RwLock::new(IncrementalDetector::new())),
            stats: parking_lot::RwLock::new(EmbeddingStats::default()),
        }
    }
    
    /// Embed a single node, using cache if available
    pub fn embed_node(
        &self,
        node: &CstNode,
        file_path: &PathBuf,
    ) -> Result<Vec<f32>> {
        if let Some(stable_id) = node.stable_id {
            // Try cache first
            if let Some(entry) = self.cache.get(stable_id) {
                self.stats.write().cache_hits += 1;
                self.stats.write().embeddings_reused += 1;
                return Ok(entry.embedding);
            }
            
            // Cache miss - generate embedding
            self.stats.write().cache_misses += 1;
            let embedding = self.model.embed(&node.text)?;
            self.stats.write().embeddings_generated += 1;
            
            // Store in cache
            let entry = CacheEntry {
                embedding: embedding.clone(),
                source_text: node.text.clone(),
                node_kind: node.kind.clone(),
                timestamp: current_timestamp(),
                file_path: file_path.clone(),
            };
            self.cache.insert(stable_id, entry);
            
            Ok(embedding)
        } else {
            // No stable ID - just generate embedding (no caching)
            self.stats.write().cache_misses += 1;
            self.stats.write().embeddings_generated += 1;
            self.model.embed(&node.text)
        }
    }
    
    /// Embed AST node using metadata's stable ID
    pub fn embed_ast_node(
        &self,
        node: &AstNode,
    ) -> Result<Vec<f32>> {
        if let Some(stable_id) = node.metadata.stable_id {
            // Try cache first
            if let Some(entry) = self.cache.get(stable_id) {
                self.stats.write().cache_hits += 1;
                self.stats.write().embeddings_reused += 1;
                return Ok(entry.embedding);
            }
            
            // Cache miss - generate embedding from node text/identifier
            self.stats.write().cache_misses += 1;
            let text = if !node.text.is_empty() {
                node.text.as_str()
            } else if let Some(ref id) = node.identifier {
                id.as_str()
            } else {
                ""
            };
            let embedding = self.model.embed(text)?;
            self.stats.write().embeddings_generated += 1;
            
            // Store in cache
            let entry = CacheEntry {
                embedding: embedding.clone(),
                source_text: text.to_string(),
                node_kind: format!("{:?}", node.node_type),
                timestamp: current_timestamp(),
                file_path: node.metadata.source_file.clone().unwrap_or_default(),
            };
            self.cache.insert(stable_id, entry);
            
            Ok(embedding)
        } else {
            // No stable ID - just generate embedding
            self.stats.write().cache_misses += 1;
            self.stats.write().embeddings_generated += 1;
            let text = if !node.text.is_empty() {
                node.text.as_str()
            } else if let Some(ref id) = node.identifier {
                id.as_str()
            } else {
                ""
            };
            self.model.embed(text)
        }
    }
    
    /// Perform incremental embedding update for a file
    /// Returns: (embeddings, changeset) where embeddings[i] corresponds to changeset nodes
    pub fn embed_file_incremental(
        &self,
        cst: &CstNode,
        file_path: &PathBuf,
    ) -> Result<(Vec<(u64, Vec<f32>)>, ChangeSet)> {
        // Detect changes
        let changeset = self.detector.write().detect_changes(file_path, cst);
        
        let mut embeddings = Vec::new();
        
        // For unchanged nodes, retrieve from cache
        for stable_id in &changeset.unchanged {
            if let Some(entry) = self.cache.get(*stable_id) {
                embeddings.push((*stable_id, entry.embedding));
                self.stats.write().cache_hits += 1;
                self.stats.write().embeddings_reused += 1;
            }
        }
        
        // For modified and added nodes, generate new embeddings
        let nodes_to_embed: Vec<_> = changeset.modified.iter()
            .chain(changeset.added.iter())
            .copied()
            .collect();
        
        if !nodes_to_embed.is_empty() {
            // Collect nodes by stable ID
            let mut node_map = std::collections::HashMap::new();
            collect_nodes_by_id(cst, &mut node_map);
            
            // Generate embeddings in batch
            let texts: Vec<&str> = nodes_to_embed.iter()
                .filter_map(|id| node_map.get(id).map(|n| n.text.as_str()))
                .collect();
            
            if !texts.is_empty() {
                let batch_embeddings = self.model.embed_batch(texts)?;
                self.stats.write().cache_misses += nodes_to_embed.len();
                self.stats.write().embeddings_generated += nodes_to_embed.len();
                
                // Store results
                for (i, stable_id) in nodes_to_embed.iter().enumerate() {
                    if let (Some(node), Some(embedding)) = (node_map.get(stable_id), batch_embeddings.get(i)) {
                        let entry = CacheEntry {
                            embedding: embedding.clone(),
                            source_text: node.text.clone(),
                            node_kind: node.kind.clone(),
                            timestamp: current_timestamp(),
                            file_path: file_path.clone(),
                        };
                        self.cache.insert(*stable_id, entry);
                        embeddings.push((*stable_id, embedding.clone()));
                    }
                }
            }
        }
        
        // Remove deleted nodes from cache
        for stable_id in &changeset.deleted {
            self.cache.remove(*stable_id);
        }
        
        Ok((embeddings, changeset))
    }
    
    /// Clear cache for a specific file
    pub fn invalidate_file(&self, file_path: &PathBuf) {
        self.cache.invalidate_file(file_path);
        self.detector.write().remove_snapshot(file_path);
    }
    
    /// Get current statistics
    pub fn stats(&self) -> EmbeddingStats {
        self.stats.read().clone()
    }
    
    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.write() = EmbeddingStats::default();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (u64, u64, u64, usize) {
        self.cache.stats()
    }
}

/// Helper to collect all nodes by stable ID
fn collect_nodes_by_id(node: &CstNode, map: &mut std::collections::HashMap<u64, CstNode>) {
    if let Some(stable_id) = node.stable_id {
        map.insert(stable_id, node.clone());
    }
    for child in &node.children {
        collect_nodes_by_id(child, map);
    }
}

/// Get current UNIX timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Mock embedding model for testing
    struct MockEmbedder;
    
    impl EmbeddingModel for MockEmbedder {
        fn embed(&self, text: &str) -> Result<Vec<f32>> {
            // Simple hash-based deterministic embedding
            let hash = text.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));
            Ok(vec![hash as f32 / 1000.0, (hash % 100) as f32, text.len() as f32])
        }
        
        fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
            texts.into_iter().map(|t| self.embed(t)).collect()
        }
    }
    
    fn create_test_node(stable_id: u64, text: &str) -> CstNode {
        CstNode {
            kind: "test".to_string(),
            text: text.to_string(),
            start_byte: 0,
            end_byte: text.len(),
            start_position: (0, 0),
            end_position: (0, 0),
            is_named: true,
            is_missing: false,
            is_extra: false,
            field_name: None,
            children: vec![],
            stable_id: Some(stable_id),
        }
    }
    
    #[test]
    fn test_cache_hit_on_second_embed() {
        let embedder = CachedEmbedder::new(Arc::new(MockEmbedder));
        let file = PathBuf::from("/test.rs");
        let node = create_test_node(1, "fn test() {}");
        
        // First embed - cache miss
        let emb1 = embedder.embed_node(&node, &file).unwrap();
        let stats1 = embedder.stats();
        assert_eq!(stats1.cache_misses, 1);
        assert_eq!(stats1.embeddings_generated, 1);
        
        // Second embed - cache hit
        let emb2 = embedder.embed_node(&node, &file).unwrap();
        let stats2 = embedder.stats();
        assert_eq!(stats2.cache_hits, 1);
        assert_eq!(stats2.embeddings_reused, 1);
        
        // Should be same embedding
        assert_eq!(emb1, emb2);
    }
    
    #[test]
    fn test_incremental_update_reuses_unchanged() {
        let embedder = CachedEmbedder::new(Arc::new(MockEmbedder));
        let file = PathBuf::from("/test.rs");
        
        // Initial parse
        let mut cst1 = create_test_node(1, "root");
        cst1.children.push(create_test_node(2, "fn a() {}"));
        cst1.children.push(create_test_node(3, "fn b() {}"));
        
        let (embeddings1, changeset1) = embedder.embed_file_incremental(&cst1, &file).unwrap();
        
        // All nodes are new
        assert_eq!(changeset1.added.len(), 3);
        assert_eq!(embeddings1.len(), 3);
        
        // Second parse - same content
        let (embeddings2, changeset2) = embedder.embed_file_incremental(&cst1, &file).unwrap();
        
        // All nodes unchanged
        assert_eq!(changeset2.unchanged.len(), 3);
        assert_eq!(embeddings2.len(), 3);
        
        let stats = embedder.stats();
        assert_eq!(stats.embeddings_reused, 3);
    }
    
    #[test]
    fn test_incremental_update_detects_modification() {
        let embedder = CachedEmbedder::new(Arc::new(MockEmbedder));
        let file = PathBuf::from("/test.rs");
        
        // Initial parse
        let mut cst1 = create_test_node(1, "root");
        cst1.children.push(create_test_node(2, "fn old() {}"));
        
        embedder.embed_file_incremental(&cst1, &file).unwrap();
        
        // Modify node 2
        let mut cst2 = create_test_node(1, "root");
        cst2.children.push(create_test_node(2, "fn new() {}"));
        
        let (embeddings, changeset) = embedder.embed_file_incremental(&cst2, &file).unwrap();
        
        assert_eq!(changeset.unchanged.len(), 1); // Root unchanged
        assert_eq!(changeset.modified.len(), 1); // Child modified
        assert!(changeset.modified.contains(&2));
    }
    
    #[test]
    fn test_file_invalidation() {
        let embedder = CachedEmbedder::new(Arc::new(MockEmbedder));
        let file = PathBuf::from("/test.rs");
        let node = create_test_node(1, "fn test() {}");
        
        // Embed and cache
        embedder.embed_node(&node, &file).unwrap();
        
        // Invalidate file
        embedder.invalidate_file(&file);
        
        // Next embed should be cache miss
        embedder.reset_stats();
        embedder.embed_node(&node, &file).unwrap();
        let stats = embedder.stats();
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cache_hits, 0);
    }
}
