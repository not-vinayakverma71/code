/// Embedding API Integration - Using proprietary APIs instead of local models
/// This eliminates the 450MB+ memory overhead of loading BERT/MiniLM locally

use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Semaphore;
use std::time::Duration;

/// Generic embedding provider trait
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn embed_single(&self, text: &str) -> Result<Vec<f32>>;
    fn dimensions(&self) -> usize;
    fn name(&self) -> &str;
    fn max_batch_size(&self) -> usize;
}

/// OpenAI Embedding Provider - Best for general purpose
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
    rate_limiter: Arc<Semaphore>,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "text-embedding-3-small".to_string(), // 1536 dimensions, $0.02 per 1M tokens
            rate_limiter: Arc::new(Semaphore::new(50)), // 3000 rpm = 50 rps
        }
    }
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    input: Vec<String>,
    encoding_format: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    data: Vec<OpenAIEmbedding>,
    usage: OpenAIUsage,
}

#[derive(Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

#[async_trait]
impl EmbeddingProvider for OpenAIProvider {
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let _permit = self.rate_limiter.acquire().await?;
        
        let request = OpenAIRequest {
            model: self.model.clone(),
            input: texts,
            encoding_format: "float".to_string(),
        };
        
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .timeout(Duration::from_secs(30))
            .send()
            .await?
            .json::<OpenAIResponse>()
            .await?;
        
        let mut embeddings = vec![vec![]; response.data.len()];
        for item in response.data {
            embeddings[item.index] = item.embedding;
        }
        
        Ok(embeddings)
    }
    
    async fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed_batch(vec![text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap())
    }
    
    fn dimensions(&self) -> usize {
        1536 // text-embedding-ada-002 dimensions
    }
    
    fn name(&self) -> &str {
        "openai"
    }
    
    fn max_batch_size(&self) -> usize {
        100
    }
}

/// Cohere Embedding Provider - Best for multilingual
pub struct CohereProvider {
    client: Client,
    api_key: String,
    model: String,
    rate_limiter: Arc<Semaphore>,
}

impl CohereProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "embed-english-v3.0".to_string(), // 4096 dimensions
            rate_limiter: Arc::new(Semaphore::new(100)), // Higher rate limit
        }
    }
}

#[derive(Serialize)]
struct CohereRequest {
    texts: Vec<String>,
    model: String,
    input_type: String,
    truncate: String,
}

#[derive(Deserialize)]
struct CohereResponse {
    embeddings: Vec<Vec<f32>>,
}

#[async_trait]
impl EmbeddingProvider for CohereProvider {
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let _permit = self.rate_limiter.acquire().await?;
        
        let request = CohereRequest {
            texts,
            model: self.model.clone(),
            input_type: "search_document".to_string(),
            truncate: "END".to_string(),
        };
        
        let response = self.client
            .post("https://api.cohere.ai/v1/embed")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .timeout(Duration::from_secs(30))
            .send()
            .await?
            .json::<CohereResponse>()
            .await?;
        
        Ok(response.embeddings)
    }
    
    async fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed_batch(vec![text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap())
    }
    
    fn dimensions(&self) -> usize {
        4096 // embed-english-v3.0 dimensions
    }
    
    fn name(&self) -> &str {
        "cohere"
    }
    
    fn max_batch_size(&self) -> usize {
        96
    }
}

/// Voyage AI Provider - Specialized for code
pub struct VoyageProvider {
    client: Client,
    api_key: String,
    model: String,
    rate_limiter: Arc<Semaphore>,
}

impl VoyageProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "voyage-2".to_string(), // 1024 dimensions, optimized for code
            rate_limiter: Arc::new(Semaphore::new(20)),
        }
    }
}

#[derive(Serialize)]
struct VoyageRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct VoyageResponse {
    data: Vec<VoyageEmbedding>,
}

#[derive(Deserialize)]
struct VoyageEmbedding {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingProvider for VoyageProvider {
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let _permit = self.rate_limiter.acquire().await?;
        
        let request = VoyageRequest {
            input: texts,
            model: self.model.clone(),
        };
        
