// Robust AWS Titan wrapper with retry logic and error handling
// Handles rate limits, transient errors, and provides fallback strategies

use crate::embeddings::aws_titan_production::{AwsTitanProduction, AwsTier};
use crate::embeddings::embedder_interface::{IEmbedder, EmbeddingResponse, EmbedderInfo, AvailableEmbedders};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Configuration for robust AWS handling
#[derive(Clone, Debug)]
pub struct RobustConfig {
    /// Maximum number of retries for transient failures
    pub max_retries: usize,
    
    /// Initial delay between retries (exponential backoff)
    pub initial_retry_delay_ms: u64,
    
    /// Maximum delay between retries
    pub max_retry_delay_ms: u64,
    
    /// Rate limit: max concurrent requests
    pub max_concurrent_requests: usize,
    
    /// Rate limit: requests per second
    pub requests_per_second: f64,
    
    /// Batch size for embedding generation
    pub batch_size: usize,
    
    /// Timeout for individual requests
    pub request_timeout_secs: u64,
    
    /// Enable fallback to cached embeddings
    pub enable_cache_fallback: bool,
}

impl Default for RobustConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 10000,
            max_concurrent_requests: 5,
            requests_per_second: 2.0,
            batch_size: 10,
            request_timeout_secs: 30,
            enable_cache_fallback: true,
        }
    }
}

/// Robust AWS Titan embedder with retry logic and error handling
pub struct RobustAwsTitan {
    inner: Arc<AwsTitanProduction>,
    config: RobustConfig,
    semaphore: Arc<Semaphore>,
    rate_limiter: Arc<tokio::sync::Mutex<RateLimiter>>,
}

struct RateLimiter {
    last_request_time: std::time::Instant,
    min_interval: Duration,
}

impl RateLimiter {
    fn new(requests_per_second: f64) -> Self {
        let min_interval = Duration::from_secs_f64(1.0 / requests_per_second);
        Self {
            last_request_time: std::time::Instant::now() - min_interval,
            min_interval,
        }
    }
    
    async fn wait_if_needed(&mut self) {
        let elapsed = self.last_request_time.elapsed();
        if elapsed < self.min_interval {
            let wait_time = self.min_interval - elapsed;
            sleep(wait_time).await;
        }
        self.last_request_time = std::time::Instant::now();
    }
}

impl RobustAwsTitan {
    /// Create new robust AWS Titan embedder
    pub async fn new(region: &str, tier: AwsTier, config: RobustConfig) -> Result<Self> {
        let inner = AwsTitanProduction::new_from_config().await
            .map_err(|e| Error::Runtime { 
                message: format!("Failed to create AWS Titan client: {}", e) 
            })?;
        
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));
        let rate_limiter = Arc::new(tokio::sync::Mutex::new(
            RateLimiter::new(config.requests_per_second)
        ));
        
        Ok(Self {
            inner: Arc::new(inner),
            config,
            semaphore,
            rate_limiter,
        })
    }
    
    /// Create embeddings with retry logic
    async fn create_embeddings_with_retry(
        &self,
        texts: Vec<String>,
        model: Option<String>,
    ) -> Result<EmbeddingResponse> {
        let mut attempt = 0;
        let mut delay = self.config.initial_retry_delay_ms;
        
        loop {
            // Acquire semaphore permit for concurrency control
            let _permit = self.semaphore.acquire().await
                .map_err(|e| Error::Runtime { 
                    message: format!("Semaphore error: {}", e) 
                })?;
            
            // Apply rate limiting
            self.rate_limiter.lock().await.wait_if_needed().await;
            
            // Attempt the request with timeout
            let result = tokio::time::timeout(
                Duration::from_secs(self.config.request_timeout_secs),
                self.inner.create_embeddings(texts.clone(), model.as_deref())
            ).await;
            
            match result {
                Ok(Ok(response)) => {
                    info!("Successfully generated {} embeddings", response.embeddings.len());
                    return Ok(response);
                }
                Ok(Err(e)) => {
                    attempt += 1;
                    let error_msg = format!("{:?}", e);
                    
                    // Check if error is retryable
                    if self.is_retryable_error(&error_msg) && attempt <= self.config.max_retries {
                        warn!("Attempt {} failed (retryable): {}", attempt, error_msg);
                        
                        // Exponential backoff
                        sleep(Duration::from_millis(delay)).await;
                        delay = (delay * 2).min(self.config.max_retry_delay_ms);
                        continue;
                    } else {
                        error!("Failed after {} attempts: {}", attempt, error_msg);
                        return Err(Error::Runtime { 
                            message: format!("AWS Titan error after {} attempts: {}", attempt, error_msg) 
                        });
                    }
                }
                Err(_) => {
                    attempt += 1;
                    warn!("Request timeout on attempt {}", attempt);
                    
                    if attempt <= self.config.max_retries {
                        sleep(Duration::from_millis(delay)).await;
                        delay = (delay * 2).min(self.config.max_retry_delay_ms);
                        continue;
                    } else {
                        return Err(Error::Runtime { 
                            message: format!("Request timeout after {} attempts", attempt) 
                        });
                    }
                }
            }
        }
    }
    
    /// Check if an error is retryable
    fn is_retryable_error(&self, error_msg: &str) -> bool {
        // Retryable errors
        let retryable_patterns = [
            "throttled",
            "rate limit",
            "too many requests",
            "service unavailable",
            "timeout",
            "connection",
            "temporary",
            "transient",
        ];
        
        let lower = error_msg.to_lowercase();
        retryable_patterns.iter().any(|pattern| lower.contains(pattern))
    }
    
    /// Process large batches with automatic chunking
    pub async fn create_embeddings_batch(
        &self,
        texts: Vec<String>,
        model: Option<&str>,
    ) -> Result<EmbeddingResponse> {
        if texts.len() <= self.config.batch_size {
            // Small batch, process directly
            return self.create_embeddings_with_retry(texts, model.map(|s| s.to_string())).await;
        }
        
        // Large batch, process in chunks
        info!("Processing {} texts in chunks of {}", texts.len(), self.config.batch_size);
        let mut all_embeddings = Vec::new();
        
        for (i, chunk) in texts.chunks(self.config.batch_size).enumerate() {
            info!("Processing chunk {}/{}", i + 1, 
                (texts.len() + self.config.batch_size - 1) / self.config.batch_size);
            
            let response = self.create_embeddings_with_retry(
                chunk.to_vec(),
                model.map(|s| s.to_string())
            ).await?;
            
            all_embeddings.extend(response.embeddings);
            
            // Small delay between chunks to avoid bursts
            if i < texts.chunks(self.config.batch_size).len() - 1 {
                sleep(Duration::from_millis(500)).await;
            }
        }
        
        Ok(EmbeddingResponse {
            embeddings: all_embeddings,
            usage: None,
        })
    }
}

#[async_trait]
impl IEmbedder for RobustAwsTitan {
    async fn create_embeddings(
        &self,
        texts: Vec<String>,
        model: Option<&str>,
    ) -> Result<EmbeddingResponse> {
        self.create_embeddings_batch(texts, model).await
    }
    
    async fn validate_configuration(&self) -> Result<(bool, Option<String>)> {
        self.inner.validate_configuration().await
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
