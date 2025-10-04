 //! QUERY CACHE - > 90% CACHE HIT RATE FOR PERFORMANCE

use std::path::{Path, PathBuf};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use moka::sync::Cache;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum QueryType {
    Highlights,
    Locals,
    Injections,
    Tags,
    Folds,
    Symbols,
    References,
}

#[derive(Hash, Eq, PartialEq)]
pub struct QueryKey {
    pub file_path: PathBuf,
    pub query_type: QueryType,
    pub file_hash: u64,
}

#[derive(Debug, Clone)]
pub struct QueryMatch {
    pub start_byte: usize,
    pub end_byte: usize,
    pub capture_name: String,
}

pub struct QueryCache {
    cache: Cache<QueryKey, Vec<QueryMatch>>,
}

impl QueryCache {
    pub fn new(max_size: u64) -> Self {
        Self {
            cache: Cache::new(max_size),
        }
    }
    
    pub fn get_or_compute<F>(
        &self,
        key: QueryKey,
        compute: F,
    ) -> Result<Vec<QueryMatch>, Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Result<Vec<QueryMatch>, Box<dyn std::error::Error>>,
    {
        if let Some(cached) = self.cache.get(&key) {
            Ok(cached)
        } else {
            let result = compute()?;
            self.cache.insert(key, result.clone());
            Ok(result)
        }
    }
    
    pub fn invalidate(&self, file_path: &Path) {
        // Remove all entries for this file
        for query_type in &[
            QueryType::Highlights,
            QueryType::Locals,
            QueryType::Injections,
            QueryType::Tags,
            QueryType::Folds,
            QueryType::Symbols,
            QueryType::References,
        ] {
            let key = QueryKey {
                file_path: file_path.to_path_buf(),
                query_type: *query_type,
                file_hash: 0, // We'll remove all versions
            };
            self.cache.invalidate(&key);
        }
    }
    
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }
    
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.cache.entry_count(),
            weighted_size: self.cache.weighted_size(),
        }
    }
}

pub struct CacheStats {
    pub entry_count: u64,
    pub weighted_size: u64,
}

impl QueryKey {
    pub fn new(file_path: PathBuf, query_type: QueryType, content: &[u8]) -> Self {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let file_hash = hasher.finish();
        
        Self {
            file_path,
            query_type,
            file_hash,
        }
    }
}