        let response = self.client
            .post("https://api.voyageai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .timeout(Duration::from_secs(30))
            .send()
            .await?
            .json::<VoyageResponse>()
            .await?;
        
        Ok(response.data.into_iter().map(|e| e.embedding).collect())
    }
    
    async fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed_batch(vec![text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap())
    }
    
    fn dimensions(&self) -> usize {
        1024 // voyage-2 dimensions
    }
    
    fn name(&self) -> &str {
        "voyage"
    }
    
    fn max_batch_size(&self) -> usize {
        128
    }
}

/// Smart provider selector with fallback
pub struct EmbeddingService {
    primary: Box<dyn EmbeddingProvider>,
    fallback: Option<Box<dyn EmbeddingProvider>>,
    cache: Arc<tokio::sync::RwLock<lru::LruCache<String, Vec<f32>>>>,
}

impl EmbeddingService {
    pub fn new(primary: Box<dyn EmbeddingProvider>) -> Self {
        Self {
            primary,
            fallback: None,
            cache: Arc::new(tokio::sync::RwLock::new(lru::LruCache::new(std::num::NonZeroUsize::new(10000).unwrap()))),
        }
    }
    
    pub fn with_fallback(mut self, fallback: Box<dyn EmbeddingProvider>) -> Self {
        self.fallback = Some(fallback);
        self
    }
    
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        let cache_key = format!("{}-{}", self.primary.name(), text);
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.peek(&cache_key) {
                return Ok(cached.clone());
            }
        }
        
        // Try primary provider
        let result = match self.primary.embed_single(text).await {
            Ok(embedding) => Ok(embedding),
            Err(e) => {
                // Try fallback if available
                if let Some(fallback) = &self.fallback {
                    eprintln!("Primary provider failed: {}, trying fallback", e);
                    fallback.embed_single(text).await
                } else {
                    Err(e)
                }
            }
        };
        
        // Cache successful result
        if let Ok(ref embedding) = result {
            let mut cache = self.cache.write().await;
            cache.push(cache_key, embedding.clone());
        }
        
        result
    }
    
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        // Split into chunks based on provider's max batch size
        let max_batch = self.primary.max_batch_size();
        let mut all_embeddings = Vec::with_capacity(texts.len());
        
        for chunk in texts.chunks(max_batch) {
            let batch_result = match self.primary.embed_batch(chunk.to_vec()).await {
                Ok(embeddings) => Ok(embeddings),
                Err(e) => {
                    if let Some(fallback) = &self.fallback {
                        eprintln!("Primary provider batch failed: {}, trying fallback", e);
                        fallback.embed_batch(chunk.to_vec()).await
                    } else {
                        Err(e)
                    }
                }
            };
            
            all_embeddings.extend(batch_result?);
        }
        
        Ok(all_embeddings)
    }
}

/// Cost tracking
pub struct EmbeddingCostTracker {
    openai_tokens: u64,
    cohere_calls: u64,
    voyage_calls: u64,
}

impl EmbeddingCostTracker {
    pub fn estimate_monthly_cost(&self) -> f64 {
        let openai_cost = (self.openai_tokens as f64 / 1_000_000.0) * 0.02; // $0.02 per 1M tokens
        let cohere_cost = (self.cohere_calls as f64 / 1000.0) * 0.1; // $0.1 per 1000 calls
        let voyage_cost = (self.voyage_calls as f64 / 1000.0) * 0.05; // $0.05 per 1000 calls
        
        openai_cost + cohere_cost + voyage_cost
    }
    
    pub fn log_usage(&mut self, provider: &str, count: u64) {
        match provider {
            "openai" => self.openai_tokens += count,
            "cohere" => self.cohere_calls += count,
            "voyage" => self.voyage_calls += count,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_embedding_service() {
        // This would need actual API keys to run
        let provider = Box::new(OpenAIProvider::new("test_key".to_string()));
        let service = EmbeddingService::new(provider);
        
        // Test dimensions
        assert_eq!(service.primary.dimensions(), 1536);
        assert_eq!(service.primary.max_batch_size(), 2048);
    }
}
