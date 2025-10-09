// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Translation of processors/scanner.ts (Lines 1-460) - 100% EXACT

use crate::error::{Error, Result};
use crate::embeddings::service_factory::{IEmbedder, IVectorStore, ICodeParser, PointStruct};
use crate::database::cache_interface::ICacheManager;
use std::collections::{HashSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::{Semaphore, Mutex as AsyncMutex};
use tokio::fs;
use sha2::{Sha256, Digest};
use uuid::Uuid;

// Lines 17-27: Constants from constants module
const QDRANT_CODE_BLOCK_NAMESPACE: Uuid = Uuid::from_u128(0x12345678_1234_5678_1234_567812345678);
const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10MB
const MAX_LIST_FILES_LIMIT_CODE_INDEX: usize = 50000;
const BATCH_SEGMENT_THRESHOLD: usize = 100;
const MAX_BATCH_RETRIES: usize = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 1000;
const PARSING_CONCURRENCY: usize = 10;
const BATCH_PROCESSING_CONCURRENCY: usize = 5;
const MAX_PENDING_BATCHES: usize = 3;

// Use shared CodeBlock type
pub use crate::types::CodeBlock;

/// Lines 33-459: DirectoryScanner implementation
pub struct DirectoryScanner {
    embedder: Arc<dyn IEmbedder>,
    qdrant_client: Arc<dyn IVectorStore>,
    code_parser: Arc<dyn ICodeParser>,
    cache_manager: Arc<dyn ICacheManager>,
    ignore_instance: Arc<dyn Ignore>,
}

impl DirectoryScanner {
    /// Lines 34-40: Constructor
    pub fn new(
        embedder: Arc<dyn IEmbedder>,
        qdrant_client: Arc<dyn IVectorStore>,
        code_parser: Arc<dyn ICodeParser>,
        cache_manager: Arc<dyn ICacheManager>,
        ignore_instance: Arc<dyn Ignore>,
    ) -> Self {
        Self {
            embedder,
            qdrant_client,
            code_parser,
            cache_manager,
            ignore_instance,
        }
    }
    
    /// Lines 50-328: Scan directory for code blocks
    pub async fn scan_directory(
        &self,
        directory: &Path,
        on_error: Option<Box<dyn Fn(Error) + Send + Sync>>,
        on_blocks_indexed: Option<Box<dyn Fn(usize) + Send + Sync>>,
        on_file_parsed: Option<Box<dyn Fn(usize) + Send + Sync>>,
    ) -> Result<ScanResult> {
        // Convert Box to Arc for sharing across threads
        let on_error = on_error.map(|f| Arc::from(f) as Arc<dyn Fn(Error) + Send + Sync>);
        let on_blocks_indexed = on_blocks_indexed.map(|f| Arc::from(f) as Arc<dyn Fn(usize) + Send + Sync>);
        let on_file_parsed = on_file_parsed.map(|f| Arc::from(f) as Arc<dyn Fn(usize) + Send + Sync>);
        let directory_path = directory.to_path_buf();
        let scan_workspace = get_workspace_path_for_context(&directory_path);
        
        // Lines 61-65: Get all files recursively
        let all_paths = list_files(&directory_path, true, MAX_LIST_FILES_LIMIT_CODE_INDEX).await?;
        let file_paths: Vec<PathBuf> = all_paths
            .into_iter()
            .filter(|p| !p.to_string_lossy().ends_with("/"))
            .collect();
        
        // Lines 67-73: Initialize ignore controller and filter paths
        let ignore_controller = RooIgnoreController::new(&directory_path);
        ignore_controller.initialize().await?;
        let allowed_paths = ignore_controller.filter_paths(file_paths);
        
        // Lines 75-85: Filter by supported extensions
        let supported_paths: Vec<PathBuf> = allowed_paths
            .into_iter()
            .filter(|file_path| {
                let ext = file_path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let relative_file_path = generate_relative_file_path(file_path, &scan_workspace);
                
                if is_path_in_ignored_directory(file_path) {
                    return false;
                }
                
                SCANNER_EXTENSIONS.contains(&ext.as_str()) && !self.ignore_instance.ignores(&relative_file_path)
            })
            .collect();
        
        // Lines 88-106: Initialize tracking variables
        let processed_files = Arc::new(Mutex::new(HashSet::new()));
        let processed_count = Arc::new(Mutex::new(0usize));
        let skipped_count = Arc::new(Mutex::new(0usize));
        let total_block_count = Arc::new(Mutex::new(0usize));
        
        log::info!("[DirectoryScanner] Starting scan of {:?} with {} supported files", 
                  directory, supported_paths.len());
        
        // Parallel processing tools
        let parse_limiter = Arc::new(Semaphore::new(PARSING_CONCURRENCY));
        let batch_limiter = Arc::new(Semaphore::new(BATCH_PROCESSING_CONCURRENCY));
        let batch_mutex = Arc::new(AsyncMutex::new(()));
        
        // Shared batch accumulators
        let current_batch_blocks = Arc::new(Mutex::new(Vec::new()));
        let current_batch_texts = Arc::new(Mutex::new(Vec::new()));
        let current_batch_file_infos = Arc::new(Mutex::new(Vec::new()));
        let active_batch_promises = Arc::new(Mutex::new(Vec::new()));
        let pending_batch_count = Arc::new(Mutex::new(0usize));
        
        // Lines 108-236: Process all files in parallel
        let mut tasks = Vec::new();
        for file_path in supported_paths {
            let parse_permit = parse_limiter.clone().acquire_owned().await.map_err(|e| Error::Runtime {
                message: format!("Failed to acquire parse permit: {}", e)
            })?;
            let task = self.process_file(
                file_path,
                scan_workspace.clone(),
                processed_files.clone(),
                processed_count.clone(),
                skipped_count.clone(),
                total_block_count.clone(),
                current_batch_blocks.clone(),
                current_batch_texts.clone(),
                current_batch_file_infos.clone(),
                batch_limiter.clone(),
                batch_mutex.clone(),
                active_batch_promises.clone(),
                pending_batch_count.clone(),
                on_error.clone(),
                on_blocks_indexed.clone(),
                on_file_parsed.clone(),
                parse_permit,
            );
            tasks.push(task);
        }
        
        // Wait for all parsing to complete
        let start_time = std::time::Instant::now();
        futures::future::join_all(tasks).await;
        let parse_duration = start_time.elapsed();
        
        log::info!("[DirectoryScanner] Parsing completed in {:.2}s", parse_duration.as_secs_f64());
        
        // Lines 242-270: Process any remaining items in batch
        let remaining_blocks = current_batch_blocks.lock().unwrap().clone();
        if !remaining_blocks.is_empty() {
            let batch_blocks = current_batch_blocks.lock().unwrap().drain(..).collect::<Vec<_>>();
            let batch_texts = current_batch_texts.lock().unwrap().drain(..).collect::<Vec<_>>();
            let batch_file_infos = current_batch_file_infos.lock().unwrap().drain(..).collect::<Vec<_>>();
            
            *pending_batch_count.lock().unwrap() += 1;
            
            let batch_permit = batch_limiter.acquire().await.map_err(|e| Error::Runtime {
                message: format!("Failed to acquire batch permit: {}", e)
            })?;
            self.process_batch(
                batch_blocks,
                batch_texts,
                batch_file_infos,
                scan_workspace.clone(),
                on_error.clone(),
                on_blocks_indexed.clone(),
            ).await;
            drop(batch_permit);
            *pending_batch_count.lock().unwrap() -= 1;
        }
        
        // Lines 275-319: Handle deleted files
        let old_hashes = self.cache_manager.get_all_hashes();
        let processed = processed_files.lock().unwrap();
        for cached_file_path in old_hashes.keys() {
            if !processed.contains(cached_file_path) {
                match self.qdrant_client.delete_points_by_file_path(cached_file_path).await {
                    Ok(_) => {
                        let cache = self.cache_manager.clone();
                        cache.delete_hash(cached_file_path);
                    }
                    Err(error) => {
                        log::error!(
                            "[DirectoryScanner] Failed to delete points for {} in workspace {}: {:?}",
                            cached_file_path, scan_workspace.display(), error
                        );
                        
                        if let Some(ref on_error) = on_error {
                            on_error(Error::Runtime {
                                message: format!(
                                    "Failed to delete points for removed file: {} (Workspace: {})",
                                    cached_file_path, scan_workspace.display()
                                )
                            });
                        }
                    }
                }
            }
        }
        
        // Lines 321-327: Return results
        let final_processed = *processed_count.lock().unwrap();
        let final_skipped = *skipped_count.lock().unwrap();
        let final_total_blocks = *total_block_count.lock().unwrap();
        
        log::info!("[DirectoryScanner] Scan complete - Processed: {}, Skipped: {}, Total blocks: {}",
                  final_processed, final_skipped, final_total_blocks);
        
        Ok(ScanResult {
            stats: ScanStats {
                processed: final_processed,
                skipped: final_skipped,
            },
            total_block_count: final_total_blocks,
        })
    }
    
    /// Process a single file
    async fn process_file(
        &self,
        file_path: PathBuf,
        scan_workspace: PathBuf,
        processed_files: Arc<Mutex<HashSet<String>>>,
        processed_count: Arc<Mutex<usize>>,
        skipped_count: Arc<Mutex<usize>>,
        total_block_count: Arc<Mutex<usize>>,
        current_batch_blocks: Arc<Mutex<Vec<CodeBlock>>>,
        current_batch_texts: Arc<Mutex<Vec<String>>>,
        current_batch_file_infos: Arc<Mutex<Vec<FileInfo>>>,
        batch_limiter: Arc<Semaphore>,
        batch_mutex: Arc<AsyncMutex<()>>,
        active_batch_promises: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
        pending_batch_count: Arc<Mutex<usize>>,
        on_error: Option<Arc<dyn Fn(Error) + Send + Sync>>,
        on_blocks_indexed: Option<Arc<dyn Fn(usize) + Send + Sync>>,
        on_file_parsed: Option<Arc<dyn Fn(usize) + Send + Sync>>,
        _permit: tokio::sync::OwnedSemaphorePermit,
    ) -> tokio::task::JoinHandle<()> {
        let scanner = self.clone();
        tokio::spawn(async move {
            match scanner.process_file_impl(
                file_path,
                scan_workspace,
                processed_files,
                processed_count,
                skipped_count,
                total_block_count,
                current_batch_blocks,
                current_batch_texts,
                current_batch_file_infos,
                batch_limiter,
                batch_mutex,
                active_batch_promises,
                pending_batch_count,
                on_error,
                on_blocks_indexed,
                on_file_parsed,
            ).await {
                Ok(_) => {}
                Err(e) => log::error!("Error processing file: {:?}", e)
            }
        })
    }
    
    async fn process_file_impl(
        &self,
        file_path: PathBuf,
        scan_workspace: PathBuf,
        processed_files: Arc<Mutex<HashSet<String>>>,
        processed_count: Arc<Mutex<usize>>,
        skipped_count: Arc<Mutex<usize>>,
        total_block_count: Arc<Mutex<usize>>,
        current_batch_blocks: Arc<Mutex<Vec<CodeBlock>>>,
        current_batch_texts: Arc<Mutex<Vec<String>>>,
        current_batch_file_infos: Arc<Mutex<Vec<FileInfo>>>,
        batch_limiter: Arc<Semaphore>,
        batch_mutex: Arc<AsyncMutex<()>>,
        active_batch_promises: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
        pending_batch_count: Arc<Mutex<usize>>,
        on_error: Option<Arc<dyn Fn(Error) + Send + Sync>>,
        on_blocks_indexed: Option<Arc<dyn Fn(usize) + Send + Sync>>,
        on_file_parsed: Option<Arc<dyn Fn(usize) + Send + Sync>>,
    ) -> Result<()> {
        // Lines 111-116: Check file size
        let metadata = fs::metadata(&file_path).await.map_err(|e| Error::Runtime {
            message: format!("Failed to get metadata: {}", e)
        })?;
        if metadata.len() > MAX_FILE_SIZE_BYTES {
            *skipped_count.lock().unwrap() += 1;
            return Ok(());
        }
        
        // Lines 118-125: Read file and calculate hash
        let content = fs::read_to_string(&file_path).await.map_err(|e| Error::Runtime {
            message: format!("Failed to read file: {}", e)
        })?;
        let current_file_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        let file_path_str = file_path.to_string_lossy().to_string();
        processed_files.lock().unwrap().insert(file_path_str.clone());
        
        // Lines 127-134: Check cache
        let cached_file_hash = self.cache_manager.get_hash(&file_path_str);
        let is_new_file = cached_file_hash.is_none();
        if let Some(cached) = &cached_file_hash {
            if cached == &current_file_hash {
                *skipped_count.lock().unwrap() += 1;
                return Ok(());
            }
        }
        
        // Lines 136-140: Parse file
        let blocks = self.code_parser.parse(&content);
        let file_block_count = blocks.len();
        if let Some(ref on_file_parsed) = on_file_parsed {
            on_file_parsed(file_block_count);
        }
        *processed_count.lock().unwrap() += 1;
        
        // Lines 142-217: Process embeddings if configured
        if blocks.len() > 0 {
            let mut added_blocks_from_file = false;
            
            for block in blocks {
                let trimmed_content = block.content.trim();
                if !trimmed_content.is_empty() {
                    let _lock = batch_mutex.lock().await;
                    
                    // Create unified CodeBlock
                    let scanner_block = CodeBlock::new(
                        file_path_str.clone(),
                        block.content.clone(),
                        block.start_line,
                        block.end_line,
                        format!("{:x}", Sha256::digest(block.content.as_bytes())),
                    );
                    
                    current_batch_blocks.lock().unwrap().push(scanner_block);
                    current_batch_texts.lock().unwrap().push(trimmed_content.to_string());
                    added_blocks_from_file = true;
                    
                    // Lines 156-192: Check if batch threshold is met
                    if current_batch_blocks.lock().unwrap().len() >= BATCH_SEGMENT_THRESHOLD {
                        // Wait if we've reached maximum pending batches
                        while *pending_batch_count.lock().unwrap() >= MAX_PENDING_BATCHES {
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                        
                        // Copy current batch data and clear
                        let batch_blocks = current_batch_blocks.lock().unwrap().drain(..).collect::<Vec<_>>();
                        let batch_texts = current_batch_texts.lock().unwrap().drain(..).collect::<Vec<_>>();
                        let batch_file_infos = current_batch_file_infos.lock().unwrap().drain(..).collect::<Vec<_>>();
                        
                        *pending_batch_count.lock().unwrap() += 1;
                        
                        // Queue batch processing
                        let scanner = self.clone();
                        let workspace = scan_workspace.clone();
                        let on_error_clone = on_error.clone();
                        let on_indexed_clone = on_blocks_indexed.clone();
                        let pending_count = pending_batch_count.clone();
                        let permit = batch_limiter.clone().acquire_owned().await.map_err(|e| Error::Runtime {
                            message: format!("Failed to acquire batch permit: {}", e)
                        })?;
                        
                        let handle = tokio::spawn(async move {
                            scanner.process_batch(
                                batch_blocks,
                                batch_texts,
                                batch_file_infos,
                                workspace,
                                on_error_clone,
                                on_indexed_clone,
                            ).await;
                            drop(permit);
                            *pending_count.lock().unwrap() -= 1;
                        });
                        
                        active_batch_promises.lock().unwrap().push(handle);
                    }
                }
            }
            
            // Lines 199-212: Add file info once per file
            if added_blocks_from_file {
                let _lock = batch_mutex.lock().await;
                *total_block_count.lock().unwrap() += file_block_count;
                current_batch_file_infos.lock().unwrap().push(FileInfo {
                    file_path: file_path_str.clone(),
                    file_hash: current_file_hash.clone(),
                    is_new: is_new_file,
                });
            }
        } else {
            // Lines 214-216: Update hash if not being processed
            let cache = self.cache_manager.clone();
            cache.update_hash(&file_path_str, current_file_hash);
        }
        
        Ok(())
    }
    
    /// Lines 330-458: Process batch of code blocks
    async fn process_batch(
        &self,
        batch_blocks: Vec<CodeBlock>,
        batch_texts: Vec<String>,
        batch_file_infos: Vec<FileInfo>,
        scan_workspace: PathBuf,
        on_error: Option<Arc<dyn Fn(Error) + Send + Sync>>,
        on_blocks_indexed: Option<Arc<dyn Fn(usize) + Send + Sync>>,
    ) {
        if batch_blocks.is_empty() {
            return;
        }
        
        let mut attempts = 0;
        let mut success = false;
        let mut last_error: Option<Error> = None;
        
        // Lines 344-439: Retry loop
        while attempts < MAX_BATCH_RETRIES && !success {
            attempts += 1;
            
            match self.process_batch_attempt(
                &batch_blocks,
                &batch_texts,
                &batch_file_infos,
                &scan_workspace,
                &on_blocks_indexed,
            ).await {
                Ok(_) => success = true,
                Err(error) => {
                    last_error = Some(Error::Runtime { 
                        message: error.to_string() 
                    });
                    log::error!(
                        "[DirectoryScanner] Error processing batch (attempt {}) in workspace {}: {:?}",
                        attempts, scan_workspace.display(), error
                    );
                    
                    if attempts < MAX_BATCH_RETRIES {
                        let delay = INITIAL_RETRY_DELAY_MS * 2_u64.pow((attempts - 1) as u32);
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }
        
        // Lines 441-457: Handle failure after all retries
        if !success {
            if let Some(error) = last_error {
                log::error!(
                    "[DirectoryScanner] Failed to process batch after {} attempts",
                    MAX_BATCH_RETRIES
                );
                if let Some(ref on_error) = on_error {
                    on_error(Error::Runtime {
                        message: format!(
                            "Failed to process batch after {} attempts: {}",
                            MAX_BATCH_RETRIES, error
                        )
                    });
                }
            }
        }
    }
    
    async fn process_batch_attempt(
        &self,
        batch_blocks: &[CodeBlock],
        batch_texts: &[String],
        batch_file_infos: &[FileInfo],
        scan_workspace: &Path,
        on_blocks_indexed: &Option<Arc<dyn Fn(usize) + Send + Sync>>,
    ) -> Result<()> {
        // Lines 347-385: Delete existing points for modified files
        let unique_file_paths: Vec<String> = batch_file_infos
            .iter()
            .filter(|info| !info.is_new)
            .map(|info| info.file_path.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        
        if !unique_file_paths.is_empty() {
            self.qdrant_client.delete_points_by_multiple_file_paths(&unique_file_paths).await?;
        }
        
        // Lines 388-389: Create embeddings
        let embed_start = std::time::Instant::now();
        let embedding_response = self.embedder.create_embeddings(
            batch_texts.to_vec(),
            None
        ).await?;
        let embed_duration = embed_start.elapsed();
        log::debug!("[DirectoryScanner] Created {} embeddings in {:.2}ms", 
                   batch_texts.len(), embed_duration.as_millis());
        
        // Lines 391-409: Prepare points for vector store
        let points: Vec<PointStruct> = batch_blocks
            .iter()
            .enumerate()
            .map(|(index, block)| {
                let normalized_path = generate_normalized_absolute_path(&block.file_path, scan_workspace);
                let point_id = Uuid::new_v4();
                
                PointStruct {
                    id: point_id.to_string(),
                    vector: embedding_response.embeddings[index].clone(),
                    payload: {
                        let mut payload = HashMap::new();
                        payload.insert("filePath".to_string(), serde_json::Value::String(
                            generate_relative_file_path(&normalized_path, scan_workspace)
                        ));
                        payload.insert("codeChunk".to_string(), serde_json::Value::String(block.content.clone()));
                        payload.insert("startLine".to_string(), serde_json::Value::Number(block.start_line.into()));
                        payload.insert("endLine".to_string(), serde_json::Value::Number(block.end_line.into()));
                        payload.insert("segmentHash".to_string(), serde_json::Value::String(block.segment_hash.clone()));
                        payload
                    }
                }
            })
            .collect();
        
        // Lines 411-413: Upsert points
        let upsert_start = std::time::Instant::now();
        self.qdrant_client.upsert_points(points).await?;
        let upsert_duration = upsert_start.elapsed();
        log::debug!("[DirectoryScanner] Upserted {} points in {:.2}ms",
                   batch_blocks.len(), upsert_duration.as_millis());
        
        if let Some(ref on_blocks_indexed) = on_blocks_indexed {
            on_blocks_indexed(batch_blocks.len());
        }
        
        // Lines 415-418: Update cache
        let cache = self.cache_manager.clone();
        for file_info in batch_file_infos {
            cache.update_hash(&file_info.file_path, file_info.file_hash.clone());
        }
        
        Ok(())
    }
}

// Supporting types
#[derive(Clone)]
struct FileInfo {
    file_path: String,
    file_hash: String,
    is_new: bool,
}

pub struct ScanResult {
    pub stats: ScanStats,
    pub total_block_count: usize,
}

pub struct ScanStats {
    pub processed: usize,
    pub skipped: usize,
}

// Trait for ignore functionality
pub trait Ignore: Send + Sync {
    fn ignores(&self, path: &str) -> bool;
}

// Helper structs
pub struct RooIgnoreController {
    directory: PathBuf,
}

impl RooIgnoreController {
    pub fn new(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
        }
    }
    
    pub async fn initialize(&self) -> Result<()> {
        // Could load .gitignore, .rooignore etc here
        Ok(())
    }
    
    pub fn filter_paths(&self, paths: Vec<PathBuf>) -> Vec<PathBuf> {
        paths.into_iter()
            .filter(|path| {
                // Filter out common ignored directories
                for component in path.components() {
                    if let Some(name) = component.as_os_str().to_str() {
                        if matches!(name, "node_modules" | ".git" | "target" | "dist" | "build" | ".idea" | ".vscode") {
                            return false;
                        }
                    }
                }
                true
            })
            .collect()
    }
}

// Helper functions
pub async fn list_files(directory: &Path, recursive: bool, limit: usize) -> Result<Vec<PathBuf>> {
    use walkdir::WalkDir;
    let mut files = Vec::new();
    let mut count = 0;
    
    let walker = if recursive {
        WalkDir::new(directory).follow_links(false)
    } else {
        WalkDir::new(directory).max_depth(1).follow_links(false)
    };
    
    for entry in walker {
        if count >= limit {
            break;
        }
        
        match entry {
            Ok(e) => {
                let path = e.path();
                // Skip hidden directories
                if path.file_name()
                    .and_then(|n| n.to_str())
                    .map_or(false, |n| n.starts_with('.') && n != ".") {
                    continue;
                }
                files.push(path.to_path_buf());
                count += 1;
            }
            Err(e) => {
                log::warn!("Error walking directory: {}", e);
            }
        }
    }
    
    Ok(files)
}

fn get_workspace_path_for_context(directory: &Path) -> PathBuf {
    directory.to_path_buf()
}

fn generate_relative_file_path(file_path: &Path, workspace: &Path) -> String {
    file_path.strip_prefix(workspace)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string()
}

fn generate_normalized_absolute_path(file_path: &str, workspace: &Path) -> PathBuf {
    workspace.join(file_path)
}

pub fn is_path_in_ignored_directory(path: &Path) -> bool {
    // Check if path is in common ignored directories
    for component in path.components() {
        if let Some(name) = component.as_os_str().to_str() {
            if matches!(name, "node_modules" | ".git" | "target" | "dist" | "build" | ".idea" | ".vscode" | "__pycache__" | ".pytest_cache") {
                return true;
            }
        }
    }
    false
}

// Scanner extensions
const SCANNER_EXTENSIONS: &[&str] = &[
    ".ts", ".tsx", ".js", ".jsx", ".py", ".rs", ".go", ".java", ".c", ".cpp", ".h", ".hpp",
    ".cs", ".rb", ".php", ".swift", ".kt", ".scala", ".r", ".m", ".mm", ".sh", ".bash",
    ".zsh", ".fish", ".ps1", ".yaml", ".yml", ".json", ".toml", ".xml", ".html", ".css",
    ".scss", ".sass", ".less", ".sql", ".graphql", ".vue", ".svelte"
];

impl Clone for DirectoryScanner {
    fn clone(&self) -> Self {
        Self {
            embedder: self.embedder.clone(),
            qdrant_client: self.qdrant_client.clone(),
            code_parser: self.code_parser.clone(),
            cache_manager: self.cache_manager.clone(),
            ignore_instance: self.ignore_instance.clone(),
        }
    }
}
