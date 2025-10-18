// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

pub mod parser;
pub mod scanner;
pub mod file_watcher;
pub mod cst_to_ast_pipeline;
pub mod language_registry;
pub mod language_transformers;
pub mod unified_language_detection;
pub mod lapce_integration;
pub mod native_file_watcher;

#[cfg(feature = "cst_ts")]
pub mod cst_cache_integration;

#[cfg(feature = "cst_ts")]
pub use cst_cache_integration::{CstCache, CstCacheConfig, CstCacheStats, CachedCstParser};
