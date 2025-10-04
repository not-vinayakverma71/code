/// Consolidated Codebase Search Tool
/// Merged from codebase_search_tool.rs and codebase_search_tool_impl.rs
/// Line-by-line translation of codebaseSearchTool.ts to Rust

use anyhow::{Result, anyhow, Context as _};
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

// Consolidated search params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseSearchParams {
    pub query: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseSearchResult {
    pub query: String,
    pub results: Vec<SearchResultItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultItem {
    pub file_path: String,
    pub score: f32,
    pub start_line: u32,
    pub end_line: u32,
    pub code_chunk: String,
}

pub struct CodebaseSearchTool {
    code_index_manager: Arc<RwLock<CodeIndexManager>>,
    workspace_path: PathBuf,
}

impl CodebaseSearchTool {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            code_index_manager: Arc::new(RwLock::new(CodeIndexManager::new())),
            workspace_path,
        }
    }
    
    /// Main search function - exact translation of codebaseSearchTool()
    pub async fn search(
        &self,
        params: SearchParams,
    ) -> Result<String> {
        let tool_name = "codebase_search";
        
        // Parameter extraction and validation (lines 29-36)
        let query = params.query.trim();
        if query.is_empty() {
            return Err(anyhow!("Missing required parameter: query"));
        }
        
        let directory_prefix = params.path.as_ref().map(|p| {
            Path::new(p).to_path_buf()
        });
        
        // Core logic (lines 66-85)
        let manager = self.code_index_manager.read().await;
        
        if !manager.is_feature_enabled() {
            return Err(anyhow!("Code Indexing is disabled in the settings."));
        }
        
        if !manager.is_feature_configured() {
            return Err(anyhow!("Code Indexing is not configured (Missing API Key or LanceDB URL)."));
        }
        
        // Search the index
        let search_results = manager.search_index(query, directory_prefix.as_deref()).await?;
        
        // Format results (lines 88-120)
        if search_results.is_empty() {
            return Ok(format!("No relevant code snippets found for the query: \"{}\"", query));
        }
        
        let mut json_result = SearchResultJson {
            query: query.to_string(),
            results: Vec::new(),
        };
        
        // Process each result (lines 107-120)
        for result in search_results {
            let relative_path = self.get_relative_path(&result.payload.file_path);
            
            json_result.results.push(SearchResultItem {
                file_path: relative_path,
                score: result.score,
                start_line: result.payload.start_line,
                end_line: result.payload.end_line,
                code_chunk: result.payload.code_chunk.trim().to_string(),
            });
        }
        
        // Format output for AI (lines 127-138)
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
    
    fn get_relative_path(&self, absolute_path: &str) -> String {
        Path::new(absolute_path)
            .strip_prefix(&self.workspace_path)
            .unwrap_or(Path::new(absolute_path))
            .to_string_lossy()
            .to_string()
    }
}

/// Mock CodeIndexManager for now - will be replaced with real LanceDB implementation
pub struct CodeIndexManager {
    enabled: bool,
    configured: bool,
}

impl CodeIndexManager {
    pub fn new() -> Self {
        Self {
            enabled: true,
            configured: true,
        }
    }
    
    pub fn is_feature_enabled(&self) -> bool {
        self.enabled
    }
    
    pub fn is_feature_configured(&self) -> bool {
        self.configured
    }
    
    pub async fn search_index(
        &self,
        query: &str,
        directory_prefix: Option<&Path>,
    ) -> Result<Vec<VectorStoreSearchResult>> {
        // This will be replaced with real LanceDB search
        Ok(vec![])
    }
}
