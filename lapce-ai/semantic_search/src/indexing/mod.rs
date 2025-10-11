//! Incremental indexing infrastructure (Phase B)
//!
//! This module provides stable ID-based caching and incremental update
//! capabilities for the semantic code search system.

pub mod stable_id_cache;

pub use stable_id_cache::{StableIdEmbeddingCache, CacheEntry};
