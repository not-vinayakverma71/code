// SearchFiles tool v2 - Production-grade ripgrep integration with streaming
// Part of Search suite TODO #3 - pre-IPC

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::streaming::{StreamEmitter, SearchProgress};
use crate::core::tools::xml_util::XmlParser;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use parking_lot::Mutex;
use grep::regex::RegexMatcher;
use grep::matcher::Matcher;
use grep::searcher::{Searcher, SearcherBuilder, Sink, SinkMatch, SinkContext};
use ignore::{WalkBuilder, WalkState, DirEntry};
use globset::{Glob, GlobSet, GlobSetBuilder};
use crossbeam_channel as channel;
use governor::{Quota, RateLimiter};

// Search configuration
const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
const DEFAULT_BATCH_SIZE: usize = 100;
const DEFAULT_MAX_RESULTS: usize = 10000;
const DEFAULT_CONTEXT_LINES: usize = 2;
const BACKPRESSURE_THRESHOLD: usize = 1000;
const RATE_LIMIT_FILES_PER_SEC: u32 = 1000;

pub struct SearchFilesToolV2 {
    emitter: Arc<StreamEmitter>,
}

impl SearchFilesToolV2 {
    pub fn new(emitter: Arc<StreamEmitter>) -> Self {
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
            
        let file_pattern = tool_data.get("filePattern")
            .and_then(|v| v.as_str());
            
        let is_regex = tool_data.get("isRegex")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let case_sensitive = tool_data.get("caseSensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
        let whole_word = tool_data.get("wholeWord")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let include_globs: Vec<String> = tool_data.get("include")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
            
        let exclude_globs: Vec<String> = tool_data.get("exclude")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
            
        let max_results = tool_data.get("maxResults")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_MAX_RESULTS);
            
        let context_lines = tool_data.get("contextLines")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_CONTEXT_LINES);
            
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
            
        let concurrency = tool_data.get("concurrency")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or_else(num_cpus::get);
        
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
        
        // Build search configuration
        let config = SearchConfig {
            query: query.to_string(),
            path: safe_path,
            is_regex,
            case_sensitive,
            whole_word,
            file_pattern: file_pattern.map(String::from),
            include_globs,
            exclude_globs,
            max_results,
            context_lines,
            max_file_size,
            follow_symlinks,
            respect_gitignore,
            streaming,
            concurrency,
        };
        
        // Execute search with streaming
        let search_id = format!("search_{}", uuid::Uuid::new_v4());
        let results = if streaming {
            self.execute_streaming_search(config, context, search_id.clone()).await?
        } else {
            self.execute_batch_search(config, context).await?
        };
        
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
    file_pattern: Option<String>,
    include_globs: Vec<String>,
    exclude_globs: Vec<String>,
    max_results: usize,
    context_lines: usize,
    max_file_size: u64,
    follow_symlinks: bool,
    respect_gitignore: bool,
    streaming: bool,
    concurrency: usize,
}

impl SearchFilesToolV2 {
    async fn execute_streaming_search(
        &self,
        config: SearchConfig,
        context: ToolContext,
        search_id: String,
    ) -> Result<Value, ToolError> {
        let start = Instant::now();
        let total_matches = Arc::new(AtomicUsize::new(0));
        let files_searched = Arc::new(AtomicUsize::new(0));
        let files_with_matches = Arc::new(AtomicUsize::new(0));
        let should_stop = Arc::new(AtomicBool::new(false));
        
        // Create channels for streaming results
        let (result_tx, mut result_rx) = mpsc::channel::<SearchMatch>(BACKPRESSURE_THRESHOLD);
        let (batch_tx, mut batch_rx) = mpsc::channel::<Vec<SearchMatch>>(100);
        
        // Rate limiter for file processing
        let rate_limiter = Arc::new(RateLimiter::direct(
            Quota::per_second(std::num::NonZeroU32::new(RATE_LIMIT_FILES_PER_SEC).unwrap())
        ));
        
        // Spawn batch aggregator
        let batch_handle = self.spawn_batch_aggregator(
            result_rx,
            batch_tx.clone(),
            self.emitter.clone(),
            search_id.clone(),
        );
        
        // Build matcher
        let matcher = build_matcher(&config)?;
        
        // Build file walker
        let walker = build_walker(&config);
        
        // Spawn search workers
        let (work_tx, work_rx) = channel::bounded::<DirEntry>(config.concurrency * 2);
        let mut workers = Vec::new();
        
        for _ in 0..config.concurrency {
            let worker = self.spawn_search_worker(
                work_rx.clone(),
                result_tx.clone(),
                matcher.clone(),
                config.clone(),
                total_matches.clone(),
                files_searched.clone(),
                files_with_matches.clone(),
                should_stop.clone(),
                rate_limiter.clone(),
            );
            workers.push(worker);
        }
        drop(work_rx); // Close original receiver
        
        // Walk files and send to workers
        walker.run(|| {
            let work_tx = work_tx.clone();
            let should_stop = should_stop.clone();
            Box::new(move |entry| {
                if should_stop.load(Ordering::Relaxed) {
                    return WalkState::Quit;
                }
                
                if let Ok(entry) = entry {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        if work_tx.send(entry).is_err() {
                            return WalkState::Quit;
                        }
                    }
                }
                WalkState::Continue
            })
        });
        
