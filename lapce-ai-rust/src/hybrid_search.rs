/// Hybrid Search Implementation - Semantic + Keyword with Reciprocal Rank Fusion
/// Following docs/06-SEMANTIC-SEARCH-LANCEDB.md specification

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;

use tantivy::{
    schema::{Schema as TantivySchema, TEXT, STORED, Field as TantivyField},
    Index as TantivyIndex,
    IndexWriter,
    Document,
    collector::TopDocs,
    query::QueryParser,
    IndexReader,
};

use crate::lancedb_semantic_search::{SemanticSearchEngine, SearchResult, SearchFilters};

/// Hybrid search combining semantic and keyword search
pub struct HybridSearcher {
    semantic_engine: Arc<SemanticSearchEngine>,
    keyword_index: Arc<TantivyIndex>,
    index_writer: Arc<tokio::sync::Mutex<IndexWriter>>,
    reader: IndexReader,
    fusion_weight: f32,
    
    // Schema fields
    path_field: TantivyField,
    content_field: TantivyField,
    id_field: TantivyField,
}

impl HybridSearcher {
    pub fn new(semantic_engine: Arc<SemanticSearchEngine>) -> Result<Self> {
        // Create Tantivy schema
        let mut schema_builder = TantivySchema::builder();
        let path_field = schema_builder.add_text_field("path", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let id_field = schema_builder.add_text_field("id", STORED);
        let schema = schema_builder.build();
        
        // Create index
        let index = TantivyIndex::create_in_ram(schema);
        let index_writer = index.writer(50_000_000)?; // 50MB buffer
        let reader = index.reader()?;
        
        Ok(Self {
            semantic_engine,
            keyword_index: Arc::new(index),
            index_writer: Arc::new(tokio::sync::Mutex::new(index_writer)),
            reader,
            fusion_weight: 0.7, // 70% semantic, 30% keyword
            path_field,
            content_field,
            id_field,
        })
    }
    
    /// Add document to keyword index
    pub async fn add_document(&self, id: &str, path: &str, content: &str) -> Result<()> {
        let mut doc = Document::default();
        doc.add_text(self.id_field, id);
        doc.add_text(self.path_field, path);
        doc.add_text(self.content_field, content);
        
        let mut writer = self.index_writer.lock().await;
        writer.add_document(doc)?;
        
        Ok(())
    }
    
    /// Commit changes to keyword index
    pub async fn commit(&self) -> Result<()> {
        let mut writer = self.index_writer.lock().await;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }
    
    /// Perform hybrid search with Reciprocal Rank Fusion
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Run both searches in parallel
        let (semantic_results, keyword_results) = tokio::join!(
            self.semantic_engine.search(query, limit * 2, None),
            self.search_keyword(query, limit * 2)
        );
        
        let semantic_results = semantic_results?;
        let keyword_results = keyword_results?;
        
        // Apply Reciprocal Rank Fusion
        self.reciprocal_rank_fusion(semantic_results, keyword_results, limit)
    }
    
    /// Search with filters
    pub async fn search_with_filters(
        &self, 
        query: &str, 
        limit: usize,
        filters: SearchFilters
    ) -> Result<Vec<SearchResult>> {
        let (semantic_results, keyword_results) = tokio::join!(
            self.semantic_engine.search(query, limit * 2, Some(filters.clone())),
            self.search_keyword_with_filters(query, limit * 2, filters)
        );
        
        let semantic_results = semantic_results?;
        let keyword_results = keyword_results?;
        
        self.reciprocal_rank_fusion(semantic_results, keyword_results, limit)
    }
    
    /// Keyword search using Tantivy
    async fn search_keyword(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.keyword_index, 
            vec![self.content_field, self.path_field]
        );
        
        let query = query_parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            
            let id = doc.get_first(self.id_field)
                .and_then(|f| f.as_text())
                .unwrap_or("").to_string();
                
            let path = doc.get_first(self.path_field)
                .and_then(|f| f.as_text())
                .unwrap_or("").to_string();
                
            let content = doc.get_first(self.content_field)
                .and_then(|f| f.as_text())
                .unwrap_or("").to_string();
            
            results.push(SearchResult {
                id,
                path: PathBuf::from(path),
                content,
                score,
                language: None,
                start_line: 0,
                end_line: 0,
                metadata: None,
            });
        }
        
        Ok(results)
    }
    
    /// Keyword search with filters
    async fn search_keyword_with_filters(
        &self, 
        query: &str, 
        limit: usize,
        filters: SearchFilters
    ) -> Result<Vec<SearchResult>> {
        // For now, just do regular keyword search
        // In production, would apply filters to Tantivy query
        self.search_keyword(query, limit).await
    }
    
    /// Reciprocal Rank Fusion algorithm
    fn reciprocal_rank_fusion(
        &self,
        semantic: Vec<SearchResult>,
        keyword: Vec<SearchResult>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut scores: HashMap<String, f32> = HashMap::new();
        let mut result_map: HashMap<String, SearchResult> = HashMap::new();
        let k = 60.0; // RRF constant from the spec
        
        // Score semantic results with weight
        for (rank, result) in semantic.iter().enumerate() {
            let score = self.fusion_weight / (k + rank as f32 + 1.0);
            scores.entry(result.id.clone())
                .and_modify(|s| *s += score)
                .or_insert(score);
            result_map.insert(result.id.clone(), result.clone());
        }
        
        // Score keyword results with (1 - weight)
        for (rank, result) in keyword.iter().enumerate() {
            let score = (1.0 - self.fusion_weight) / (k + rank as f32 + 1.0);
            scores.entry(result.id.clone())
                .and_modify(|s| *s += score)
                .or_insert(score);
            result_map.entry(result.id.clone())
                .or_insert_with(|| result.clone());
        }
        
        // Sort by fused score
        let mut fused: Vec<(String, f32)> = scores.into_iter().collect();
        fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return top results with updated scores
        Ok(fused.into_iter()
            .take(limit)
            .filter_map(|(id, fused_score)| {
                result_map.get(&id).map(|result| {
                    let mut updated = result.clone();
                    updated.score = fused_score;
                    updated
                })
            })
            .collect())
    }
    
    /// Set fusion weight (0.0 = keyword only, 1.0 = semantic only)
    pub fn set_fusion_weight(&mut self, weight: f32) {
        self.fusion_weight = weight.clamp(0.0, 1.0);
    }
    
    /// Get current fusion weight
    pub fn get_fusion_weight(&self) -> f32 {
        self.fusion_weight
    }
}
