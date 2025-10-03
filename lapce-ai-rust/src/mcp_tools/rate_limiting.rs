// Rate Limiting Implementation
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use anyhow::{Result, bail};

/// Token bucket implementation for rate limiting
pub struct TokenBucket {
    capacity: f64,
    tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }
    
    /// Check if tokens are available and consume them if so
    pub fn check_and_consume(&mut self, tokens_needed: f64) -> bool {
        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
        
        // Check and consume tokens
        if self.tokens >= tokens_needed {
            self.tokens -= tokens_needed;
            true
        } else {
            false
        }
    }
    
    pub fn tokens_available(&self) -> f64 {
        self.tokens
    }
}

pub struct RateLimiter {
    limits: Arc<RwLock<HashMap<String, TokenBucket>>>,
    default_capacity: f64,
    default_refill_rate: f64,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            default_capacity: 100.0,
            default_refill_rate: 10.0, // 10 tokens per second
        }
    }
    
    pub fn with_rate(rate_per_minute: u32) -> Self {
        let capacity = rate_per_minute as f64;
        let refill_rate = rate_per_minute as f64 / 60.0; // Convert to per-second rate
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            default_capacity: capacity,
            default_refill_rate: refill_rate,
        }
    }
    
    pub async fn check(&self, user_id: &str) -> Result<()> {
        self.check_with_cost(user_id, 1.0).await
    }
    
    pub async fn check_with_cost(&self, user_id: &str, cost: f64) -> Result<()> {
        let mut limits = self.limits.write().await;
        
        let bucket = limits.entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(self.default_capacity, self.default_refill_rate));
        
        if !bucket.check_and_consume(cost) {
            bail!("Rate limit exceeded for user: {} (cost: {})", user_id, cost);
        }
        
        Ok(())
    }
    
    pub async fn get_available_tokens(&self, user_id: &str) -> f64 {
        let limits = self.limits.read().await;
        limits.get(user_id)
            .map(|b| b.tokens_available())
            .unwrap_or(self.default_capacity)
    }
}