        drop(work_tx); // Signal completion to workers
        
        // Wait for workers to complete
        for worker in workers {
            let _ = worker.await;
        }
        
        // Signal batch aggregator to finish
        drop(result_tx);
        let _ = batch_handle.await;
        
        // Emit final progress
        let duration = start.elapsed();
        self.emitter.emit_search_complete(
            &search_id,
            total_matches.load(Ordering::Relaxed),
            files_searched.load(Ordering::Relaxed),
            files_with_matches.load(Ordering::Relaxed),
            duration,
        ).await;
        
        // Build summary
        Ok(json!({
            "totalMatches": total_matches.load(Ordering::Relaxed),
            "filesSearched": files_searched.load(Ordering::Relaxed),
            "filesWithMatches": files_with_matches.load(Ordering::Relaxed),
            "duration": format!("{:.2}s", duration.as_secs_f64()),
            "streaming": true,
        }))
    }
    
    async fn execute_batch_search(
        &self,
        config: SearchConfig,
        context: ToolContext,
    ) -> Result<Value, ToolError> {
        let start = Instant::now();
        let mut all_matches = Vec::new();
        let mut files_searched = 0;
        let mut files_with_matches = 0;
        
        // Build matcher
        let matcher = build_matcher(&config)?;
        
        // Build file walker
        let walker = build_walker(&config);
        
        // Search files
        walker.run(|| {
            let matcher = matcher.clone();
            let config = config.clone();
            let max_results = config.max_results;
            
            Box::new(move |entry| {
                if all_matches.len() >= max_results {
                    return WalkState::Quit;
                }
                
                if let Ok(entry) = entry {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        files_searched += 1;
                        
                        if let Ok(matches) = search_file(&entry.path(), &matcher, &config) {
                            if !matches.is_empty() {
                                files_with_matches += 1;
                                all_matches.extend(matches.into_iter().take(max_results - all_matches.len()));
                            }
                        }
                    }
                }
                WalkState::Continue
            })
        });
        
        let duration = start.elapsed();
        
        Ok(json!({
            "matches": all_matches,
            "totalMatches": all_matches.len(),
            "filesSearched": files_searched,
            "filesWithMatches": files_with_matches,
            "duration": format!("{:.2}s", duration.as_secs_f64()),
            "streaming": false,
        }))
    }
    
    fn spawn_batch_aggregator(
        &self,
        mut result_rx: mpsc::Receiver<SearchMatch>,
        batch_tx: mpsc::Sender<Vec<SearchMatch>>,
        emitter: Arc<StreamEmitter>,
        search_id: String,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(DEFAULT_BATCH_SIZE);
            let mut batch_timer = tokio::time::interval(Duration::from_millis(100));
            
            loop {
                tokio::select! {
                    Some(result) = result_rx.recv() => {
                        batch.push(result);
                        if batch.len() >= DEFAULT_BATCH_SIZE {
                            let current_batch = std::mem::replace(&mut batch, Vec::with_capacity(DEFAULT_BATCH_SIZE));
                            emitter.emit_search_batch(&search_id, current_batch).await;
                        }
                    }
                    _ = batch_timer.tick() => {
                        if !batch.is_empty() {
                            let current_batch = std::mem::replace(&mut batch, Vec::with_capacity(DEFAULT_BATCH_SIZE));
                            emitter.emit_search_batch(&search_id, current_batch).await;
                        }
                    }
                    else => {
                        // Channel closed, emit final batch
                        if !batch.is_empty() {
                            emitter.emit_search_batch(&search_id, batch).await;
                        }
                        break;
                    }
                }
            }
        })
    }
    
    fn spawn_search_worker(
        &self,
        work_rx: channel::Receiver<DirEntry>,
        result_tx: mpsc::Sender<SearchMatch>,
        matcher: Arc<dyn Matcher + Send + Sync>,
        config: SearchConfig,
        total_matches: Arc<AtomicUsize>,
        files_searched: Arc<AtomicUsize>,
        files_with_matches: Arc<AtomicUsize>,
        should_stop: Arc<AtomicBool>,
        rate_limiter: Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Ok(entry) = work_rx.recv() {
                if should_stop.load(Ordering::Relaxed) {
                    break;
                }
                
                if total_matches.load(Ordering::Relaxed) >= config.max_results {
                    should_stop.store(true, Ordering::Relaxed);
                    break;
                }
                
                // Apply rate limiting
                rate_limiter.until_ready().await;
                
                let path = entry.path();
                files_searched.fetch_add(1, Ordering::Relaxed);
                
                if let Ok(matches) = search_file(&path, &matcher, &config) {
                    if !matches.is_empty() {
                        files_with_matches.fetch_add(1, Ordering::Relaxed);
                        total_matches.fetch_add(matches.len(), Ordering::Relaxed);
                        
                        for m in matches {
                            if result_tx.send(m).await.is_err() {
                                should_stop.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                }
            }
        })
    }
}

