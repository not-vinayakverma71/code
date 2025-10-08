/// Cache Architecture V3 - EXACT Implementation from docs/09-CACHE-ARCHITECTURE.md
/// Multi-Layer Intelligent Caching with LRU, Query, and Embedding Cache
/// Target: 3MB Footprint, 85%+ Hit Rate, <1ms Latency

pub mod cache_system;
pub mod l1_cache;
pub mod l2_cache;
pub mod l3_cache;
pub mod access_counter;
pub mod count_min_sketch;
pub mod compression_strategy;
pub mod cache_coordinator;
pub mod promotion_policy;
pub mod query_cache;
pub mod embedding_cache;
pub mod cache_warmer;
pub mod cache_metrics;
pub mod bloom_filter;
pub mod types;

pub use cache_system::CacheSystem;
pub use types::{CacheKey, CacheValue, CacheLevel};
pub use cache_metrics::CacheMetrics;
