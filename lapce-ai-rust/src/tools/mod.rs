// Direct translation of Codex tools

use serde::{Deserialize, Serialize};

/// Direct translation of VectorStoreSearchResult from Codex
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreSearchResult {
    pub id: String,
    pub score: f32,
    pub payload: Option<SearchPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPayload {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "startLine")]
    pub start_line: usize,
    #[serde(rename = "endLine")]
    pub end_line: usize,
    #[serde(rename = "codeChunk")]
    pub code_chunk: String,
}