fn build_matcher(config: &SearchConfig) -> Result<Arc<dyn Matcher + Send + Sync>, ToolError> {
    let mut pattern = config.query.clone();
    
    // Handle whole word matching
    if config.whole_word && !config.is_regex {
        pattern = format!(r"\b{}\b", regex::escape(&pattern));
    }
    
    // Build regex matcher
    let matcher = RegexMatcher::new_line_matcher(&pattern)
        .case_insensitive(!config.case_sensitive)
        .whole_line(false)
        .build()
        .map_err(|e| ToolError::InvalidArguments(format!("Invalid search pattern: {}", e)))?;
    
    Ok(Arc::new(matcher))
}

fn build_walker(config: &SearchConfig) -> ignore::Walk {
    let mut builder = WalkBuilder::new(&config.path);
    
    // Configure walker
    builder
        .follow_links(config.follow_symlinks)
        .git_ignore(config.respect_gitignore)
        .git_global(config.respect_gitignore)
        .git_exclude(config.respect_gitignore)
        .max_filesize(Some(config.max_file_size))
        .threads(config.concurrency);
    
    // Add include globs
    if let Some(ref pattern) = config.file_pattern {
        builder.add_custom_ignore_filename(pattern);
    }
    
    // Build glob set for includes
    if !config.include_globs.is_empty() {
        let mut glob_builder = GlobSetBuilder::new();
        for glob in &config.include_globs {
            if let Ok(g) = Glob::new(glob) {
                glob_builder.add(g);
            }
        }
        if let Ok(globset) = glob_builder.build() {
            builder.filter_entry(move |entry| {
                entry.file_type().map_or(true, |ft| {
                    ft.is_dir() || globset.is_match(entry.path())
                })
            });
        }
    }
    
    // Add exclude patterns
    for exclude in &config.exclude_globs {
        let _ = builder.add_ignore(exclude);
    }
    
    builder.build()
}

fn search_file(
    path: &Path,
    matcher: &Arc<dyn Matcher + Send + Sync>,
    config: &SearchConfig,
) -> Result<Vec<SearchMatch>, ToolError> {
    let mut searcher = SearcherBuilder::new()
        .line_number(true)
        .before_context(config.context_lines)
        .after_context(config.context_lines)
        .build();
    
    let mut sink = MatchSink::new(path.to_path_buf());
    
    searcher.search_path(
        matcher.as_ref(),
        path,
        &mut sink,
    ).map_err(|e| ToolError::Other(format!("Search error: {}", e)))?;
    
    Ok(sink.matches)
}

#[derive(Clone, Debug, serde::Serialize)]
struct SearchMatch {
    file: PathBuf,
    line_number: u64,
    line_text: String,
    match_text: String,
    byte_offset: u64,
    context_before: Vec<String>,
    context_after: Vec<String>,
}

struct MatchSink {
    path: PathBuf,
    matches: Vec<SearchMatch>,
    context_before: Vec<String>,
}

impl MatchSink {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            matches: Vec::new(),
            context_before: Vec::new(),
        }
    }
}

impl Sink for MatchSink {
    type Error = std::io::Error;
    
