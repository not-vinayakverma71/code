/// Rate limiting implementation
/// DAY 7 H5-6: Translate rate limiting

use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Rate limiter using token bucket algorithm
pub struct TokenBucketRateLimiter {
    capacity: f64,
    tokens: Arc<Mutex<f64>>,
    refill_rate: f64, // tokens per second
    last_refill: Arc<Mutex<Instant>>,
}

impl TokenBucketRateLimiter {
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: Arc::new(Mutex::new(capacity)),
            refill_rate,
            last_refill: Arc::new(Mutex::new(Instant::now())),
        }
    }
    
    /// Try to consume tokens, returns true if successful
    pub async fn try_consume(&self, tokens_needed: f64) -> bool {
        let mut tokens = self.tokens.lock().await;
        let mut last_refill = self.last_refill.lock().await;
        
        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let tokens_to_add = (elapsed * self.refill_rate).min(self.capacity - *tokens);
        
        *tokens += tokens_to_add;
        *last_refill = now;
        
        // Check if we have enough tokens
        if *tokens >= tokens_needed {
            *tokens -= tokens_needed;
            true
        } else {
            false
        }
    }
    
    /// Wait until tokens are available
    pub async fn consume(&self, tokens_needed: f64) {
        loop {
            if self.try_consume(tokens_needed).await {
                return;
            }
            // Calculate wait time
            let tokens_deficit = tokens_needed - *self.tokens.lock().await;
            let wait_seconds = tokens_deficit / self.refill_rate;
            tokio::time::sleep(Duration::from_secs_f64(wait_seconds.max(0.01))).await;
        }
    }
    
    /// Get current token count
    pub async fn available_tokens(&self) -> f64 {
        let mut tokens = self.tokens.lock().await;
        let mut last_refill = self.last_refill.lock().await;
        
        // Refill before checking
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        let tokens_to_add = (elapsed * self.refill_rate).min(self.capacity - *tokens);
        
        *tokens += tokens_to_add;
        *last_refill = now;
        
        *tokens
    }
}

/// Sliding window rate limiter
pub struct SlidingWindowRateLimiter {
    window_size: Duration,
    max_requests: usize,
    requests: Arc<Mutex<VecDeque<Instant>>>,
}

impl SlidingWindowRateLimiter {
    pub fn new(window_size: Duration, max_requests: usize) -> Self {
        Self {
            window_size,
            max_requests,
            requests: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    
    /// Check if request is allowed
    pub async fn check_and_consume(&self) -> bool {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();
        
        // Remove old requests outside the window
        while let Some(front) = requests.front() {
            if now.duration_since(*front) > self.window_size {
                requests.pop_front();
            } else {
                break;
            }
        }
        
        // Check if we can add new request
        if requests.len() < self.max_requests {
            requests.push_back(now);
            true
        } else {
            false
        }
    }
    
    /// Get current request count in window
    pub async fn current_count(&self) -> usize {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();
        
        // Clean old requests
        while let Some(front) = requests.front() {
            if now.duration_since(*front) > self.window_size {
                requests.pop_front();
            } else {
                break;
            }
        }
        
        requests.len()
    }
    
    /// Reset the rate limiter
    pub async fn reset(&self) {
        self.requests.lock().await.clear();
    }
}

/// Multi-tier rate limiter with different limits per tier
pub struct MultiTierRateLimiter {
    tiers: HashMap<String, Arc<TokenBucketRateLimiter>>,
    default_limiter: Arc<TokenBucketRateLimiter>,
}

impl MultiTierRateLimiter {
    pub fn new() -> Self {
        Self {
            tiers: HashMap::new(),
            default_limiter: Arc::new(TokenBucketRateLimiter::new(100.0, 10.0)),
        }
    }
    
    /// Add a tier with specific limits
    pub fn add_tier(&mut self, tier_name: String, capacity: f64, refill_rate: f64) {
        self.tiers.insert(
            tier_name,
            Arc::new(TokenBucketRateLimiter::new(capacity, refill_rate)),
        );
    }
    
    /// Get rate limiter for a specific tier
    pub fn get_limiter(&self, tier: &str) -> Arc<TokenBucketRateLimiter> {
        self.tiers.get(tier)
            .cloned()
            .unwrap_or_else(|| self.default_limiter.clone())
    }
    
    /// Check and consume from specific tier
    pub async fn try_consume(&self, tier: &str, tokens: f64) -> bool {
        self.get_limiter(tier).try_consume(tokens).await
    }
}

/// Distributed rate limiter using shared state
pub struct DistributedRateLimiter {
    limits: Arc<RwLock<HashMap<String, RateLimitState>>>,
    default_limit: RateLimitConfig,
}

#[derive(Debug, Clone)]
struct RateLimitState {
    count: usize,
    window_start: Instant,
    config: RateLimitConfig,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: usize,
    pub window_duration: Duration,
    pub burst_capacity: Option<usize>,
}

impl DistributedRateLimiter {
    pub fn new(default_config: RateLimitConfig) -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            default_limit: default_config,
        }
    }
    
