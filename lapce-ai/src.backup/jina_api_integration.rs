/// Real Jina Embeddings API Integration
/// Uses your provided endpoint for 768-dim embeddings

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::Duration;

pub const API_URL: &str = "https://hierarchy-trigger-bulk-coding.trycloudflare.com/v1/embeddings";
pub const EMBEDDING_DIM: usize = 768;

#[derive(Serialize)]
struct JinaRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct JinaResponse {
    data: Vec<EmbeddingData>,
    usage: Usage,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u32,
    total_tokens: u32,
}

pub struct JinaEmbedding {
    client: Client,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl JinaEmbedding {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache
        if let Some(cached) = self.cache.read().await.get(text) {
            return Ok(cached.clone());
        }
        
        let request = JinaRequest {
            model: "jinaai/jina-embeddings-v2-base-code".to_string(),
            input: vec![text.to_string()],
        };
        
        let response = self.client
            .post(API_URL)
            .json(&request)
            .send()
            .await?
            .json::<JinaResponse>()
            .await?;
        
        let embedding = response.data[0].embedding.clone();
        
        // Cache it
        self.cache.write().await.insert(text.to_string(), embedding.clone());
        
        Ok(embedding)
    }
    
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let request = JinaRequest {
            model: "jinaai/jina-embeddings-v2-base-code".to_string(),
            input: texts.clone(),
        };
        
        let response = self.client
            .post(API_URL)
            .json(&request)
            .send()
            .await?
            .json::<JinaResponse>()
            .await?;
        
        let mut embeddings = vec![vec![]; response.data.len()];
        for item in response.data {
            embeddings[item.index] = item.embedding;
        }
        
        // Cache them
        let mut cache = self.cache.write().await;
        for (text, emb) in texts.iter().zip(embeddings.iter()) {
            cache.insert(text.clone(), emb.clone());
        }
        
        Ok(embeddings)
    }
}

/// Production-ready semantic search with real API
pub struct SemanticSearch {
    embeddings: Arc<JinaEmbedding>,
    documents: Arc<RwLock<Vec<Document>>>,
}

pub struct Document {
    pub id: String,
    pub path: String,
    pub content: String,
    pub embedding: Vec<f32>,
}

pub struct SearchResult {
    pub path: String,
    pub content: String,
    pub score: f32,
}

impl SemanticSearch {
    pub fn new() -> Self {
        Self {
            embeddings: Arc::new(JinaEmbedding::new()),
            documents: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn index(&self, path: &str, content: &str) -> Result<()> {
        let embedding = self.embeddings.embed(content).await?;
        
        let doc = Document {
            id: uuid::Uuid::new_v4().to_string(),
            path: path.to_string(),
            content: content.to_string(),
            embedding,
        };
        
        self.documents.write().await.push(doc);
        Ok(())
    }
    
    pub async fn index_batch(&self, items: Vec<(String, String)>) -> Result<()> {
        let contents: Vec<String> = items.iter().map(|(_, c)| c.clone()).collect();
        let embeddings = self.embeddings.embed_batch(contents).await?;
        
        let mut docs = self.documents.write().await;
        for ((path, content), embedding) in items.iter().zip(embeddings.iter()) {
            docs.push(Document {
                id: uuid::Uuid::new_v4().to_string(),
                path: path.clone(),
                content: content.clone(),
                embedding: embedding.clone(),
            });
        }
        
        Ok(())
    }
    
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query_embedding = self.embeddings.embed(query).await?;
        let docs = self.documents.read().await;
        
        let mut results: Vec<SearchResult> = docs
            .iter()
            .map(|doc| {
                let score = cosine_similarity(&query_embedding, &doc.embedding);
                SearchResult {
                    path: doc.path.clone(),
                    content: doc.content.clone(),
                    score,
                }
            })
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);
        
        Ok(results)
    }
    
    pub async fn stats(&self) -> (usize, usize) {
        let docs = self.documents.read().await;
        let cache_size = self.embeddings.cache.read().await.len();
        (docs.len(), cache_size)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_jina_api() {
        let jina = JinaEmbedding::new();
        let embedding = jina.embed("test code").await.unwrap();
        assert_eq!(embedding.len(), EMBEDDING_DIM);
    }
    
    #[tokio::test]
    async fn test_semantic_search() {
        let search = SemanticSearch::new();
        
        // Index some code
        search.index("test.rs", "async fn main() {}").await.unwrap();
        search.index("lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }").await.unwrap();
        
        // Search
        let results = search.search("async function", 2).await.unwrap();
        assert!(!results.is_empty());
        assert!(results[0].score > 0.0);
    }
}
