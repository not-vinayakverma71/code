// Production-ready AWS Titan Embedder with enterprise features
use crate::error::{Error, Result};
use crate::embeddings::embedder_interface::{IEmbedder, EmbeddingResponse, EmbedderInfo, AvailableEmbedders, EmbeddingUsage};
use crate::embeddings::aws_config_validator::{validate_aws_config, load_env_file};
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_config;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use tracing::{info, warn, error, debug};

/// AWS Tier configuration for rate limiting
#[derive(Debug, Clone, PartialEq)]
pub enum AwsTier {
    Free,
    OnDemand,
    Standard,
    Premium,
    Enterprise,
    Provisioned,
}

impl AwsTier {
    fn max_requests_per_second(&self) -> usize {
        match self {
            AwsTier::Free => 2,
            AwsTier::OnDemand => 5,
            AwsTier::Standard => 10,
            AwsTier::Premium => 50,
            AwsTier::Enterprise => 500,
            AwsTier::Provisioned => 100,
        }
    }
    
    fn max_tokens_per_minute(&self) -> usize {
        match self {
            AwsTier::Free => 8_000,
            AwsTier::OnDemand => 20_000,
            AwsTier::Standard => 40_000,
            AwsTier::Premium => 200_000,
            AwsTier::Enterprise => 2_000_000,
            AwsTier::Provisioned => 500_000,
        }
    }
    
    fn batch_size(&self) -> usize {
        match self {
            AwsTier::Free => 5,
            AwsTier::OnDemand => 8,
            AwsTier::Standard => 10,
            AwsTier::Premium => 25,
            AwsTier::Enterprise => 100,
            AwsTier::Provisioned => 50,
        }
    }
}

/// Cache entry for embeddings
#[derive(Debug, Clone)]
struct CachedEmbedding {
    embedding: Arc<[f32]>,  // Changed to Arc for zero-copy
    text_hash: u64,
    created_at: Instant,
    access_count: usize,
}

/// Usage metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub cached_requests: usize,
    pub total_tokens: usize,
    pub total_cost_usd: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub rate_limit_hits: usize,
    pub retry_count: usize,
}

impl Default for UsageMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            cached_requests: 0,
            total_tokens: 0,
            total_cost_usd: 0.0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            rate_limit_hits: 0,
            retry_count: 0,
        }
    }
}

/// Production AWS Titan Embedder with enterprise features
pub struct AwsTitanProduction {
    client: BedrockClient,
    tier: AwsTier,
    cache: Arc<RwLock<HashMap<u64, CachedEmbedding>>>,
    rate_limiter: Arc<Semaphore>,
    metrics: Arc<RwLock<UsageMetrics>>,
    latencies: Arc<RwLock<Vec<f64>>>,
    cache_ttl: Duration,
    max_cache_size: usize,
    retry_config: RetryConfig,
}

#[derive(Debug, Clone)]
struct RetryConfig {
    max_retries: usize,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    exponential_base: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 100,
            max_delay_ms: 30_000,
            exponential_base: 2.0,
        }
    }
}

impl AwsTitanProduction {
    pub fn new(
        client: BedrockClient,
        tier: AwsTier,
        max_batch_size: usize,
        requests_per_second: f64,
    ) -> Self {
        let rate_limiter = Arc::new(Semaphore::new(
            (requests_per_second as usize).max(1)
        ));
        
        Self {
            client,
            tier,
            cache: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter,
            metrics: Arc::new(RwLock::new(UsageMetrics::default())),
            latencies: Arc::new(RwLock::new(Vec::new())),
            cache_ttl: Duration::from_secs(3600), // 1 hour cache
            max_cache_size: 10_000,
            retry_config: RetryConfig::default(),
        }
    }
    
    pub async fn new_from_config() -> Result<Self> {
        // Load .env file if it exists
        load_env_file();
        
        // Validate AWS configuration with detailed errors
        let aws_requirements = validate_aws_config()?;
        
        let config = aws_config::load_from_env().await;
        let client = BedrockClient::new(&config);
        
        Ok(Self {
            client,
            tier: AwsTier::Standard,
            cache: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(Semaphore::new(aws_requirements.max_batch_size)),
            metrics: Arc::new(RwLock::new(UsageMetrics::default())),
            latencies: Arc::new(RwLock::new(Vec::new())),
            cache_ttl: Duration::from_secs(3600),
            max_cache_size: 10_000,
            retry_config: RetryConfig::default(),
        })
    }
    
