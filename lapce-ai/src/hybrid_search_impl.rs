/// HYBRID SEARCH ENGINE - Tantivy + LanceDB
/// Combines keyword and semantic search with Reciprocal Rank Fusion

use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    doc,
    query::QueryParser,
    schema::{Schema, SchemaBuilder, Field, STORED, TEXT},
    Document, Index, IndexReader, IndexWriter, ReloadPolicy,
};
use tracing::{info, debug};

use crate::real_lancedb_impl::{RealLanceDBEngine, SearchResult};

pub struct HybridSearchEngine {
    lancedb_engine: Arc<RealLanceDBEngine>,
    tantivy_index: Index,
    index_reader: IndexReader,
    query_parser: QueryParser,
    path_field: Field,
    content_field: Field,
    fusion_weight: f32,
}

impl HybridSearchEngine {
    pub async fn new(db_path: &str, index_path: &str) -> Result<Self> {
        info!("Initializing hybrid search engine");
        
        // Initialize LanceDB engine
        let lancedb_engine = Arc::new(RealLanceDBEngine::new(db_path).await?);
        
        // Create Tantivy schema
        let mut schema_builder = SchemaBuilder::default();
        let path_field = schema_builder.add_text_field("path", STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let schema = schema_builder.build();
        
        // Create or open Tantivy index
        let index = if Path::new(index_path).exists() {
            let dir = MmapDirectory::open(index_path)?;
            Index::open(dir)?
        } else {
            std::fs::create_dir_all(index_path)?;
            let dir = MmapDirectory::open(index_path)?;
            Index::create(dir, schema.clone())?
        };
        
        let index_reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;
            
        let query_parser = QueryParser::for_index(&index, vec![content_field]);
        
        Ok(Self {
            lancedb_engine,
            tantivy_index: index,
            index_reader,
            query_parser,
            path_field,
            content_field,
            fusion_weight: 0.5, // Equal weight for keyword and semantic
        })
    }
    
    /// Index a code chunk in both engines
    pub async fn index_chunk(
        &self,
        path: &str,
        content: &str,
        language: &str,
        start_line: u32,
        end_line: u32,
    ) -> Result<()> {
        // Index in LanceDB (semantic)
        self.lancedb_engine
            .index_chunk(path, content, language, start_line, end_line)
            .await?;
        
        // Index in Tantivy (keyword)
        let mut index_writer = self.tantivy_index.writer(50_000_000)?;
        let doc = doc!(
            self.path_field => path,
            self.content_field => content,
        );
        index_writer.add_document(doc)?;
        index_writer.commit()?;
        
        debug!("Indexed in both engines: {}:{}-{}", path, start_line, end_line);
        Ok(())
    }
    
    /// Search using both keyword and semantic, then fuse results
    pub async fn hybrid_search(&self, query: &str, limit: usize) -> Result<Vec<HybridResult>> {
        // Run both searches in parallel
        let semantic_future = self.lancedb_engine.search(query, limit * 2);
        let keyword_results = self.keyword_search(query, limit * 2)?;
        
        let semantic_results = semantic_future.await?;
        
        // Apply Reciprocal Rank Fusion
        let fused_results = self.reciprocal_rank_fusion(
            semantic_results,
            keyword_results,
            limit,
        );
        
        Ok(fused_results)
    }
    
    /// Keyword search using Tantivy
    fn keyword_search(&self, query: &str, limit: usize) -> Result<Vec<KeywordResult>> {
        let searcher = self.index_reader.searcher();
        let parsed_query = self.query_parser.parse_query(query)?;
        
        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            
            if let Some(path) = retrieved_doc.get_first(self.path_field) {
                if let Some(content) = retrieved_doc.get_first(self.content_field) {
                    results.push(KeywordResult {
                        path: path.as_text().unwrap_or("").to_string(),
                        content: content.as_text().unwrap_or("").to_string(),
                        score: _score,
                    });
                }
            }
        }
        
        Ok(results)
    }
    
    /// Reciprocal Rank Fusion algorithm
    fn reciprocal_rank_fusion(
        &self,
        semantic: Vec<SearchResult>,
        keyword: Vec<KeywordResult>,
        limit: usize,
    ) -> Vec<HybridResult> {
        let mut scores: HashMap<String, f32> = HashMap::new();
        let mut results_map: HashMap<String, HybridResult> = HashMap::new();
        
        const K: f32 = 60.0; // RRF constant
        
        // Score semantic results
        for (rank, result) in semantic.iter().enumerate() {
            let score = self.fusion_weight / (K + rank as f32 + 1.0);
            let key = format!("{}:{}", result.path, result.content.len());
            
            scores.entry(key.clone())
                .and_modify(|s| *s += score)
                .or_insert(score);
                
            results_map.entry(key)
                .or_insert(HybridResult {
                    path: result.path.clone(),
                    content: result.content.clone(),
                    semantic_score: result.score,
                    keyword_score: 0.0,
                    fused_score: 0.0,
                });
        }
        
        // Score keyword results
        for (rank, result) in keyword.iter().enumerate() {
            let score = (1.0 - self.fusion_weight) / (K + rank as f32 + 1.0);
            let key = format!("{}:{}", result.path, result.content.len());
            
            scores.entry(key.clone())
                .and_modify(|s| *s += score)
                .or_insert(score);
                
            results_map.entry(key)
                .and_modify(|r| r.keyword_score = result.score)
                .or_insert(HybridResult {
                    path: result.path.clone(),
                    content: result.content.clone(),
                    semantic_score: 0.0,
                    keyword_score: result.score,
                    fused_score: 0.0,
                });
        }
        
        // Update fused scores and sort
        let mut fused: Vec<HybridResult> = results_map
            .into_iter()
            .map(|(key, mut result)| {
                result.fused_score = scores.get(&key).copied().unwrap_or(0.0);
                result
            })
            .collect();
            
        fused.sort_by(|a, b| b.fused_score.partial_cmp(&a.fused_score).unwrap());
        fused.truncate(limit);
        
        fused
    }
}

#[derive(Debug, Clone)]
struct KeywordResult {
    path: String,
    content: String,
    score: f32,
}

#[derive(Debug, Clone)]
pub struct HybridResult {
    pub path: String,
    pub content: String,
    pub semantic_score: f32,
    pub keyword_score: f32,
    pub fused_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_hybrid_search() {
        // Test hybrid search functionality
    }
}
