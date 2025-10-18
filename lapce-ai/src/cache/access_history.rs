/// Access History - EXACT implementation from docs line 379
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use super::types::CacheKey;

/// Access history for tracking frequency and recency
pub struct AccessHistory {
    /// History of accesses: (key, timestamp)
    history: VecDeque<(CacheKey, Instant)>,
    /// Maximum history size
    max_size: usize,
}

impl AccessHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_size),
            max_size,
        }
    }
    
    /// Record an access
    pub fn record(&mut self, key: CacheKey) {
        let now = Instant::now();
        
        // Remove old entries if at capacity
        if self.history.len() >= self.max_size {
            self.history.pop_front();
        }
        
        self.history.push_back((key, now));
    }
    
    /// Get frequency of a key
    pub fn frequency(&self, key: &CacheKey) -> u32 {
        self.history
            .iter()
            .filter(|(k, _)| k == key)
            .count() as u32
    }
    
    /// Get recency of a key (duration since last access)
    pub fn recency(&self, key: &CacheKey) -> Duration {
        let now = Instant::now();
        
        self.history
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, time)| now.duration_since(*time))
            .unwrap_or(Duration::from_secs(3600)) // Default to 1 hour if never accessed
    }
    
    /// Clean up old entries
    pub fn cleanup(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.history.retain(|(_, time)| {
            now.duration_since(*time) < max_age
        });
    }
    
    /// Get top N most frequent keys
    pub fn top_frequent(&self, n: usize) -> Vec<(CacheKey, u32)> {
        use std::collections::HashMap;
        
        let mut counts: HashMap<CacheKey, u32> = HashMap::new();
        for (key, _) in &self.history {
            *counts.entry(key.clone()).or_insert(0) += 1;
        }
        
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        sorted.truncate(n);
        sorted
    }
}
