// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Code Indexer Implementation - Lines 190-287 from doc

use crate::error::{Error, Result};
use crate::processors::parser::CodeParser;
use crate::search::semantic_search_engine::{SemanticSearchEngine, ChunkMetadata, IndexStats};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use walkdir::WalkDir;

/// Code indexer for repository scanning - Lines 192-197 from doc
pub struct CodeIndexer {
    search_engine: Arc<SemanticSearchEngine>,
    parser: Arc<CodeParser>,
    batch_size: usize,
    index_queue: Arc<Mutex<VecDeque<IndexTask>>>,
}

#[derive(Debug, Clone)]
struct IndexTask {
    path: PathBuf,
    action: IndexAction,
}

#[derive(Debug, Clone)]
pub enum IndexAction {
    Add,
    Update,
    Delete,
}

impl CodeIndexer {
    /// Create new code indexer
    pub fn new(search_engine: Arc<SemanticSearchEngine>) -> Self {
        Self {
            search_engine,
            parser: Arc::new(CodeParser::new()),
            batch_size: 100,
            index_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    
    /// Set batch size for processing
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }
    
    /// Index entire repository - Lines 200-216 from doc
    pub async fn index_repository(&self, repo_path: &Path) -> Result<IndexStats> {
        let start = Instant::now();
        let mut stats = IndexStats::default();
        
        // Walk repository files
        let files = self.collect_files(repo_path).await?;
        log::info!("Found {} files to index", files.len());
        
        // Process in batches
        for chunk in files.chunks(self.batch_size) {
            let batch_results = self.process_batch(chunk).await?;
            stats.files_indexed += batch_results.files_indexed;
            stats.chunks_created += batch_results.chunks_created;
        }
        
        // Optimize index after bulk indexing
        self.search_engine.optimize_index().await?;
        
        stats.time_elapsed = start.elapsed();
        log::info!(
            "Indexed {} files, created {} chunks in {:?}",
            stats.files_indexed, stats.chunks_created, stats.time_elapsed
        );
        
        Ok(stats)
    }
    
    /// Collect files from repository
    async fn collect_files(&self, repo_path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        // Walk directory tree
        for entry in WalkDir::new(repo_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Skip directories and non-files
            if !path.is_file() {
                continue;
            }
            
            // Skip hidden files and directories
            if path.components().any(|c| {
                c.as_os_str()
                    .to_str()
                    .map_or(false, |s| s.starts_with('.') && s != ".")
            }) {
                continue;
            }
            
            // Check if file has a supported extension
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if is_supported_extension(&ext_str) {
                    files.push(path.to_path_buf());
                }
            }
        }
        
        Ok(files)
    }
    
    /// Process batch of files - Lines 218-245 from doc
    async fn process_batch(&self, files: &[PathBuf]) -> Result<IndexStats> {
        let mut embeddings = Vec::new();
        let mut metadata = Vec::new();
        
        for file in files {
            // Parse code chunks
            let chunks = self.parse_file(file).await?;
            
            for chunk in chunks {
                // Generate embedding using the search engine's embedder
                let embedding_response = self.search_engine
                    .embedder
                    .create_embeddings(vec![chunk.content.clone()], None)
                    .await?;
                    
                if let Some(embedding) = embedding_response.embeddings.into_iter().next() {
                    embeddings.push(embedding);
                    metadata.push(chunk);
                }
            }
        }
        
        // Batch insert into LanceDB
        if !embeddings.is_empty() {
            self.search_engine.batch_insert(embeddings, metadata).await
        } else {
            Ok(IndexStats::default())
        }
    }
    
    /// Parse file into chunks
    async fn parse_file(&self, file: &Path) -> Result<Vec<ChunkMetadata>> {
        // Read file content
        let content = tokio::fs::read_to_string(file).await.map_err(|e| Error::Runtime {
            message: format!("Failed to read file {}: {}", file.display(), e)
        })?;
        
        // Use the parser to get code blocks
        let blocks = self.parser.parse_file(file, Some(&content), None).await?;
        
        // Convert to ChunkMetadata
        let chunks = blocks.into_iter().map(|block| {
            ChunkMetadata {
                path: file.to_path_buf(),
                content: block.content,
                start_line: block.start_line,
                end_line: block.end_line,
                language: detect_language(file),
                metadata: std::collections::HashMap::new(),
            }
        }).collect();
        
        Ok(chunks)
    }
    
    /// Queue file for indexing
    pub async fn queue_file(&self, path: PathBuf, action: IndexAction) {
        let mut queue = self.index_queue.lock().await;
        queue.push_back(IndexTask { path, action });
    }
    
    /// Process queued index tasks
    pub async fn process_queue(&self) -> Result<IndexStats> {
        let mut stats = IndexStats::default();
        let mut tasks = Vec::new();
        
        {
            let mut queue = self.index_queue.lock().await;
            while let Some(task) = queue.pop_front() {
                tasks.push(task);
                if tasks.len() >= self.batch_size {
                    break;
                }
            }
        }
        
        for task in tasks {
            match task.action {
                IndexAction::Add | IndexAction::Update => {
                    // Delete old entries if updating
                    if matches!(task.action, IndexAction::Update) {
                        self.search_engine.delete_by_path(&task.path).await?;
                    }
                    
                    // Index the file
                    let batch_stats = self.process_batch(&[task.path]).await?;
                    stats.files_indexed += batch_stats.files_indexed;
                    stats.chunks_created += batch_stats.chunks_created;
                }
                IndexAction::Delete => {
                    self.search_engine.delete_by_path(&task.path).await?;
                }
            }
        }
        
        Ok(stats)
    }
    
    /// Get queue size
    pub async fn queue_size(&self) -> usize {
        self.index_queue.lock().await.len()
    }
}

/// Check if file extension is supported
fn is_supported_extension(ext: &str) -> bool {
    const SUPPORTED_EXTENSIONS: &[&str] = &[
        "rs", "py", "js", "ts", "jsx", "tsx", "java", "c", "cpp", "h", "hpp",
        "cs", "go", "rb", "php", "swift", "kt", "scala", "r", "m", "mm",
        "sh", "bash", "zsh", "fish", "ps1", "yaml", "yml", "json", "toml",
        "xml", "html", "css", "scss", "sass", "less", "sql", "graphql",
        "vue", "svelte", "md", "markdown",
    ];
    
    SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

/// Detect programming language from file extension
fn detect_language(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            match ext.to_lowercase().as_str() {
                "rs" => "rust",
                "py" => "python",
                "js" | "jsx" => "javascript",
                "ts" | "tsx" => "typescript",
                "java" => "java",
                "c" | "h" => "c",
                "cpp" | "hpp" | "cc" | "cxx" => "cpp",
                "cs" => "csharp",
                "go" => "go",
                "rb" => "ruby",
                "php" => "php",
                "swift" => "swift",
                "kt" | "kts" => "kotlin",
                "scala" => "scala",
                "r" => "r",
                "m" | "mm" => "objc",
                "sh" | "bash" | "zsh" | "fish" => "shell",
                "ps1" => "powershell",
                "yaml" | "yml" => "yaml",
                "json" => "json",
                "toml" => "toml",
                "xml" => "xml",
                "html" => "html",
                "css" | "scss" | "sass" | "less" => "css",
                "sql" => "sql",
                "graphql" | "gql" => "graphql",
                "vue" => "vue",
                "svelte" => "svelte",
                "md" | "markdown" => "markdown",
                _ => "text",
            }.to_string()
        })
}
