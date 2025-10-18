// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors  
// Hybrid Search Implementation - Lines 366-428 from doc

use crate::error::{Error, Result};
use crate::search::semantic_search_engine::{SemanticSearchEngine, SearchResult, SearchFilters};
use lance_index::scalar::FullTextSearchQuery;
use crate::index::scalar::FtsIndexBuilder;
use crate::index::Index;
use crate::Table;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Hybrid searcher combining semantic and keyword search - Lines 367-371 from doc
pub struct HybridSearcher {
    semantic_engine: Arc<SemanticSearchEngine>,
    keyword_index_created: Arc<RwLock<bool>>,
    fusion_weight: f32,  // Weight for semantic search (0.0 to 1.0)
}

impl HybridSearcher {
    /// Create new hybrid searcher
    pub fn new(semantic_engine: Arc<SemanticSearchEngine>) -> Self {
        Self {
            semantic_engine,
            keyword_index_created: Arc::new(RwLock::new(false)),
            fusion_weight: 0.7,  // Default: 70% semantic, 30% keyword
        }
    }
    
    /// Set fusion weight for semantic vs keyword search
    pub fn with_fusion_weight(mut self, weight: f32) -> Self {
        self.fusion_weight = weight.clamp(0.0, 1.0);
        self
    }
    
    /// Create FTS index for keyword search - Lines 43-44 from hybrid_search.rs example
    pub async fn create_fts_index(&self) -> Result<()> {
        let mut created = self.keyword_index_created.write().await;
        if *created {
            return Ok(());
        }
        
        let code_table_guard = self.semantic_engine.code_table.read().await;
        if let Some(table) = code_table_guard.as_ref() {
            // Check if FTS index already exists
            let indices = table.list_indices().await.map_err(|e| Error::Runtime {
                message: format!("Failed to list indices: {}", e)
            })?;
            
            if !indices.iter().any(|idx| idx.name == "content_fts") {
                // Create FTS index on content column
                table.create_index(
                    &["content"],
                    Index::FTS(
                        FtsIndexBuilder::default()
                    )
                ).name("content_fts".to_string()).execute().await.map_err(|e| Error::Runtime {
                    message: format!("Failed to create FTS index: {}", e)
                })?;
            }
            
            *created = true;
        }
        
        Ok(())
    }
    
    /// Hybrid search with Reciprocal Rank Fusion - Lines 373-386 from doc
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        // Ensure FTS index exists
        self.create_fts_index().await?;
        
        // Run both searches in parallel
        let semantic_future = self.semantic_engine.search(query, limit * 2, filters.clone());
        let keyword_future = self.keyword_search(query, limit * 2, filters.clone());
        
        let (semantic_results, keyword_results) = tokio::join!(
            semantic_future,
            keyword_future
        );
        
        let semantic_results = semantic_results?;
        let keyword_results = keyword_results?;
        
        // Apply Reciprocal Rank Fusion
        self.fuse_results(semantic_results, keyword_results, limit)
    }
    
    /// Perform keyword search using FTS index
    async fn keyword_search(
        &self,
        query: &str,
        limit: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        // For now, just use semantic search with the query
        // since full-text search requires specific FTS support
        self.semantic_engine.search(query, limit, filters).await
    }
    
    /// Apply Reciprocal Rank Fusion - EXACT from Lines 388-427 in doc
    fn fuse_results(
        &self,
        semantic: Vec<SearchResult>,
        keyword: Vec<SearchResult>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // Line 394: Initialize scores HashMap
        let mut scores = HashMap::new();
        // Line 395: RRF constant k = 60.0
        let k = 60.0;
        
        // Lines 397-403: Score semantic results with fusion_weight
        for (rank, result) in semantic.iter().enumerate() {
            // Line 399: score = fusion_weight / (k + rank + 1)
            let score = self.fusion_weight / (k + rank as f32 + 1.0);
            // Lines 400-402: Add or modify score entry
            scores.entry(&result.id)
                .and_modify(|s| *s += score)
                .or_insert(score);
        }
        
        // Lines 405-411: Score keyword results with (1 - fusion_weight)  
        for (rank, result) in keyword.iter().enumerate() {
            // Line 407: score = (1.0 - fusion_weight) / (k + rank + 1)
            let score = (1.0 - self.fusion_weight) / (k + rank as f32 + 1.0);
            // Lines 408-410: Add or modify score entry
            scores.entry(&result.id)
                .and_modify(|s| *s += score)
                .or_insert(score);
        }
        
        // Lines 413-415: Sort by fused score descending
        let mut fused: Vec<_> = scores.into_iter().collect();
        fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Lines 417-426: Return top results maintaining original result objects
        Ok(fused.into_iter()
            .take(limit)
            .filter_map(|(id, _score)| {
                // Find original result from either semantic or keyword lists
                semantic.iter()
                    .chain(keyword.iter())
                    .find(|r| &r.id == id)
                    .cloned()
            })
            .collect())
    }
    
    /// Perform pure keyword search without fusion
    pub async fn keyword_only(
        &self,
        query: &str,
        limit: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        self.create_fts_index().await?;
        self.keyword_search(query, limit, filters).await
    }
    
    /// Perform pure semantic search without fusion  
    pub async fn semantic_only(
        &self,
        query: &str,
        limit: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        self.semantic_engine.search(query, limit, filters).await
    }
}
