// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Incremental Indexing for Real-time Updates - Lines 469-511 from doc

use crate::error::{Error, Result};
use crate::processors::native_file_watcher::NativeFileWatcher;
use crate::processors::cst_to_ast_pipeline::{CstToAstPipeline, PipelineOutput};
use crate::search::semantic_search_engine::{SemanticSearchEngine, ChunkMetadata, IndexStats};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::{HashSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

/// File change event
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone)]
pub enum ChangeKind {
    Create,
    Modify,
    Delete,
}

/// Incremental indexer for real-time updates - Lines 470-475 from doc
pub struct IncrementalIndexer {
    search_engine: Arc<SemanticSearchEngine>,
    file_watcher: Arc<NativeFileWatcher>,
    cst_pipeline: Arc<CstToAstPipeline>,
    change_buffer: Arc<Mutex<Vec<FileChange>>>,
    indexed_files: Arc<Mutex<HashSet<PathBuf>>>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
    use_cst: bool,
    debounce_duration: Duration,
}

impl IncrementalIndexer {
    /// Extract chunks from AST
    fn extract_chunks_from_ast(&self, ast: &crate::processors::cst_to_ast_pipeline::AstNode) -> Result<Vec<ChunkMetadata>> {
        let mut chunks = Vec::new();
        
        // Extract function/class definitions as chunks
        chunks.push(ChunkMetadata {
            path: PathBuf::from(ast.metadata.source_file.as_ref().unwrap_or(&PathBuf::from(""))),
            content: ast.text.clone(),
            start_line: ast.metadata.start_line as i32,
            end_line: ast.metadata.end_line as i32,
            language: Some(ast.metadata.language.clone()),
            metadata: HashMap::new(),
        });
        
        // Recursively extract from children
        for child in &ast.children {
            if let Ok(child_chunks) = self.extract_chunks_from_ast(child) {
                chunks.extend(child_chunks);
            }
        }
        
        Ok(chunks)
    }
    
