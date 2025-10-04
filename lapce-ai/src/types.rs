// Core types for the lapce-ai-rust library

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub limit: usize,
    pub filters: SearchFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    pub file_types: Option<Vec<String>>,
    pub directories: Option<Vec<PathBuf>>,
    pub exclude_patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: PathBuf,
    pub content: String,
    pub score: f32,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_files: usize,
    pub indexed_files: usize,
    pub total_chunks: usize,
    pub index_size_bytes: usize,
}