    /// Calculate hash for text (for caching)
    fn hash_text(&self, text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Check cache for embedding
    async fn check_cache(&self, text: &str) -> Option<Arc<[f32]>> {
        let hash = self.hash_text(text);
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(&hash) {
            // Check if cache is still valid
            if entry.created_at.elapsed() < self.cache_ttl {
                entry.access_count += 1;
                
                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.cached_requests += 1;
                
                debug!("Cache hit for text hash: {}", hash);
                return Some(entry.embedding.clone());  // Arc clone is cheap
            } else {
                // Remove expired entry
                cache.remove(&hash);
            }
        }
        
        None
    }
    
    /// Update cache with new embedding
    async fn update_cache(&self, text: &str, embedding: Arc<[f32]>) {
        let hash = self.hash_text(text);
        let mut cache = self.cache.write().await;
        
        // Implement LRU eviction if cache is full
        if cache.len() >= self.max_cache_size {
            // Find least recently used entry
            if let Some((&lru_hash, _)) = cache.iter()
                .min_by_key(|(_, entry)| entry.access_count) {
                cache.remove(&lru_hash);
                debug!("Evicted cache entry: {}", lru_hash);
            }
        }
        
        cache.insert(hash, CachedEmbedding {
            embedding,  // Store the Arc directly
            text_hash: hash,
            created_at: Instant::now(),
            access_count: 1,
        });
    }
    
    /// Execute request with exponential backoff retry
    async fn execute_with_retry(&self, text: &str) -> Result<Vec<f32>> {
        let mut retries = 0;
        let mut delay = self.retry_config.initial_delay_ms;
        
        loop {
            match self.execute_single_request(text).await {
                Ok(embedding) => {
                    if retries > 0 {
                        let mut metrics = self.metrics.write().await;
                        metrics.retry_count += retries;
                    }
                    return Ok(embedding);
                }
                Err(e) => {
                    if retries >= self.retry_config.max_retries {
                        error!("Max retries ({}) exceeded for embedding request", self.retry_config.max_retries);
                        return Err(e);
                    }
                    
                    // Check if error is retryable
                    let error_msg = format!("{}", e);
                    let is_rate_limit = error_msg.contains("throttling") || 
                                       error_msg.contains("rate") ||
                                       error_msg.contains("TooManyRequests");
                    
                    if is_rate_limit {
                        let mut metrics = self.metrics.write().await;
                        metrics.rate_limit_hits += 1;
                        
                        warn!("Rate limit hit, retry {} with {}ms delay", retries + 1, delay);
                    } else if error_msg.contains("service error") {
                        warn!("Service error, retry {} with {}ms delay", retries + 1, delay);
                    } else {
                        // Non-retryable error
                        return Err(e);
                    }
                    
                    // Exponential backoff with jitter
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    
                    // Calculate next delay with jitter
                    delay = (delay as f64 * self.retry_config.exponential_base) as u64;
                    delay = delay.min(self.retry_config.max_delay_ms);
                    
                    // Add simple jitter using current time (±25%)
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| Error::Runtime {
                            message: format!("Failed to get system time: {}", e)
                        })?
                        .as_nanos();
                    let jitter_factor = ((now % 100) as f64 / 100.0) * 0.25;
                    let jitter = (delay as f64 * jitter_factor) as u64;
                    delay = delay + jitter - (delay / 8);
                    
                    retries += 1;
                }
            }
        }
    }
    
    /// Execute single embedding request
    async fn execute_single_request(&self, text: &str) -> Result<Vec<f32>> {
        let request_body = serde_json::json!({
            "inputText": text
        });
        
        let start = Instant::now();
        
        let response = self.client
            .invoke_model()
            .model_id("amazon.titan-embed-text-v1")
            .body(aws_sdk_bedrockruntime::primitives::Blob::new(
                serde_json::to_vec(&request_body)?
            ))
            .send()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("AWS Bedrock API error: {}", e)
            })?;
        
        let latency_ms = start.elapsed().as_millis() as f64;
        
        // Update latency metrics
        let mut latencies = self.latencies.write().await;
        latencies.push(latency_ms);
        
        // Keep only last 1000 latencies for percentile calculation
        if latencies.len() > 1000 {
            latencies.drain(0..100);
        }
        
        // Parse response
        let response_json: serde_json::Value = serde_json::from_slice(response.body.as_ref())?;
        
        let embedding = response_json["embedding"]
            .as_array()
            .ok_or_else(|| Error::Runtime {
                message: "Missing embedding in AWS response".to_string()
            })?
            .iter()
            .map(|v| v.as_f64().ok_or_else(|| Error::Runtime {
                message: "Invalid float value in embedding".to_string()
            }).map(|f| f as f32))
            .collect::<std::result::Result<Vec<f32>, Error>>()?