    /// Create new incremental indexer
    pub fn new(
        search_engine: Arc<SemanticSearchEngine>,
    ) -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        
        Self {
            search_engine,
            file_watcher: Arc::new(NativeFileWatcher::default()),
            cst_pipeline: Arc::new(CstToAstPipeline::new()),
            change_buffer: Arc::new(Mutex::new(Vec::new())),
            indexed_files: Arc::new(Mutex::new(HashSet::new())),
            shutdown_tx,
            use_cst: true,
            debounce_duration: Duration::from_millis(500),
        }
    }
    
    /// Set debounce duration
    pub fn with_debounce(mut self, duration: Duration) -> Self {
        self.debounce_duration = duration;
        self
    }
    
    /// Process a file change using CST pipeline - Lines 492-510 from doc
    pub async fn process_file_change(&self, path: PathBuf, kind: ChangeKind) -> Result<()> {
        let start = std::time::Instant::now();
        
        match kind {
            ChangeKind::Delete => {
                // Delete entries for removed file
                self.search_engine.delete_by_path(&path).await?;
                
                // Clear query cache
                self.search_engine.query_cache.clear().await;
            }
            ChangeKind::Create | ChangeKind::Modify => {
                // Delete old entries first
                self.search_engine.delete_by_path(&path).await?;
                
                // Parse file with CST pipeline
                let output = self.cst_pipeline.process_file(&path).await?;
                
                // Extract chunks from AST
                let chunks = self.extract_chunks_from_ast(&output.ast)?;
                
                // Generate embeddings and insert
                let mut embeddings = Vec::new();
                for chunk in &chunks {
                    let response = self.search_engine.embedder
                        .create_embeddings(vec![chunk.content.clone()], None)
                        .await?;
                    if let Some(embedding) = response.embeddings.into_iter().next() {
                        embeddings.push(embedding);
                    }
                }
                
                if !embeddings.is_empty() {
                    self.search_engine.batch_insert(embeddings, chunks).await?;
                }
                
                let elapsed = start.elapsed();
                if elapsed > Duration::from_millis(100) {
                    tracing::warn!(
                        "File update took {:?} (target: <100ms) for {:?}",
                        elapsed, path
                    );
                } else {
                    tracing::debug!(
                        "File update completed in {:?} for {:?}",
                        elapsed, path
                    );
                }
            }
        }
        
        Ok(())
    }
    
    /// Start monitoring for file changes - Lines 477-490 from doc
    pub async fn start(&self, watch_path: PathBuf) -> Result<()> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let change_buffer = self.change_buffer.clone();
        let search_engine = self.search_engine.clone();
        let debounce = self.debounce_duration;
        
        // Clone what we need for the async block
        let shutdown_tx_clone = self.shutdown_tx.clone();
        let indexer_clone = self.clone();
        
        // Spawn processor task
        let processor_handle = tokio::spawn(async move {
            let mut shutdown_rx = shutdown_tx_clone.subscribe();
            
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        log::info!("Shutting down processor");
                        break;
                    }
                    _ = sleep(debounce) => {
                        // Process buffered changes
                        let changes = {
                            let mut buffer = change_buffer.lock().await;
                            std::mem::take(&mut *buffer)
                        };
                        
                        for change in changes {
                            if let Err(e) = indexer_clone.process_file_change(change.path, change.kind).await {
                                log::error!("Failed to process change: {}", e);
                            }
                        }
                    }
                }
            }
        });
        
        // Wait for processor to complete
        processor_handle.await
            .map_err(|e| Error::Runtime {
                message: format!("Task failed: {}", e)
            })?;
        
        Ok(())
    }
    
    /// Handle a file change - Lines 492-510 from doc
    pub async fn handle_change(&self, change: FileChange) -> Result<()> {
        // Buffer the change for debouncing
        let mut buffer = self.change_buffer.lock().await;
        
        // Check if this path is already in the buffer
        if let Some(existing) = buffer.iter_mut().find(|c| c.path == change.path) {
            // Update the change kind (latest wins)
            existing.kind = change.kind;
        } else {
            buffer.push(change);
        }
        
        Ok(())
    }
    
    /// Flush and process buffered changes
    pub async fn flush_changes(&self) -> Result<IndexStats> {
        let changes = {
            let mut buffer = self.change_buffer.lock().await;
            if buffer.is_empty() {
                return Ok(IndexStats::default());
            }
            std::mem::take(&mut *buffer)
        };
        
        let mut stats = IndexStats::default();
        
        for change in changes {
            match change.kind {
                ChangeKind::Create | ChangeKind::Modify => {
                    // Re-index file
                    log::debug!("Re-indexing file: {}", change.path.display());
                    
                    // Delete old entries if modifying
                    if matches!(change.kind, ChangeKind::Modify) {
                        self.search_engine.delete_by_path(&change.path).await?;
                    }
                    
                    // Parse and index the file
                    let content = tokio::fs::read_to_string(&change.path).await
                        .map_err(|e| Error::Runtime {
                            message: format!("Failed to read file: {}", e)
                        })?;
                    
                    let chunks = vec![ChunkMetadata {
                        path: change.path.clone(),
                        content,
                        start_line: 1,
                        end_line: 1,
                        language: change.path.extension()
                            .and_then(|ext| ext.to_str())
                            .map(|s| s.to_string()),
                        metadata: HashMap::new(),
                    }];
                    
                    if !chunks.is_empty() {
                        let mut embeddings = Vec::new();
                        
                        for chunk in &chunks {
                            let embedding_response = self.search_engine
                                .embedder
                                .create_embeddings(vec![chunk.content.clone()], None)
                                .await?;
                                
                            if let Some(embedding) = embedding_response.embeddings.into_iter().next() {
                                embeddings.push(embedding);
                            }
                        }
                        
                        if !embeddings.is_empty() {
                            let batch_stats = self.search_engine
                                .batch_insert(embeddings, chunks)
                                .await?;
                            stats.files_indexed += batch_stats.files_indexed;
                            stats.chunks_created += batch_stats.chunks_created;
                        }
                    }
                }
                ChangeKind::Delete => {
                    // Remove from index
                    log::debug!("Removing from index: {}", change.path.display());
                    self.search_engine.delete_by_path(&change.path).await?;
                    
                    // Invalidate cache entries for this path
                    self.search_engine.query_cache.clear().await;
                }
            }
        }
        
        // Optimize index periodically
        if stats.files_indexed > 10 {
            self.search_engine.optimize_index().await?;
        }
        
        Ok(stats)
    }
    
    /// Stop the incremental indexer
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(());
    }
    
    /// Get the number of pending changes
    pub async fn pending_changes(&self) -> usize {
        self.change_buffer.lock().await.len()
    }
}
