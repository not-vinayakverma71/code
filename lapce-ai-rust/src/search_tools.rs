// EXACT TRANSLATION OF TypeScript SEARCH TOOLS TO RUST
// Following docs/06-SEMANTIC-SEARCH-LANCEDB.md requirement:
// "TRANSLATE LINE-BY-LINE FROM codebaseSearchTool.ts and searchFilesTool.ts"

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// TypeScript interface translation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreSearchResult {
    pub file_path: String,
    pub score: f32,
    pub content: String,
    pub start_line: i32,
    pub end_line: i32,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchToolParams {
    pub query: String,
    pub path: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchToolResponse {
    pub query: String,
    pub results: Vec<SearchResultFormatted>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultFormatted {
    pub file_path: String,
    pub score: f32,
    pub content: String,
    pub start_line: i32,
    pub end_line: i32,
}

// Direct translation of codebaseSearchTool function
pub async fn codebase_search_tool(
    params: SearchToolParams,
    search_engine: &crate::semantic_engine::SemanticSearchEngine,
) -> Result<SearchToolResponse> {
    // Line-by-line translation from TypeScript:
    
    // Extract and validate parameters (lines 29-37 in TS)
    let query = params.query;
    let directory_prefix = params.path.map(|p| Path::new(&p).to_path_buf());
    
    // Validate query exists (lines 51-55 in TS)
    if query.is_empty() {
        return Err(anyhow!("Query parameter is required"));
    }
    
    // Prepare filters if path is specified
    let filters = directory_prefix.map(|path| {
        crate::semantic_engine::SearchFilters {
            language: None,
            path_pattern: Some(path.to_string_lossy().to_string()),
            min_score: None,
        }
    });
    
    // Core logic (lines 66-91 in TS)
    // Call search engine (equivalent to manager.searchIndex)
    let search_results = search_engine
        .search(&query, params.limit.unwrap_or(10), filters)
        .await?;
    
    // Format results (lines 88-91 in TS)
    if search_results.is_empty() {
        return Ok(SearchToolResponse {
            query: query.clone(),
            results: vec![],
        });
    }
    
    // Convert to formatted results (lines 93-145 in TS)
    let formatted_results: Vec<SearchResultFormatted> = search_results
        .into_iter()
        .map(|result| SearchResultFormatted {
            file_path: result.path,
            score: result.score,
            content: result.content,
            start_line: result.start_line,
            end_line: result.end_line,
        })
        .collect();
    
    Ok(SearchToolResponse {
        query,
        results: formatted_results,
    })
}

// Ranking algorithm from TypeScript (exact translation)
pub fn rank_search_results(
    results: Vec<VectorStoreSearchResult>,
    query: &str,
) -> Vec<VectorStoreSearchResult> {
    let mut ranked = results;
    
    // Apply the same ranking logic as in TypeScript
    // 1. Sort by score (descending)
    ranked.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // 2. Boost results that have exact query match
    let query_lower = query.to_lowercase();
    for result in &mut ranked {
        if result.content.to_lowercase().contains(&query_lower) {
            result.score *= 1.2; // Boost by 20% for exact matches
        }
        
        // Additional boost for filename match
        if result.file_path.to_lowercase().contains(&query_lower) {
            result.score *= 1.1; // Additional 10% boost
        }
    }
    
    // Re-sort after boosting
    ranked.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    ranked
}

// File search tool (translation of searchFilesTool.ts)
pub async fn search_files_tool(
    pattern: &str,
    base_path: &Path,
    include_content: bool,
) -> Result<Vec<FileSearchResult>> {
    use walkdir::WalkDir;
    use std::fs;
    
    let mut results = Vec::new();
    let pattern_lower = pattern.to_lowercase();
    
    for entry in WalkDir::new(base_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Check if filename matches pattern
        if file_name.to_lowercase().contains(&pattern_lower) {
            let content = if include_content {
                fs::read_to_string(path).ok()
            } else {
                None
            };
            
            results.push(FileSearchResult {
                path: path.to_path_buf(),
                file_name: file_name.to_string(),
                content,
                size: entry.metadata().map(|m| m.len()).unwrap_or(0),
            });
        }
    }
    
    Ok(results)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
    pub path: PathBuf,
    pub file_name: String,
    pub content: Option<String>,
    pub size: u64,
}

// Result formatting exactly as in TypeScript
pub fn format_search_results_for_ai(results: &[SearchResultFormatted]) -> String {
    if results.is_empty() {
        return "No relevant code snippets found for the query.".to_string();
    }
    
    let mut output = String::new();
    output.push_str("Found the following relevant code snippets:\n\n");
    
    for (i, result) in results.iter().enumerate() {
        output.push_str(&format!(
            "{}. {} (score: {:.3})\n   Lines {}-{}\n```\n{}\n```\n\n",
            i + 1,
            result.file_path,
            result.score,
            result.start_line,
            result.end_line,
            result.content
        ));
    }
    
    output
}

// Cache key computation (same as TypeScript)
pub fn compute_cache_key(query: &str, filters: &Option<crate::semantic_engine::SearchFilters>) -> String {
    use blake3::Hasher;
    
    let mut hasher = Hasher::new();
    hasher.update(query.as_bytes());
    
    if let Some(f) = filters {
        hasher.update(format!("{:?}", f).as_bytes());
    }
    
    hasher.finalize().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ranking_algorithm() {
        let results = vec![
            VectorStoreSearchResult {
                file_path: "src/main.rs".to_string(),
                score: 0.8,
                content: "fn main() { println!(\"Hello\"); }".to_string(),
                start_line: 1,
                end_line: 3,
                metadata: None,
            },
            VectorStoreSearchResult {
                file_path: "src/search.rs".to_string(),
                score: 0.9,
                content: "fn search() { /* search implementation */ }".to_string(),
                start_line: 10,
                end_line: 20,
                metadata: None,
            },
        ];
        
        let ranked = rank_search_results(results, "search");
        
        // The search.rs file should rank higher due to name match
        assert!(ranked[0].file_path.contains("search.rs"));
        assert!(ranked[0].score > 0.9); // Should be boosted
    }
    
    #[test]
    fn test_format_for_ai() {
        let results = vec![
            SearchResultFormatted {
                file_path: "test.rs".to_string(),
                score: 0.95,
                content: "test content".to_string(),
                start_line: 1,
                end_line: 5,
            },
        ];
        
        let formatted = format_search_results_for_ai(&results);
        assert!(formatted.contains("test.rs"));
        assert!(formatted.contains("score: 0.950"));
        assert!(formatted.contains("Lines 1-5"));
    }
}
