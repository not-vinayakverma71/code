// Shared types for semantic_search crate
use serde::{Serialize, Deserialize};

/// Unified CodeBlock type used across parser and scanner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub file_path: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub segment_hash: String,
}

impl CodeBlock {
    pub fn new(
        file_path: String,
        content: String,
        start_line: usize,
        end_line: usize,
        segment_hash: String,
    ) -> Self {
        Self {
            file_path,
            content,
            start_line,
            end_line,
            segment_hash,
        }
    }
}
