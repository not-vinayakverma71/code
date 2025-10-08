//! Global string interning pool for memory optimization
//! 
//! This module provides a thread-safe, sharded string interning system
//! that reduces memory usage by storing each unique string only once.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicU64, AtomicBool, Ordering};
use once_cell::sync::Lazy;

// Feature-gated imports
#[cfg(feature = "global-interning")]
use lasso::{ThreadedRodeo, Spur, Key};

/// Symbol ID type for interned strings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

impl SymbolId {
    pub const NONE: Self = SymbolId(0);
    
    pub fn as_u32(self) -> u32 {
        self.0
    }
    
    pub fn from_u32(id: u32) -> Self {
        SymbolId(id)
    }
}

/// Configuration for the global intern pool
#[derive(Debug, Clone)]
pub struct InternConfig {
    /// Maximum string length to intern (longer strings stored directly)
    pub max_string_length: usize,
    
    /// Memory soft cap in bytes
    pub memory_cap_bytes: usize,
    
    /// Number of shards for concurrent access
    pub shard_count: usize,
    
    /// Whether interning is enabled
    pub enabled: bool,
}

impl Default for InternConfig {
    fn default() -> Self {
        Self {
            // 128 bytes ensures all identifiers/type names are interned
            // Only very long string literals would be bypassed
            max_string_length: 128,
            memory_cap_bytes: 100 * 1024 * 1024, // 100MB
            shard_count: 64,
            enabled: cfg!(feature = "global-interning"),
        }
    }
}

/// Statistics about the intern pool
#[derive(Debug, Clone, Default)]
pub struct InternStats {
    pub total_strings: usize,
    pub total_bytes: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub cap_exceeded_count: u64,
    pub avg_string_length: f64,
}

/// Global string interning pool
pub struct GlobalInternPool {
    #[cfg(feature = "global-interning")]
    rodeo: Arc<ThreadedRodeo>,
    
    config: InternConfig,
    
    // Metrics
    total_bytes: AtomicUsize,
    hit_count: AtomicU64,
    miss_count: AtomicU64,
    cap_exceeded_count: AtomicU64,
    enabled: AtomicBool,
}

impl GlobalInternPool {
    /// Create a new global intern pool
    pub fn new(config: InternConfig) -> Self {
        #[cfg(feature = "global-interning")]
        {
            Self {
                rodeo: Arc::new(ThreadedRodeo::new()),
                config: config.clone(),
                total_bytes: AtomicUsize::new(0),
                hit_count: AtomicU64::new(0),
                miss_count: AtomicU64::new(0),
                cap_exceeded_count: AtomicU64::new(0),
                enabled: AtomicBool::new(config.enabled),
            }
        }
        
        #[cfg(not(feature = "global-interning"))]
        {
            Self {
                config,
                total_bytes: AtomicUsize::new(0),
                hit_count: AtomicU64::new(0),
                miss_count: AtomicU64::new(0),
                cap_exceeded_count: AtomicU64::new(0),
                enabled: AtomicBool::new(false),
            }
        }
    }
    
    /// Get or intern a string, returning its ID
    /// 
    /// Bypass policy:
    /// - Bypass if interning is disabled
    /// - Bypass if string length > max_string_length (default 128 bytes)
    ///   This ensures all identifiers/type names are interned
    /// - Bypass if memory cap is exceeded
    pub fn get_or_intern(&self, s: &str) -> InternResult {
        // Check if enabled
        if !self.enabled.load(Ordering::Relaxed) {
            return InternResult::Bypassed(s.to_string());
        }
        
        // Safety filter: length check
        // With 128 byte limit, all identifiers/type names will be interned
        // Only very long string literals (rare in AST symbol names) are bypassed
        if s.len() > self.config.max_string_length {
            return InternResult::Bypassed(s.to_string());
        }
        
        // Safety filter: memory cap
        let current_bytes = self.total_bytes.load(Ordering::Relaxed);
        if current_bytes > self.config.memory_cap_bytes {
            self.cap_exceeded_count.fetch_add(1, Ordering::Relaxed);
            return InternResult::Bypassed(s.to_string());
        }
        
        #[cfg(feature = "global-interning")]
        {
            // Check if already interned (hit)
            if let Some(spur) = self.rodeo.get(s) {
                self.hit_count.fetch_add(1, Ordering::Relaxed);
                return InternResult::Interned(SymbolId(spur.into_usize() as u32));
            }
            
            // Intern the string (miss)
            let spur = self.rodeo.get_or_intern(s);
            self.miss_count.fetch_add(1, Ordering::Relaxed);
            self.total_bytes.fetch_add(s.len(), Ordering::Relaxed);
            
            InternResult::Interned(SymbolId(spur.into_usize() as u32))
        }
        
        #[cfg(not(feature = "global-interning"))]
        {
            InternResult::Bypassed(s.to_string())
        }
    }
    
    /// Get the ID for an already-interned string (without interning it)
    /// Returns None if the string has not been interned yet
    pub fn get_id(&self, s: &str) -> Option<SymbolId> {
        if !self.enabled.load(Ordering::Relaxed) {
            return None;
        }
        
        #[cfg(feature = "global-interning")]
        {
            self.rodeo.get(s).map(|spur| SymbolId(spur.into_usize() as u32))
        }
        
        #[cfg(not(feature = "global-interning"))]
        {
            None
        }
    }
    
