/// Query Hasher - EXACT implementation from docs line 417
use blake3;
use super::types::CacheKey;

/// Query hasher for consistent cache key generation
pub struct QueryHasher {
    /// Salt for additional security
    salt: Vec<u8>,
}

impl QueryHasher {
    pub fn new() -> Self {
        Self {
            salt: b"lapce_cache_v3".to_vec(),
        }
    }
    
    /// Hash a query string into a cache key
    pub fn hash(&self, query: &str) -> CacheKey {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.salt);
        hasher.update(query.as_bytes());
        
        let hash = hasher.finalize();
        CacheKey(hash.to_hex().to_string())
    }
    
    /// Hash with additional context
    pub fn hash_with_context(&self, query: &str, context: &str) -> CacheKey {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.salt);
        hasher.update(query.as_bytes());
        hasher.update(b":");
        hasher.update(context.as_bytes());
        
        let hash = hasher.finalize();
        CacheKey(hash.to_hex().to_string())
    }
}
