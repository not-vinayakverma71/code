/// Enhanced Rate Limiter with Sliding Window
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct EnhancedRateLimiter {
    windows: Arc<RwLock<HashMap<String, SlidingWindow>>>,
    config: RateLimitConfig,
}

#[derive(Clone)]
pub struct RateLimitConfig {
    pub window_size: Duration,
    pub max_requests: usize,
    pub burst_size: usize,
    pub adaptive: bool,
}

struct SlidingWindow {
    requests: VecDeque<Instant>,
    window_size: Duration,
    max_requests: usize,
    burst_tokens: usize,
    burst_size: usize,
}

impl EnhancedRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            windows: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    pub async fn check_and_consume(&self, key: &str) -> Result<(), RateLimitError> {
        let mut windows = self.windows.write().await;
        let window = windows.entry(key.to_string())
            .or_insert_with(|| SlidingWindow::new(
                self.config.window_size,
                self.config.max_requests,
                self.config.burst_size,
            ));
        
        window.check_and_consume()
    }
    
    pub async fn get_metrics(&self, key: &str) -> RateLimitMetrics {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(key) {
            window.get_metrics()
        } else {
            RateLimitMetrics::default()
        }
    }
}

impl SlidingWindow {
    fn new(window_size: Duration, max_requests: usize, burst_size: usize) -> Self {
        Self {
            requests: VecDeque::new(),
            window_size,
            max_requests,
            burst_tokens: burst_size,
            burst_size,
        }
    }
    
    fn check_and_consume(&mut self) -> Result<(), RateLimitError> {
        let now = Instant::now();
        
        // Remove old requests outside the window
        while let Some(&front) = self.requests.front() {
            if now.duration_since(front) > self.window_size {
                self.requests.pop_front();
            } else {
                break;
            }
        }
        
        // Check if we can accept the request
        if self.requests.len() >= self.max_requests {
            // Try burst tokens
            if self.burst_tokens > 0 {
                self.burst_tokens -= 1;
                self.requests.push_back(now);
                Ok(())
            } else {
                Err(RateLimitError::Exceeded {
                    retry_after: self.calculate_retry_after(),
                })
            }
        } else {
            self.requests.push_back(now);
            
            // Regenerate burst tokens
            if self.burst_tokens < self.burst_size {
                self.burst_tokens += 1;
            }
            
            Ok(())
        }
    }
    
    fn calculate_retry_after(&self) -> Duration {
        if let Some(&front) = self.requests.front() {
            let elapsed = Instant::now().duration_since(front);
            if elapsed < self.window_size {
                self.window_size - elapsed
            } else {
                Duration::from_secs(0)
            }
        } else {
            Duration::from_secs(0)
        }
    }
    
    fn get_metrics(&self) -> RateLimitMetrics {
        RateLimitMetrics {
            current_requests: self.requests.len(),
            max_requests: self.max_requests,
            burst_tokens: self.burst_tokens,
            window_size: self.window_size,
        }
    }
}

#[derive(Debug)]
pub enum RateLimitError {
    Exceeded { retry_after: Duration },
}

#[derive(Default)]
pub struct RateLimitMetrics {
    pub current_requests: usize,
    pub max_requests: usize,
    pub burst_tokens: usize,
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            window_size: Duration::from_secs(60),
            max_requests: 60,
            burst_size: 10,
            adaptive: false,
        }
    }
}