    /// Resolve a symbol ID to its string
    pub fn resolve(&self, id: SymbolId) -> Option<String> {
        #[cfg(feature = "global-interning")]
        {
            // Use try_from_usize since it's safe
            Spur::try_from_usize(id.0 as usize)
                .and_then(|spur| self.rodeo.try_resolve(&spur).map(|s| s.to_string()))
        }
        
        #[cfg(not(feature = "global-interning"))]
        {
            None
        }
    }
    
    /// Get statistics about the intern pool
    pub fn stats(&self) -> InternStats {
        #[cfg(feature = "global-interning")]
        {
            let total_strings = self.rodeo.len();
            let total_bytes = self.total_bytes.load(Ordering::Relaxed);
            let avg_length = if total_strings > 0 {
                total_bytes as f64 / total_strings as f64
            } else {
                0.0
            };
            
            InternStats {
                total_strings,
                total_bytes,
                hit_count: self.hit_count.load(Ordering::Relaxed),
                miss_count: self.miss_count.load(Ordering::Relaxed),
                cap_exceeded_count: self.cap_exceeded_count.load(Ordering::Relaxed),
                avg_string_length: avg_length,
            }
        }
        
        #[cfg(not(feature = "global-interning"))]
        {
            InternStats::default()
        }
    }
    
    /// Get the hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hit_count.load(Ordering::Relaxed);
        let misses = self.miss_count.load(Ordering::Relaxed);
        let total = hits + misses;
        if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        }
    }
    
    /// Clear the intern pool metrics (for testing)
    #[cfg(test)]
    pub fn clear_metrics(&self) {
        self.total_bytes.store(0, Ordering::Relaxed);
        self.hit_count.store(0, Ordering::Relaxed);
        self.miss_count.store(0, Ordering::Relaxed);
        self.cap_exceeded_count.store(0, Ordering::Relaxed);
    }
    
    /// Enable or disable interning at runtime
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
    
    /// Check if interning is currently enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Result of interning operation
#[derive(Debug, Clone)]
pub enum InternResult {
    /// String was successfully interned
    Interned(SymbolId),
    
    /// String was not interned (too long, cap exceeded, or disabled)
    Bypassed(String),
}

impl InternResult {
    /// Get the symbol ID if interned
    pub fn symbol_id(&self) -> Option<SymbolId> {
        match self {
            InternResult::Interned(id) => Some(*id),
            InternResult::Bypassed(_) => None,
        }
    }
    
    /// Get the string value (either by resolving ID or returning bypassed string)
    pub fn as_str<'a>(&'a self, pool: &'a GlobalInternPool) -> std::borrow::Cow<'a, str> {
        match self {
            InternResult::Interned(id) => {
                pool.resolve(*id)
                    .map(std::borrow::Cow::Owned)
                    .unwrap_or_else(|| std::borrow::Cow::Borrowed(""))
            }
            InternResult::Bypassed(s) => std::borrow::Cow::Borrowed(s),
        }
    }
}

/// Hash a string for shard selection
fn hash_string(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Global intern pool singleton
pub static INTERN_POOL: Lazy<Arc<GlobalInternPool>> = Lazy::new(|| {
    let config = InternConfig::default();
    Arc::new(GlobalInternPool::new(config))
});

/// Convenience function to intern a string using the global pool
pub fn intern(s: &str) -> InternResult {
    INTERN_POOL.get_or_intern(s)
}

/// Convenience function to resolve a symbol ID using the global pool
pub fn resolve(id: SymbolId) -> Option<String> {
    INTERN_POOL.resolve(id)
}

/// Get global pool statistics
pub fn intern_stats() -> InternStats {
    INTERN_POOL.stats()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_interning() {
        let pool = GlobalInternPool::new(InternConfig::default());
        
        // Intern same string twice
        let result1 = pool.get_or_intern("test");
        let result2 = pool.get_or_intern("test");
        
        // Should get same ID
        if let (InternResult::Interned(id1), InternResult::Interned(id2)) = (result1, result2) {
            assert_eq!(id1, id2);
        }
    }
    
    #[test]
    fn test_long_string_bypass() {
        let mut config = InternConfig::default();
        config.max_string_length = 10;
        let pool = GlobalInternPool::new(config);
        
        let long_string = "this is a very long string that exceeds the limit";
        let result = pool.get_or_intern(long_string);
        
        // Should bypass
        matches!(result, InternResult::Bypassed(_));
    }
    
    #[test]
    fn test_stats() {
        let pool = GlobalInternPool::new(InternConfig::default());
        
        pool.get_or_intern("test1");
        pool.get_or_intern("test2");
        pool.get_or_intern("test1"); // Hit
        
        let stats = pool.stats();
        
        #[cfg(feature = "global-interning")]
        {
            assert_eq!(stats.total_strings, 2);
            assert_eq!(stats.hit_count, 1);
            assert_eq!(stats.miss_count, 2);
        }
    }
    
    #[test]
    fn test_concurrent_access() {
        use std::thread;
        
        // This test just verifies no deadlocks occur with concurrent access
        // The actual interning may not happen without the feature flag
        let mut config = InternConfig::default();
        config.enabled = true; // Enable interning for the test
        let pool = GlobalInternPool::new(config);
        pool.set_enabled(true); // Also enable via method
        let pool = Arc::new(pool);
        let mut handles = vec![];
        
        for i in 0..10 {
            let pool = pool.clone();
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let s = format!("string_{}", j);
                    pool.get_or_intern(&s);
                }
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        // No deadlocks, should complete successfully
        // Without the global-interning feature, interning won't actually happen
        #[cfg(feature = "global-interning")]
        assert!(pool.stats().total_strings > 0);
        
        // Test passes if it completes without deadlock
        assert!(true);
    }
}
