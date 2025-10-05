/// Amazon Titan Embedding Client for Rust
/// Uses AWS SDK to generate 1024-dimensional embeddings

use anyhow::{Result, anyhow};
use aws_sdk_bedrockruntime::primitives::Blob;
use aws_sdk_bedrockruntime::Client;
use aws_config::{Region, BehaviorVersion};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use std::collections::HashMap;
use std::time::Duration;

pub const TITAN_MODEL: &str = "amazon.titan-embed-text-v2:0";
pub const EMBEDDING_DIM: usize = 1024;
pub const MAX_BATCH_SIZE: usize = 25;  // Titan's limit
pub const MAX_INPUT_LENGTH: usize = 8192;  // Titan's token limit

#[derive(Debug, Serialize)]
struct TitanRequest {
    #[serde(rename = "inputText")]
    input_text: String,
    dimensions: usize,
    normalize: bool,
}

#[derive(Debug, Deserialize)]
struct TitanResponse {
    embedding: Vec<f32>,
}

/// Titan embedding client with caching and rate limiting
pub struct TitanEmbeddingClient {
    client: Arc<Client>,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    rate_limiter: Arc<Semaphore>,
    stats: Arc<RwLock<EmbeddingStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct EmbeddingStats {
    pub api_calls: u64,
    pub cache_hits: u64,
    pub total_embeddings: u64,
    pub errors: u64,
}

impl TitanEmbeddingClient {
    /// Initialize with AWS credentials from environment
    pub async fn new() -> Result<Self> {
        // AWS SDK temporarily disabled
        // let config = aws_config::defaults(BehaviorVersion::latest())
        //     .region(Region::new("us-east-1"))
        //     .load()
        //     .await;
        // let client = Client::new(&config);
        
        return Err(anyhow::anyhow!("AWS temporarily disabled"));
    }
    
    /// Initialize with explicit credentials
    pub async fn from_credentials(
        access_key: String,
        secret_key: String,
        region: Option<String>,
    ) -> Result<Self> {
        // AWS SDK temporarily disabled
        // let config = aws_config::defaults(BehaviorVersion::latest())
        //     .region(Region::new("us-east-1"))
        //     .load()
        //     .await;
        // let client = Client::new(&config);
        
        use aws_credential_types::Credentials;
        use aws_credential_types::provider::SharedCredentialsProvider;
        
        let creds = Credentials::new(
            access_key,
            secret_key,
            None,
            None,
            "titan_embedding_client"
        );
        
        // let config = aws_config::defaults(BehaviorVersion::latest())
        //     .region(Region::new(region))
        //     .credentials_provider(SharedCredentialsProvider::new(creds))
        //     .load()
        //     .await;
        return Err(anyhow::anyhow!("AWS temporarily disabled"));
    }
    
    /// Generate embedding for single text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache
        let cache_key = self.compute_cache_key(text);
        {
            let cache = self.cache.read().await;
            if let Some(embedding) = cache.get(&cache_key) {
                self.stats.write().await.cache_hits += 1;
                return Ok(embedding.clone());
            }
        }
        
        // Truncate if too long
        let text = if text.len() > MAX_INPUT_LENGTH {
            &text[..MAX_INPUT_LENGTH]
        } else {
            text
        };
        
        // Rate limiting
        let _permit = self.rate_limiter.acquire().await?;
        
        // Prepare request
        let request = TitanRequest {
            input_text: text.to_string(),
            dimensions: EMBEDDING_DIM,
            normalize: true,
        };
        
        let body = serde_json::to_string(&request)?;
        
        // Call Bedrock
        let response = self.client
            .invoke_model()
            .model_id(TITAN_MODEL)
            .content_type("application/json")
            .accept("application/json")
            .body(Blob::new(body.as_bytes()))
            .send()
            .await
            .map_err(|e| anyhow!("Bedrock API error: {}", e))?;
        
        // Parse response
        let response_body = response.body().as_ref();
        let titan_response: TitanResponse = serde_json::from_slice(response_body)?;
        
        // Update cache and stats
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, titan_response.embedding.clone());
        }
        
        {
            let mut stats = self.stats.write().await;
            stats.api_calls += 1;
            stats.total_embeddings += 1;
        }
        
        Ok(titan_response.embedding)
    }
    
    /// Batch embedding for multiple texts
    pub async fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        
        // Process in chunks of MAX_BATCH_SIZE
        for chunk in texts.chunks(MAX_BATCH_SIZE) {
            let mut chunk_results = Vec::new();
            
            // Process each text in parallel
            let futures: Vec<_> = chunk.iter()
                .map(|text| self.embed(text))
                .collect();
            
            for future in futures {
                match future.await {
                    Ok(embedding) => chunk_results.push(embedding),
                    Err(e) => {
                        self.stats.write().await.errors += 1;
                        return Err(e);
                    }
                }
            }
            
            results.extend(chunk_results);
            
            // Small delay between batches to avoid rate limiting
            if texts.len() > MAX_BATCH_SIZE {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        
        Ok(results)
    }
    
    fn compute_cache_key(&self, text: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Get statistics
    pub async fn get_stats(&self) -> EmbeddingStats {
        self.stats.read().await.clone()
    }
    
    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }
}

/// Embedding model implementation for LanceDB integration
pub struct TitanEmbeddingModel {
    client: Arc<TitanEmbeddingClient>,
}

impl TitanEmbeddingModel {
    pub async fn new() -> Result<Self> {
        let client = TitanEmbeddingClient::new().await?;
        Ok(Self {
            client: Arc::new(client),
        })
    }
    
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        self.client.embed(text).await
    }
    
    pub async fn embed_texts(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        self.client.embed_batch(texts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cache_key() {
        let client = TitanEmbeddingClient::new().await.unwrap();
        let key1 = client.compute_cache_key("test");
        let key2 = client.compute_cache_key("test");
        let key3 = client.compute_cache_key("different");
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
