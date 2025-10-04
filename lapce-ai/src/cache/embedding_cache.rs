/// EmbeddingCache - EXACT implementation from docs lines 447-490
use std::sync::Arc;
use std::future::Future;
use anyhow::Result;
use dashmap::DashMap;

pub struct EmbeddingCache {
    pub cache: DashMap<String, Arc<Vec<f32>>>,
    pub max_entries: usize,
}

impl EmbeddingCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: DashMap::new(),
            max_entries,
        }
    }
    
    pub async fn get_or_embed<F, Fut>(
        &self,
        text: &str,
        embed: F,
    ) -> Result<Arc<Vec<f32>>>
    where
        F: FnOnce(&str) -> Fut,
        Fut: Future<Output = Result<Vec<f32>>>,
    {
        // Check cache
        if let Some(embedding) = self.cache.get(text) {
            return Ok(embedding.clone());
        }
        
        // Compute embedding
        let embedding = Arc::new(embed(text).await?);
        
        // Cache with size limit
        if self.cache.len() < self.max_entries {
            self.cache.insert(text.to_string(), embedding.clone());
        } else {
            // Random eviction for simplicity
            if rand::random::<f32>() < 0.1 {
                let key = self.cache.iter()
                    .next()
                    .map(|entry| entry.key().clone());
                    
                if let Some(key) = key {
                    self.cache.remove(&key);
                    self.cache.insert(text.to_string(), embedding.clone());
                }
            }
        }
        
        Ok(embedding)
    }
}
