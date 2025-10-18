/// Access Predictor - EXACT implementation from docs lines 517-527
use std::collections::HashMap;
use std::time::Instant;
use super::types::CacheKey;

/// Markov Chain for cache access prediction
pub struct MarkovChain<T> {
    /// Transition matrix: current state -> next state -> probability
    transitions: HashMap<T, HashMap<T, f64>>,
    /// Total observations for each state
    observations: HashMap<T, u32>,
}

impl MarkovChain<CacheKey> {
    pub fn new() -> Self {
        Self {
            transitions: HashMap::new(),
            observations: HashMap::new(),
        }
    }
    
    /// Record a transition from one state to another
    pub fn record_transition(&mut self, from: CacheKey, to: CacheKey) {
        let count = self.observations.entry(from.clone()).or_insert(0);
        *count += 1;
        
        let transitions = self.transitions.entry(from).or_insert_with(HashMap::new);
        let to_count = transitions.entry(to).or_insert(0.0);
        *to_count += 1.0;
    }
    
    /// Predict next states based on current pattern
    pub fn predict(&self, current: &CacheKey) -> Vec<(CacheKey, f64)> {
        if let Some(transitions) = self.transitions.get(current) {
            let total = self.observations.get(current).unwrap_or(&1);
            let mut predictions: Vec<(CacheKey, f64)> = transitions
                .iter()
                .map(|(key, count)| (key.clone(), count / *total as f64))
                .collect();
            
            // Sort by probability descending
            predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            predictions
        } else {
            Vec::new()
        }
    }
}

/// Time series data for access patterns
pub struct TimeSeries {
    /// Historical access data: (timestamp, cache key)
    data: Vec<(Instant, CacheKey)>,
    /// Window size for pattern detection
    window_size: usize,
}

impl TimeSeries {
    pub fn new(window_size: usize) -> Self {
        Self {
            data: Vec::new(),
            window_size,
        }
    }
    
    /// Record an access
    pub fn record(&mut self, key: CacheKey) {
        self.data.push((Instant::now(), key));
        
        // Keep only recent data
        if self.data.len() > self.window_size * 10 {
            let cutoff = self.data.len() - self.window_size * 5;
            self.data.drain(..cutoff);
        }
    }
    
    /// Get current access pattern
    pub fn current_pattern(&self) -> Vec<CacheKey> {
        let now = Instant::now();
        let window_duration = std::time::Duration::from_secs(60); // Last minute
        
        self.data
            .iter()
            .rev()
            .take_while(|(time, _)| now.duration_since(*time) < window_duration)
            .map(|(_, key)| key.clone())
            .take(self.window_size)
            .collect()
    }
    
    /// Detect repeating patterns
    pub fn detect_cycles(&self) -> Option<usize> {
        if self.data.len() < self.window_size * 2 {
            return None;
        }
        
        // Simple cycle detection: look for repeating subsequences
        let recent: Vec<_> = self.data.iter()
            .rev()
            .take(self.window_size)
            .map(|(_, k)| k.clone())
            .collect();
        
        for cycle_len in 2..=self.window_size / 2 {
            let mut is_cycle = true;
            for i in 0..cycle_len {
                if recent[i] != recent[i + cycle_len] {
                    is_cycle = false;
                    break;
                }
            }
            if is_cycle {
                return Some(cycle_len);
            }
        }
        
        None
    }
}

/// Access predictor combining Markov chains and time series
pub struct AccessPredictor {
    pub markov_chain: MarkovChain<CacheKey>,
    pub time_series: TimeSeries,
}

impl AccessPredictor {
    pub fn new() -> Self {
        Self {
            markov_chain: MarkovChain::new(),
            time_series: TimeSeries::new(100),
        }
    }
    
    /// Record an access and update models
    pub fn record_access(&mut self, key: CacheKey) {
        // Update time series
        let pattern = self.time_series.current_pattern();
        if let Some(last) = pattern.first() {
            // Record transition in Markov chain
            self.markov_chain.record_transition(last.clone(), key.clone());
        }
        self.time_series.record(key);
    }
    
    /// Predict next accesses based on current state
    pub fn predict_next_accesses(&self) -> Vec<(CacheKey, f64)> {
        let current_pattern = self.time_series.current_pattern();
        
        if let Some(current) = current_pattern.first() {
            // Use Markov chain for prediction
            let mut predictions = self.markov_chain.predict(current);
            
            // Boost probability if cycle detected
            if let Some(cycle_len) = self.time_series.detect_cycles() {
                if cycle_len < current_pattern.len() {
                    // Next in cycle has higher probability
                    let next_in_cycle = &current_pattern[cycle_len];
                    for (key, prob) in &mut predictions {
                        if key == next_in_cycle {
                            *prob = (*prob + 1.0) / 2.0; // Boost probability
                        }
                    }
                }
            }
            
            predictions
        } else {
            Vec::new()
        }
    }
}
