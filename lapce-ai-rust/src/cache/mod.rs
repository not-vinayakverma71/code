/// Cache Architecture V3 - EXACT Implementation from docs/09-CACHE-ARCHITECTURE.md
/// Multi-Layer Intelligent Caching with LRU, Query, and Embedding Cache
/// Target: 3MB Footprint, 85%+ Hit Rate, <1ms Latency

pub mod access_counter;
pub mod access_history;
pub mod adaptive_cache;
pub mod aes_encryption;
pub mod bloom_filter;
pub mod real_l1_cache;
pub mod real_l2_cache;
pub mod real_l3_cache;
pub mod cache_coordinator;
pub mod cache_metrics;
pub mod cache_system;
pub mod cache_warmer;
pub mod compression_strategy;
pub mod count_min_sketch;
pub mod embedding_cache;
pub mod l1_cache;
pub mod l2_cache;
pub mod l3_cache;
pub mod optimized_hybrid;
pub mod promotion_policy;
pub mod query_cache;
pub mod query_hasher;
pub mod serializer;
pub mod types;
pub mod final_cache;

// Use the final consolidated implementation
pub use final_cache::{Cache as CacheSystem, CacheV3};
pub use types::{CacheKey, CacheValue, QueryResult, CacheLevel, CacheConfig};
pub use query_cache::QueryCache;
pub use cache_metrics::CacheMetrics;
