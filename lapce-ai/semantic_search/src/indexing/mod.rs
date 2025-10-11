//! Incremental indexing infrastructure (Phase B)
//!
//! This module provides stable ID-based caching and incremental update
//! capabilities for the semantic code search system.

pub mod stable_id_cache;
pub mod incremental_detector;
pub mod cached_embedder;
pub mod async_indexer;
pub mod indexing_metrics;

#[cfg(all(test, feature = "cst_ts"))]
mod integration_tests;

pub use stable_id_cache::{StableIdEmbeddingCache, CacheEntry};
pub use incremental_detector::{IncrementalDetector, ChangeSet};
pub use cached_embedder::{CachedEmbedder, EmbeddingModel, EmbeddingStats};
pub use async_indexer::{AsyncIndexer, IndexerConfig, IndexTask, TaskPriority, IndexResult};
pub use indexing_metrics::{MetricTimer, record_cache_hit_rate, record_speedup, record_changeset};
