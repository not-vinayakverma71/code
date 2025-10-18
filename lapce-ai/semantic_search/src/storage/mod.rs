// Storage module for compressed embeddings and memory-mapped files
pub mod mmap_storage;
pub mod hierarchical_cache;

pub use mmap_storage::{MmapStorage, ConcurrentMmapStorage, StorageStats};
pub use hierarchical_cache::{HierarchicalCache, CacheConfig, CacheStats};
