/// Query Cache - EXACT implementation from docs lines 413-443
use std::sync::Arc;
use std::future::Future;
use anyhow::Result;

use super::{
    l1_cache::L1Cache,
    query_hasher::QueryHasher,
    types::{CacheKey, CacheValue, QueryResult},
};

pub struct QueryCache {
    pub cache: Arc<L1Cache>,
    pub query_hasher: QueryHasher,
}

impl QueryCache {
    pub fn new(cache: Arc<L1Cache>) -> Self {
        Self {
            cache,
            query_hasher: QueryHasher::new(),
        }
    }
    
    pub async fn get_or_compute<F, Fut>(
        &self,
        query: &str,
        compute: F,
    ) -> Result<QueryResult>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<QueryResult>>,
    {
        let key = self.query_hasher.hash(query);
        
        if let Some(cached) = self.cache.get(&key).await {
            if let Ok(result) = cached.try_into() {
                return Ok(result);
            }
        }
        
        let result = compute().await?;
        self.cache.put(key, CacheValue::from(result.clone())).await;
        
        Ok(result)
    }
}