    /// Check if request is allowed for given key
    pub async fn check_rate_limit(&self, key: &str) -> bool {
        let mut limits = self.limits.write().await;
        let now = Instant::now();
        
        let state = limits.entry(key.to_string()).or_insert(RateLimitState {
            count: 0,
            window_start: now,
            config: self.default_limit.clone(),
        });
        
        // Check if window has expired
        if now.duration_since(state.window_start) > state.config.window_duration {
            state.count = 0;
            state.window_start = now;
        }
        
        // Check burst capacity if configured
        if let Some(burst) = state.config.burst_capacity {
            if state.count == 0 && burst > state.config.max_requests {
                // Allow burst
                state.count = 1;
                return true;
            }
        }
        
        // Check regular limit
        if state.count < state.config.max_requests {
            state.count += 1;
            true
        } else {
            false
        }
    }
    
    /// Update configuration for a specific key
    pub async fn update_config(&self, key: &str, config: RateLimitConfig) {
        let mut limits = self.limits.write().await;
        if let Some(state) = limits.get_mut(key) {
            state.config = config;
        }
    }
    
    /// Get current usage for a key
    pub async fn get_usage(&self, key: &str) -> Option<(usize, Duration)> {
        let limits = self.limits.read().await;
        limits.get(key).map(|state| {
            let remaining_window = state.config.window_duration
                .saturating_sub(Instant::now().duration_since(state.window_start));
            (state.count, remaining_window)
        })
    }
}

/// Adaptive rate limiter that adjusts based on system load
pub struct AdaptiveRateLimiter {
    base_limiter: Arc<RwLock<TokenBucketRateLimiter>>,
    load_factor: Arc<RwLock<f64>>,
    min_rate: f64,
    max_rate: f64,
}

impl AdaptiveRateLimiter {
    pub fn new(initial_capacity: f64, initial_rate: f64, min_rate: f64, max_rate: f64) -> Self {
        Self {
            base_limiter: Arc::new(RwLock::new(TokenBucketRateLimiter::new(initial_capacity, initial_rate))),
            load_factor: Arc::new(RwLock::new(1.0)),
            min_rate,
            max_rate,
        }
    }
    
    /// Update load factor based on system metrics
    pub async fn update_load_factor(&self, cpu_usage: f64, memory_usage: f64) {
        // Simple load calculation
        let load = (cpu_usage + memory_usage) / 2.0;
        
        let mut factor = self.load_factor.write().await;
        if load > 0.8 {
            // High load, reduce rate
            *factor = (*factor * 0.9).max(self.min_rate / self.max_rate);
        } else if load < 0.5 {
            // Low load, increase rate
            *factor = (*factor * 1.1).min(1.0);
        }
        
        // Update base limiter rate
        let new_rate = self.max_rate * *factor;
        let mut limiter = self.base_limiter.write().await;
        limiter.refill_rate = new_rate.clamp(self.min_rate, self.max_rate);
    }
    
    /// Try to consume with adaptive rate
    pub async fn try_consume(&self, tokens: f64) -> bool {
        self.base_limiter.read().await.try_consume(tokens).await
    }
    
    /// Get current effective rate
    pub async fn get_effective_rate(&self) -> f64 {
        self.base_limiter.read().await.refill_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_token_bucket() {
        let limiter = TokenBucketRateLimiter::new(10.0, 5.0);
        
        // Should have full capacity initially
        assert!(limiter.try_consume(5.0).await);
        assert!(limiter.try_consume(5.0).await);
        assert!(!limiter.try_consume(1.0).await);
        
        // Wait for refill
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // Should have refilled some tokens
        assert!(limiter.try_consume(1.0).await);
    }
    
    #[tokio::test]
    async fn test_sliding_window() {
        let limiter = SlidingWindowRateLimiter::new(Duration::from_secs(1), 3);
        
        // Should allow 3 requests
        assert!(limiter.check_and_consume().await);
        assert!(limiter.check_and_consume().await);
        assert!(limiter.check_and_consume().await);
        
        // 4th should be denied
        assert!(!limiter.check_and_consume().await);
        
        // Wait for window to pass
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Should allow again
        assert!(limiter.check_and_consume().await);
    }
    
    #[tokio::test]
    async fn test_distributed_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_duration: Duration::from_secs(1),
            burst_capacity: Some(10),
        };
        
        let limiter = DistributedRateLimiter::new(config);
        
        // Test rate limiting for a key
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("user1").await);
        }
        assert!(!limiter.check_rate_limit("user1").await);
        
        // Different key should work
        assert!(limiter.check_rate_limit("user2").await);
        
        // Check usage
        let usage = limiter.get_usage("user1").await;
        assert!(usage.is_some());
        let (count, _) = usage.unwrap();
        assert_eq!(count, 5);
    }
    
    #[tokio::test]
    async fn test_multi_tier() {
        let mut limiter = MultiTierRateLimiter::new();
        limiter.add_tier("premium".to_string(), 100.0, 20.0);
        limiter.add_tier("free".to_string(), 10.0, 2.0);
        
        // Premium tier should have higher limits
        assert!(limiter.try_consume("premium", 50.0).await);
        
        // Free tier has lower limits
        assert!(!limiter.try_consume("free", 50.0).await);
    }
}