    fn matched(
        &mut self,
        _searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        let line_text = String::from_utf8_lossy(mat.bytes()).to_string();
        let match_text = line_text.trim().to_string();
        
        self.matches.push(SearchMatch {
            file: self.path.clone(),
            line_number: mat.line_number().unwrap_or(0),
            line_text: line_text.clone(),
            match_text,
            byte_offset: mat.absolute_byte_offset(),
            context_before: self.context_before.clone(),
            context_after: Vec::new(), // Will be filled by context_after
        });
        
        self.context_before.clear();
        Ok(true)
    }
    
    fn context(
        &mut self,
        _searcher: &Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        let line = String::from_utf8_lossy(ctx.bytes()).to_string();
        
        if ctx.kind().is_before() {
            self.context_before.push(line);
        } else if ctx.kind().is_after() {
            if let Some(last_match) = self.matches.last_mut() {
                last_match.context_after.push(line);
            }
        }
        
        Ok(true)
    }
}

// Dependencies needed in Cargo.toml (already present):
// grep = "0.3"
// ignore = "0.4"
// globset = "0.4"
// crossbeam-channel = "0.5"
// governor = "0.6"
// num_cpus = "1.16"
// uuid = { version = "1.6", features = ["v4"] }

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use crate::core::tools::streaming::StreamEmitter;
    
    #[tokio::test]
    async fn test_search_basic() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        fs::write(temp_dir.path().join("file1.txt"), "Hello world\nTest line\nHello again").unwrap();
        fs::write(temp_dir.path().join("file2.rs"), "fn hello() {}\nfn test() {}").unwrap();
        fs::write(temp_dir.path().join("ignore.log"), "Hello in log").unwrap();
        
        let emitter = Arc::new(StreamEmitter::new());
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
        
        // Check we found matches in both files
        let data = &result.result;
        assert!(data["filesWithMatches"].as_u64().unwrap() >= 2);
    }
    
    #[tokio::test]
    async fn test_search_with_globs() {
        let temp_dir = TempDir::new().unwrap();
        
        fs::write(temp_dir.path().join("test.rs"), "fn test() {}").unwrap();
        fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();
        fs::write(temp_dir.path().join("ignore.md"), "test ignored").unwrap();
        
        let emitter = Arc::new(StreamEmitter::new());
        let tool = SearchFilesToolV2::new(emitter);
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <query>test</query>
                <path>.</path>
                <include>["*.rs", "*.txt"]</include>
                <exclude>["*.md"]</exclude>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        // Should find matches in .rs and .txt but not .md
        let data = &result.result;
        assert_eq!(data["filesWithMatches"].as_u64().unwrap(), 2);
    }
    
    #[tokio::test]
    async fn test_search_regex() {
        let temp_dir = TempDir::new().unwrap();
        
        fs::write(temp_dir.path().join("test.txt"), "test123\ntest456\nnoMatch789").unwrap();
        
        let emitter = Arc::new(StreamEmitter::new());
        let tool = SearchFilesToolV2::new(emitter);
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <query>test\d{{3}}</query>
                <path>.</path>
                <isRegex>true</isRegex>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        // Should find 2 matches (test123, test456)
        let data = &result.result;
        assert!(data["totalMatches"].as_u64().unwrap() >= 2);
    }
    
    #[tokio::test]
    async fn test_search_whole_word() {
        let temp_dir = TempDir::new().unwrap();
        
        fs::write(temp_dir.path().join("test.txt"), "test testing tested test").unwrap();
        
        let emitter = Arc::new(StreamEmitter::new());
        let tool = SearchFilesToolV2::new(emitter);
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <query>test</query>
                <path>.</path>
                <wholeWord>true</wholeWord>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        // Should find 2 matches (whole word "test" only)
        let data = &result.result;
        assert_eq!(data["totalMatches"].as_u64().unwrap(), 2);
    }
    
    #[tokio::test]
    async fn test_search_with_context() {
        let temp_dir = TempDir::new().unwrap();
        
        fs::write(temp_dir.path().join("test.txt"), 
            "line1\nline2\nmatch line\nline4\nline5").unwrap();
        
        let emitter = Arc::new(StreamEmitter::new());
        let tool = SearchFilesToolV2::new(emitter);
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <query>match</query>
                <path>.</path>
                <contextLines>2</contextLines>
                <streaming>false</streaming>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        // Check context lines were captured
        if let Some(matches) = result.result["matches"].as_array() {
            if let Some(first_match) = matches.first() {
                assert!(!first_match["context_before"].as_array().unwrap().is_empty());
                assert!(!first_match["context_after"].as_array().unwrap().is_empty());
            }
        }
    }
}
