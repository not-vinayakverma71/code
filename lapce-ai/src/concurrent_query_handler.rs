/// Concurrent query handler for 100+ simultaneous searches
/// Uses tokio for async concurrency

use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, RwLock};
use tokio::time::timeout;
use tracing::{info, warn, debug};

use crate::hybrid_search_impl::{HybridSearchEngine, HybridResult};
use crate::query_cache::{QueryCache, SearchFilters};

pub struct ConcurrentQueryHandler {
    search_engine: Arc<HybridSearchEngine>,
    query_cache: Arc<QueryCache>,
    semaphore: Arc<Semaphore>,
    metrics: Arc<RwLock<QueryMetrics>>,
    timeout_duration: Duration,
}

impl ConcurrentQueryHandler {
    pub fn new(
        search_engine: Arc<HybridSearchEngine>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            search_engine,
            query_cache: Arc::new(QueryCache::new(10000, 300)), // 10k entries, 5 min TTL
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            metrics: Arc::new(RwLock::new(QueryMetrics::default())),
            timeout_duration: Duration::from_secs(10),
        }
    }
    
    /// Handle a single query with concurrency control
    pub async fn handle_query(
        &self,
        query: String,
        limit: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<HybridResult>> {
        // Acquire semaphore permit
        let _permit = self.semaphore.acquire().await?;
        let start = Instant::now();
        
        // Check cache
        let cache_key = self.query_cache.compute_key(&query, filters.as_ref());
        if let Some(cached) = self.query_cache.get(&cache_key).await {
            self.record_query(start.elapsed(), true).await;
            
            // Convert cached results
            return Ok(cached.results.into_iter().map(|r| HybridResult {
                path: r.path,
                content: r.content,
                semantic_score: r.score,
                keyword_score: 0.0,
                fused_score: r.score,
            }).collect());
        }
        
        // Execute search with timeout
        let result = timeout(
            self.timeout_duration,
            self.search_engine.hybrid_search(&query, limit)
        ).await;
        
        match result {
            Ok(Ok(results)) => {
                // Cache results
                let cached = crate::query_cache::CachedResult {
                    results: results.iter().map(|r| crate::query_cache::SearchResult {
                        path: r.path.clone(),
                        content: r.content.clone(),
                        score: r.fused_score,
                        start_line: 0,
                        end_line: 0,
                    }).collect(),
                    query: query.clone(),
                    timestamp: chrono::Utc::now(),
                };
                
                self.query_cache.insert(cache_key, cached).await;
                self.record_query(start.elapsed(), false).await;
                
                Ok(results)
            }
            Ok(Err(e)) => {
                self.record_error().await;
                Err(e)
            }
            Err(_) => {
                self.record_timeout().await;
                Err(anyhow::anyhow!("Query timeout after {:?}", self.timeout_duration))
            }
        }
    }
    
    /// Handle multiple queries concurrently
    pub async fn handle_batch_queries(
        &self,
        queries: Vec<(String, usize, Option<SearchFilters>)>,
    ) -> Vec<Result<Vec<HybridResult>>> {
        let tasks: Vec<_> = queries
            .into_iter()
            .map(|(query, limit, filters)| {
                let handler = self.clone();
                tokio::spawn(async move {
                    handler.handle_query(query, limit, filters).await
                })
            })
            .collect();
        
        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(anyhow::anyhow!("Task error: {}", e))),
            }
        }
        
        results
    }
    
    /// Record query metrics
    async fn record_query(&self, latency: Duration, from_cache: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.total_queries += 1;
        
        if from_cache {
            metrics.cache_hits += 1;
        }
        
        let latency_ms = latency.as_millis() as u64;
        metrics.total_latency_ms += latency_ms;
        
        if latency_ms < 5 {
            metrics.queries_under_5ms += 1;
        }
        
        metrics.min_latency_ms = metrics.min_latency_ms.min(latency_ms);
        metrics.max_latency_ms = metrics.max_latency_ms.max(latency_ms);
    }
    
    async fn record_error(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.errors += 1;
    }
    
    async fn record_timeout(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.timeouts += 1;
    }
    
    /// Get current metrics
    pub async fn get_metrics(&self) -> QueryMetrics {
        self.metrics.read().await.clone()
    }
}

impl Clone for ConcurrentQueryHandler {
    fn clone(&self) -> Self {
        Self {
            search_engine: self.search_engine.clone(),
            query_cache: self.query_cache.clone(),
            semaphore: self.semaphore.clone(),
            metrics: self.metrics.clone(),
            timeout_duration: self.timeout_duration,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueryMetrics {
    pub total_queries: u64,
    pub cache_hits: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub queries_under_5ms: u64,
    pub total_latency_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
}

impl QueryMetrics {
    pub fn average_latency_ms(&self) -> f64 {
        if self.total_queries > 0 {
            self.total_latency_ms as f64 / self.total_queries as f64
        } else {
            0.0
        }
    }
    
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_queries > 0 {
            (self.cache_hits as f64 / self.total_queries as f64) * 100.0
        } else {
            0.0
        }
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.total_queries > 0 {
            let successful = self.total_queries - self.errors - self.timeouts;
            (successful as f64 / self.total_queries as f64) * 100.0
        } else {
            0.0
        }
    }
}

impl std::fmt::Display for QueryMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Queries: {}, Cache: {:.1}%, Avg: {:.1}ms, <5ms: {:.1}%, Success: {:.1}%",
            self.total_queries,
            self.cache_hit_rate(),
            self.average_latency_ms(),
            (self.queries_under_5ms as f64 / self.total_queries.max(1) as f64) * 100.0,
            self.success_rate()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_concurrent_queries() {
        // Test concurrent query handling
    }
}
