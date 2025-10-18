// SearchFiles tool v2 - Production-grade ripgrep integration with streaming
// Simplified implementation using working grep APIs

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::streaming_v2::{UnifiedStreamEmitter, SearchMatch as StreamSearchMatch};
use crate::core::tools::xml_util::XmlParser;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::mpsc;
use grep::regex::RegexMatcher;
use grep::searcher::{BinaryDetection, SearcherBuilder};
use grep::searcher::sinks::UTF8;
use ignore::WalkBuilder;

// Search configuration
const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
const DEFAULT_MAX_RESULTS: usize = 10000;

pub struct SearchFilesToolV2 {
    emitter: Arc<UnifiedStreamEmitter>,
}

impl SearchFilesToolV2 {
    pub fn new(emitter: Arc<UnifiedStreamEmitter>) -> Self {
        Self { emitter }
    }
}

#[async_trait]
impl Tool for SearchFilesToolV2 {
    fn name(&self) -> &'static str {
        "searchFiles"
    }
    
    fn description(&self) -> &'static str {
        "Search files using ripgrep with streaming results, ignore handling, and performance optimization"
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        // Parse XML arguments
        let parser = XmlParser::new();
        let parsed = parser.parse(args.as_str().ok_or_else(|| {
            ToolError::InvalidArguments("Expected XML string".to_string())
        })?).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        
        let tool_data = super::extract_tool_data(&parsed);
        
        // Extract search parameters
        let query = tool_data["query"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' argument".to_string()))?;
            
        let path = tool_data.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
            
        let is_regex = tool_data.get("isRegex")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let case_sensitive = tool_data.get("caseSensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
        let whole_word = tool_data.get("wholeWord")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let max_results = tool_data.get("maxResults")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_MAX_RESULTS);
            
        let max_file_size = tool_data.get("maxFileSize")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_MAX_FILE_SIZE);
            
        let follow_symlinks = tool_data.get("followSymlinks")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let respect_gitignore = tool_data.get("respectGitignore")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
        let streaming = tool_data.get("streaming")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        // Resolve search path
        let search_path = context.resolve_path(path);
        
        // Check .rooignore
        if !context.is_path_allowed(&search_path) {
            return Err(ToolError::RooIgnoreBlocked(format!(
                "Path '{}' is blocked by .rooignore",
                search_path.display()
            )));
        }
        
        // Ensure path is within workspace
        let safe_path = super::ensure_workspace_path(&context.workspace, &search_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Build config
        let config = SearchConfig {
            query: query.to_string(),
            path: safe_path,
            is_regex,
            case_sensitive,
            whole_word,
            max_results,
            max_file_size,
            follow_symlinks,
            respect_gitignore,
            streaming,
        };
        
        // Execute search
        let search_id = format!("search_{}", uuid::Uuid::new_v4());
        let results = self.execute_search(config).await?;
        
        Ok(ToolOutput::success(json!({
            "query": query,
            "path": path,
            "searchId": search_id,
            "results": results,
            "streaming": streaming,
        })))
    }
}

#[derive(Clone)]
struct SearchConfig {
    query: String,
    path: PathBuf,
    is_regex: bool,
    case_sensitive: bool,
    whole_word: bool,
    max_results: usize,
    max_file_size: u64,
    follow_symlinks: bool,
    respect_gitignore: bool,
    streaming: bool,
}

impl SearchFilesToolV2 {
    async fn execute_search(&self, config: SearchConfig) -> Result<Value, ToolError> {
        let start = Instant::now();
        let total_matches = Arc::new(AtomicUsize::new(0));
        let files_searched = Arc::new(AtomicUsize::new(0));
        let files_with_matches = Arc::new(AtomicUsize::new(0));
        
        // Build pattern
        let mut pattern = config.query.clone();
        if config.whole_word && !config.is_regex {
            pattern = format!(r"\b{}\b", regex::escape(&pattern));
        }
        if !config.case_sensitive && !pattern.starts_with("(?i)") {
            pattern = format!("(?i){}", pattern);
        }
        
        // Build matcher
        let matcher = RegexMatcher::new(&pattern)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid pattern: {}", e)))?;
        
        // Build searcher
        let searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\0'))
            .line_number(true)
            .build();
        
        // Build walker
        let mut walker_builder = WalkBuilder::new(&config.path);
        walker_builder
            .follow_links(config.follow_symlinks)
            .standard_filters(config.respect_gitignore)
            .max_filesize(Some(config.max_file_size))
            .threads(num_cpus::get());
        
        let walker = walker_builder.build();
        
        // Collect results
        let (tx, mut rx) = mpsc::channel(100);
        let max_results = config.max_results;
        let total_matches_clone = total_matches.clone();
        let files_searched_clone = files_searched.clone();
        let files_with_matches_clone = files_with_matches.clone();
        
        // Spawn search task
        tokio::task::spawn_blocking(move || {
            for entry in walker {
                if let Ok(entry) = entry {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        files_searched_clone.fetch_add(1, Ordering::Relaxed);
                        
                        let mut local_matches = Vec::new();
                        let mut searcher = searcher.clone();
                        
                        let _ = searcher.search_path(
                            &matcher,
                            entry.path(),
                            UTF8(|line_num, line| {
                                if local_matches.len() < max_results {
                                    local_matches.push(SearchMatch {
                                        file: entry.path().to_path_buf(),
                                        line_number: line_num,
                                        line_text: line.to_string(),
                                    });
                                }
                                Ok(true)
                            })
                        );
                        
                        if !local_matches.is_empty() {
                            files_with_matches_clone.fetch_add(1, Ordering::Relaxed);
                            total_matches_clone.fetch_add(local_matches.len(), Ordering::Relaxed);
                            
                            for m in local_matches {
                                if tx.blocking_send(m).is_err() {
                                    break;
                                }
                            }
                        }
                        
                        if total_matches_clone.load(Ordering::Relaxed) >= max_results {
                            break;
                        }
                    }
                }
            }
        });
        
        // Collect results
        let mut all_matches = Vec::new();
        while let Some(result) = rx.recv().await {
            all_matches.push(result);
            if all_matches.len() >= config.max_results {
                break;
            }
        }
        
        let duration = start.elapsed();
        
        Ok(json!({
            "matches": all_matches,
            "totalMatches": total_matches.load(Ordering::Relaxed),
            "filesSearched": files_searched.load(Ordering::Relaxed),
            "filesWithMatches": files_with_matches.load(Ordering::Relaxed),
            "duration": format!("{:.2}s", duration.as_secs_f64()),
            "streaming": config.streaming,
        }))
    }
}

#[derive(Clone, Debug, serde::Serialize)]
struct SearchMatch {
    file: PathBuf,
    line_number: u64,
    line_text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use crate::core::tools::streaming_v2::BackpressureConfig;
    
    #[tokio::test]
    async fn test_search_basic() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        fs::write(temp_dir.path().join("file1.txt"), "Hello world\nTest line\nHello again").unwrap();
        fs::write(temp_dir.path().join("file2.rs"), "fn hello() {}\nfn test() {}").unwrap();
        
        let emitter = Arc::new(UnifiedStreamEmitter::new(BackpressureConfig::default()));
        let tool = SearchFilesToolV2::new(emitter);
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <query>hello</query>
                <path>.</path>
                <caseSensitive>false</caseSensitive>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        // Check we found matches
        let data = &result.result;
        assert!(data["filesWithMatches"].as_u64().unwrap() >= 1);
    }
}
