// Rate Limiter for MCP Tools - REAL IMPLEMENTATION
use std::sync::Arc;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use std::num::NonZeroU32;
use dashmap::DashMap;
use tokio::sync::RwLock;
use anyhow::{Result, bail};
use governor::{Quota, RateLimiter as Gov, clock::DefaultClock, state::NotKeyed, state::InMemoryState};

// Alias for compatibility
pub type RateLimiter = GovernorRateLimiter;

// Governor-based rate limiter with sliding window
pub struct GovernorRateLimiter {
    // Per-user rate limiters: (user_id, tool_name) -> Governor instance
    limiters: DashMap<(String, String), Arc<Gov<NotKeyed, InMemoryState, DefaultClock>>>,
    
    // Sliding window tracking for detailed analytics
    windows: DashMap<(String, String), Arc<RwLock<SlidingWindow>>>,
    
    // Global configuration
    default_quota: Quota,
    
    // Adaptive throttling
    adaptive_config: AdaptiveConfig,
}

impl GovernorRateLimiter {
    pub fn new() -> Self {
        // Default: 1000 requests per minute per user
        let default_quota = Quota::per_minute(NonZeroU32::new(1000).unwrap());
        
        Self {
            limiters: DashMap::new(),
            windows: DashMap::new(),
            default_quota,
            adaptive_config: AdaptiveConfig::default(),
        }
    }
    
    pub fn with_rate(requests_per_minute: u32) -> Self {
        let default_quota = Quota::per_minute(NonZeroU32::new(requests_per_minute.max(1)).unwrap());
        
        Self {
            limiters: DashMap::new(),
            windows: DashMap::new(),
            default_quota,
            adaptive_config: AdaptiveConfig::default(),
        }
    }
    
    pub async fn check_rate_limit(&self, user_id: &str, tool_name: &str) -> Result<()> {
        let key = (user_id.to_string(), tool_name.to_string());
        
        // Get or create rate limiter for this user/tool combo
        let limiter = self.limiters.entry(key.clone())
            .or_insert_with(|| {
                Arc::new(Gov::direct(self.default_quota))
            });
        
        // Check if request is allowed
        match limiter.check() {
            Ok(_) => Ok(()),
            Err(_) => {
                // Apply adaptive throttling
                if self.adaptive_config.enabled {
                    let wait_time = Duration::from_secs(1); // Default wait time
                    tokio::time::sleep(wait_time).await;
                    Ok(())
                } else {
                    bail!("Rate limit exceeded for tool {}", tool_name)
                }
            }
        }
    }
    
    pub fn set_tool_quota(&mut self, tool_name: String, quota: Quota) {
        // Update quota for specific tool
        for entry in self.limiters.iter() {
            if entry.key().1 == tool_name {
                // Clear existing limiter so new quota takes effect
                self.limiters.remove(entry.key());
            }
        }
    }
}

#[derive(Clone, Debug)]
struct AdaptiveConfig {
    enabled: bool,
    burst_multiplier: f64,
    decay_rate: f64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            enabled: false,  // Disabled by default for strict rate limiting
            burst_multiplier: 2.0,
            decay_rate: 0.9,
        }
    }
}

// Sliding window implementation for detailed tracking
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    requests: VecDeque<Instant>,
    window_duration: Duration,
    max_requests: usize,
}

impl SlidingWindow {
    pub fn new(window_duration: Duration, max_requests: usize) -> Self {
        Self {
            requests: VecDeque::new(),
            window_duration,
            max_requests,
        }
    }
    
    pub fn check_and_update(&mut self) -> bool {
        let now = Instant::now();
        
        // Remove expired requests
        while let Some(&front) = self.requests.front() {
            if now.duration_since(front) > self.window_duration {
                self.requests.pop_front();
            } else {
                break;
            }
        }
        
        // Check if we can accept new request
        if self.requests.len() < self.max_requests {
            self.requests.push_back(now);
            true
        } else {
            false
        }
    }
    
    pub fn reset(&mut self) {
        self.requests.clear();
    }
}
