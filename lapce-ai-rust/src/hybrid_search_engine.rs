/// Hybrid Search (Keyword + Semantic) with Reciprocal Rank Fusion
/// Implements exact algorithm from docs/06-SEMANTIC-SEARCH-LANCEDB.md lines 367-428

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::{
    schema::{Schema, Field, STORED, TEXT, STRING},
    Index, IndexWriter, Document,
    query::QueryParser,
    collector::TopDocs,
};
use tokio::sync::RwLock;
// use crate::lancedb_integration::{SemanticSearchEngine, SearchResult, SearchFilters};

// Use placeholder types from concurrent_handler
use crate::concurrent_handler::{SemanticSearchEngine, SearchResult, SearchFilters};

/// Tantivy keyword index implementation
pub struct TantivyIndex {
    index: Index,
    writer: Arc<RwLock<IndexWriter>>,
    path_field: Field,
    content_field: Field,
    language_field: Field,
}

impl TantivyIndex {
    pub fn new(index_path: &str) -> Result<Self> {
        // Build schema
        let mut schema_builder = Schema::builder();
        let path_field = schema_builder.add_text_field("path", STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let language_field = schema_builder.add_text_field("language", STRING | STORED);
        let schema = schema_builder.build();
        
        // Create index
        let index = Index::create_in_dir(index_path, schema.clone())?;
        let writer = index.writer(50_000_000)?; // 50MB buffer
        
        Ok(Self {
            index,
            writer: Arc::new(RwLock::new(writer)),
            path_field,
            content_field,
            language_field,
        })
    }
    
    /// Add document to keyword index
    pub async fn add_document(&self, path: &str, content: &str, language: &str) -> Result<()> {
        let mut doc = Document::new();
        doc.add_text(self.path_field, path);
        doc.add_text(self.content_field, content);
        doc.add_text(self.language_field, language);
        
        let mut writer = self.writer.write().await;
        writer.add_document(doc)?;
        
        Ok(())
    }
    
    /// Search using keyword query
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<KeywordResult>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        
        let query_parser = QueryParser::for_index(&self.index, vec![self.content_field]);
        let query = query_parser.parse_query(query)?;
        
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            
            let path = retrieved_doc
                .get_first(self.path_field)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            
            let content = retrieved_doc
                .get_first(self.content_field)
                .and_then(|v| v.as_text())
                .unwrap_or("")
                .to_string();
            
            results.push(KeywordResult {
                id: format!("{:?}", doc_address),
                path,
                content,
                score: _score,
            });
        }
        
        Ok(results)
    }
    
    /// Commit changes to index
    pub async fn commit(&self) -> Result<()> {
        let mut writer = self.writer.write().await;
        writer.commit()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct KeywordResult {
    pub id: String,
    pub path: String,
    pub content: String,
    pub score: f32,
}

/// Hybrid searcher combining semantic and keyword search
/// Exact implementation from lines 367-428
pub struct HybridSearcher {
    semantic_engine: Arc<SemanticSearchEngine>,
    keyword_index: Arc<TantivyIndex>,
    fusion_weight: f32,
}

impl HybridSearcher {
    pub fn new(
        semantic_engine: Arc<SemanticSearchEngine>,
        keyword_index: Arc<TantivyIndex>,
        fusion_weight: f32,
    ) -> Self {
        Self {
            semantic_engine,
            keyword_index,
            fusion_weight,
        }
    }
    
    /// Hybrid search with parallel execution (lines 374-386)
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Run both searches in parallel
        let (semantic_results, keyword_results) = tokio::join!(
            self.semantic_engine.search(query, limit * 2, None),
            self.keyword_index.search(query, limit * 2)
        );
        
        let semantic_results = semantic_results?;
        let keyword_results = keyword_results?;
        
        // Reciprocal Rank Fusion
        self.fuse_results(semantic_results, keyword_results, limit)
    }
    
    /// Reciprocal Rank Fusion implementation (lines 388-427)
    fn fuse_results(
        &self,
        semantic: Vec<SearchResult>,
        keyword: Vec<KeywordResult>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut scores = HashMap::new();
        let k = 60.0; // RRF constant
        
        // Score semantic results (lines 397-403)
        for (rank, result) in semantic.iter().enumerate() {
            let score = self.fusion_weight / (k + rank as f32 + 1.0);
            scores.entry(&result.id)
                .and_modify(|s| *s += score)
                .or_insert(score);
        }
        
        // Convert keyword results to same format and score (lines 405-411)
        let keyword_as_search: Vec<SearchResult> = keyword.iter().map(|kr| {
            SearchResult {
                id: kr.id.clone(),
                path: kr.path.clone(),
                content: kr.content.clone(),
                score: kr.score,
                start_line: 0,
                end_line: 0,
            }
        }).collect();
        
        for (rank, result) in keyword_as_search.iter().enumerate() {
            let score = (1.0 - self.fusion_weight) / (k + rank as f32 + 1.0);
            scores.entry(&result.id)
                .and_modify(|s| *s += score)
                .or_insert(score);
        }
        
        // Sort by fused score (lines 413-416)
        let mut fused: Vec<_> = scores.into_iter().collect();
        fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Return top results (lines 417-426)
        Ok(fused.into_iter()
            .take(limit)
            .filter_map(|(id, _score)| {
                semantic.iter()
                    .find(|r| &r.id == id)
                    .cloned()
                    .or_else(|| {
                        keyword_as_search.iter()
                            .find(|r| &r.id == id)
                            .cloned()
                    })
            })
            .collect())
    }
}
