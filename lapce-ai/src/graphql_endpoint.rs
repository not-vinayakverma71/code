/// GraphQL Endpoint - Day 35 PM
use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema, SimpleObject};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::working_cache_system::WorkingCacheSystem;
use crate::vector_search::VectorSearch;
use crate::minilm_embeddings::MiniLMEmbeddings;

#[derive(SimpleObject)]
pub struct CacheEntry {
    key: String,
    value: String,
    size: usize,
}

#[derive(SimpleObject)]
pub struct SearchResult {
    id: usize,
    score: f32,
    content: String,
}

#[derive(SimpleObject)]
pub struct EmbeddingResult {
    text: String,
    embedding: Vec<f32>,
    dimension: usize,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn cache_get(&self, ctx: &Context<'_>, key: String) -> Option<CacheEntry> {
        let cache = ctx.data::<Arc<WorkingCacheSystem>>().unwrap();
        
        match cache.get(&key).await {
            Ok(value) => {
                let size = value.len();
                Some(CacheEntry {
                    key,
                    value: String::from_utf8_lossy(&value).to_string(),
                    size,
                })
            }
            Err(_) => None
        }
    }
    
    async fn vector_search(&self, ctx: &Context<'_>, query_text: String, top_k: usize) -> Vec<SearchResult> {
        let embeddings = ctx.data::<Arc<MiniLMEmbeddings>>().unwrap();
        let search = ctx.data::<Arc<RwLock<VectorSearch>>>().unwrap();
        
        let query_embedding = embeddings.embed(&query_text);
        let search = search.read().await;
        
        search.search(&query_embedding, top_k)
            .into_iter()
            .map(|(id, score, content)| SearchResult {
                id,
                score,
                content,
            })
            .collect()
    }
    
    async fn generate_embedding(&self, ctx: &Context<'_>, text: String) -> EmbeddingResult {
        let embeddings = ctx.data::<Arc<MiniLMEmbeddings>>().unwrap();
        let embedding = embeddings.embed(&text);
        let dimension = embedding.len();
        
        EmbeddingResult {
            text,
            embedding,
            dimension,
        }
    }
}

pub fn create_schema() -> Schema<QueryRoot, EmptyMutation, EmptySubscription> {
    Schema::new(QueryRoot, EmptyMutation, EmptySubscription)
}
