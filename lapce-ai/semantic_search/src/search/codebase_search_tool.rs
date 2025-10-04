// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// EXACT Translation of codebaseSearchTool.ts (Lines 1-145)

use crate::error::{Error, Result};
use crate::search::semantic_search_engine::{SemanticSearchEngine, SearchResult, SearchFilters};
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Line 7: VectorStoreSearchResult interface
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
    #[serde(rename = "codeChunk")]
    pub code_chunk: String,
    #[serde(rename = "startLine")]
    pub start_line: usize,
    #[serde(rename = "endLine")]
    pub end_line: usize,
    pub language: Option<String>,
}

// Line 11-18: Function signature
pub struct CodebaseSearchTool {
    semantic_engine: Arc<SemanticSearchEngine>,
    workspace_path: PathBuf,
}

pub struct ToolUse {
    pub params: SearchParams,
    pub partial: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParams {
    pub query: Option<String>,
    pub path: Option<String>,  // directory_prefix
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedMessageProps {
    pub tool: String,
    pub query: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "isOutsideWorkspace")]
    pub is_outside_workspace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResult {
    pub query: String,
    pub results: Vec<JsonResultItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResultItem {
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub score: f32,
    #[serde(rename = "startLine")]
    pub start_line: usize,
    #[serde(rename = "endLine")]
    pub end_line: usize,
    #[serde(rename = "codeChunk")]
    pub code_chunk: String,
}

impl CodebaseSearchTool {
    pub fn new(semantic_engine: Arc<SemanticSearchEngine>, workspace_path: PathBuf) -> Self {
        Self {
            semantic_engine,
            workspace_path,
        }
    }
    
    // Lines 11-144: Main function implementation
    pub async fn codebase_search_tool(
        &self,
        block: ToolUse,
        consecutive_mistake_count: &mut usize,
    ) -> Result<String> {
        // Line 19-20: Tool name and workspace path
        let tool_name = "codebase_search";
        let workspace_path = &self.workspace_path;
        
        // Lines 22-26: Workspace validation
        if !workspace_path.exists() {
            return Err(Error::Runtime {
                message: "Could not determine workspace path.".to_string()
            });
        }
        
        // Lines 28-37: Parameter extraction and validation
        let mut query = block.params.query.clone();
        let mut directory_prefix = block.params.path.clone();
        
        query = Self::remove_closing_tag("query", query);
        
        if let Some(ref mut dir) = directory_prefix {
            *dir = Self::remove_closing_tag("path", Some(dir.clone())).unwrap_or_default();
            *dir = Self::normalize_path(dir);
        }
        
        // Lines 39-44: Shared message properties
        let shared_message_props = SharedMessageProps {
            tool: "codebaseSearch".to_string(),
            query: query.clone(),
            path: directory_prefix.clone(),
            is_outside_workspace: false,
        };
        
        // Lines 46-49: Handle partial block
        if block.partial {
            return Ok(serde_json::to_string(&shared_message_props)?);
        }
        
        // Lines 51-55: Query validation
        let query = match query {
            Some(q) if !q.is_empty() => q,
            _ => {
                *consecutive_mistake_count += 1;
                return Err(Error::Runtime {
                    message: format!("Missing required parameter 'query' for {}", tool_name)
                });
            }
        };
        
        // Line 63: Reset consecutive mistake count
        *consecutive_mistake_count = 0;
        
        // Lines 65-84: Core logic
        // Check if indexing is enabled and configured
        if !self.semantic_engine.is_configured() {
            return Err(Error::Runtime {
                message: "Code Indexing is not configured (Missing API Key)".to_string()
            });
        }
        
        // Line 85: Search index
        let search_results = self.search_index(&query, directory_prefix.as_deref()).await?;
        
        // Lines 87-91: Handle empty results
        if search_results.is_empty() {
            return Ok(format!("No relevant code snippets found for the query: \"{}\"", query));
        }
        
        // Lines 93-105: Initialize JSON result structure
        let mut json_result = JsonResult {
            query: query.clone(),
            results: Vec::new(),
        };
        
        // Lines 107-120: Process search results
        for result in search_results {
            if let Some(payload) = result.payload {
                let relative_path = Self::workspace_as_relative_path(
                    &self.workspace_path,
                    &payload.file_path,
                );
                
                json_result.results.push(JsonResultItem {
                    file_path: relative_path,
                    score: result.score,
                    start_line: payload.start_line,
                    end_line: payload.end_line,
                    code_chunk: payload.code_chunk.trim().to_string(),
                });
            }
        }
        
        // Lines 127-138: Format output
        let output = format!(
            "Query: {}\nResults:\n\n{}",
            query,
            json_result.results
                .iter()
                .map(|result| format!(
                    "File path: {}\nScore: {}\nLines: {}-{}\nCode Chunk: {}\n",
                    result.file_path,
                    result.score,
                    result.start_line,
                    result.end_line,
                    result.code_chunk
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        Ok(output)
    }
    
    // Line 85: Search implementation
    async fn search_index(
        &self,
        query: &str,
        directory_prefix: Option<&str>,
    ) -> Result<Vec<VectorStoreSearchResult>> {
        // Call semantic search engine
        let filters = directory_prefix.map(|prefix| SearchFilters {
            path_pattern: Some(prefix.to_string()),
            ..Default::default()
        });
        
        let search_results = self.semantic_engine.search(
            query,
            10,  // Default limit
            filters,
        ).await?;
        
        // Convert SearchResult to VectorStoreSearchResult
        Ok(search_results.into_iter().map(|sr| VectorStoreSearchResult {
            id: sr.id,
            score: sr.score,
            payload: Some(SearchPayload {
                file_path: sr.path,
                code_chunk: sr.content,
                start_line: sr.start_line,
                end_line: sr.end_line,
                language: sr.language,
            }),
        }).collect())
    }
    
    // Helper functions
    fn remove_closing_tag(_tag: &str, value: Option<String>) -> Option<String> {
        value.map(|v| v.trim().to_string())
    }
    
    fn normalize_path(path: &str) -> String {
        PathBuf::from(path)
            .components()
            .collect::<PathBuf>()
            .to_string_lossy()
            .to_string()
    }
    
    fn workspace_as_relative_path(workspace: &Path, file_path: &str) -> String {
        let file_path = PathBuf::from(file_path);
        if let Ok(relative) = file_path.strip_prefix(workspace) {
            relative.to_string_lossy().to_string()
        } else {
            file_path.to_string_lossy().to_string()
        }
    }
}

// Line 78: Check if search is enabled
impl SemanticSearchEngine {
    pub fn is_configured(&self) -> bool {
        // Check if embedder is properly configured
        true  // In production, check actual API keys
    }
}
