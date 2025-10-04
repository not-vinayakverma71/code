// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

// Core search modules
pub mod semantic_search_engine;
pub mod search_metrics;
pub mod code_indexer;
pub mod incremental_indexer;
pub mod hybrid_search;
pub mod codebase_search_tool;
pub mod search_files_tool;

// Optimized production modules
pub mod true_index_persistence;
pub mod improved_cache;
pub mod fully_optimized_storage;

// Public exports
pub use semantic_search_engine::{SemanticSearchEngine, SearchConfig, SearchResult};
pub use search_metrics::SearchMetrics;
pub use code_indexer::CodeIndexer;
pub use incremental_indexer::IncrementalIndexer;
pub use hybrid_search::HybridSearcher;
pub use codebase_search_tool::{CodebaseSearchTool, VectorStoreSearchResult};
pub use search_files_tool::SearchFilesTool;
pub use fully_optimized_storage::{FullyOptimizedStorage, FullyOptimizedConfig};
pub use improved_cache::{ImprovedQueryCache, CacheStats};
pub use true_index_persistence::{TrueIndexPersistence, IndexState};