;
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.successful_requests += 1;
        metrics.total_tokens += text.len() / 4; // Rough estimate
        
        // AWS Titan pricing: $0.00002 per 1K tokens
        metrics.total_cost_usd += (text.len() as f64 / 4000.0) * 0.00002;
        
        Ok(embedding)
    }
    
    /// Batch process multiple texts efficiently
    async fn process_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let batch_size = self.tier.batch_size();
        let mut all_embeddings = Vec::new();
        
        // Process in batches
        for chunk in texts.chunks(batch_size) {
            let mut batch_embeddings = Vec::new();
            
            for text in chunk {
                // Check cache first
                if let Some(cached) = self.check_cache(text).await {
                    batch_embeddings.push(cached.to_vec());  // Convert Arc to Vec at the edge
                    continue;
                }
                
                // Acquire rate limit permit
                let _permit = self.rate_limiter.acquire().await
                    .map_err(|_| Error::Runtime {
                        message: "Rate limiter error".to_string()
                    })?;
                
                // Execute with retry
                match self.execute_with_retry(text).await {
                    Ok(embedding) => {
                        // Convert to Arc for cache
                        let arc_embedding: Arc<[f32]> = Arc::from(embedding.clone().into_boxed_slice());
                        self.update_cache(text, arc_embedding).await;
                        batch_embeddings.push(embedding);
                    }
                    Err(e) => {
                        error!("Failed to get embedding after retries: {}", e);
                        let mut metrics = self.metrics.write().await;
                        metrics.failed_requests += 1;
                        return Err(e);
                    }
                }
            }
            
            all_embeddings.extend(batch_embeddings);
            
            // Small delay between batches to avoid bursts
            if chunk.len() == batch_size {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        
        Ok(all_embeddings)
    }
    
    /// Get current usage metrics
    pub async fn get_metrics(&self) -> UsageMetrics {
        let metrics = self.metrics.read().await;
        let latencies = self.latencies.read().await;
        
        let mut result = metrics.clone();
        
        // Calculate percentiles
        if !latencies.is_empty() {
            let mut sorted: Vec<f64> = latencies.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            
            result.avg_latency_ms = sorted.iter().sum::<f64>() / sorted.len() as f64;
            result.p95_latency_ms = sorted[(sorted.len() as f64 * 0.95) as usize];
            result.p99_latency_ms = sorted[(sorted.len() as f64 * 0.99) as usize];
        }
        
        result
    }
    
    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Cache cleared");
    }
    
    /// Export metrics for monitoring
    pub async fn export_metrics(&self) -> String {
        let metrics = self.get_metrics().await;
        serde_json::to_string_pretty(&metrics).unwrap_or_else(|_| "Failed to serialize metrics".to_string())
    }
}

#[async_trait]
impl IEmbedder for AwsTitanProduction {
    async fn create_embeddings(&self, texts: Vec<String>, _model: Option<&str>) -> Result<EmbeddingResponse> {
        let start = Instant::now();
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_requests += texts.len();
        }
        
        info!("Processing {} texts with AWS Titan", texts.len());
        
        // Process with batching and caching
        let embeddings = self.process_batch(texts).await?;
        
        info!("Completed in {:?}", start.elapsed());
        
        Ok(EmbeddingResponse {
            embeddings,
            usage: None,
        })
    }
    
    async fn validate_configuration(&self) -> Result<(bool, Option<String>)> {
        match self.execute_with_retry("test").await {
            Ok(_) => {
                let metrics = self.get_metrics().await;
                Ok((true, Some(format!(
                    "AWS Bedrock Titan configured (Tier: {:?}, Cache: {} entries)",
                    self.tier,
                    self.cache.read().await.len()
                ))))
            }
            Err(e) => Ok((false, Some(format!("AWS Bedrock error: {}", e))))
        }
    }
    
    fn embedder_info(&self) -> EmbedderInfo {
        EmbedderInfo {
            name: AvailableEmbedders::AwsBedrock,
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Remove rand import - we'll use a simple alternative
