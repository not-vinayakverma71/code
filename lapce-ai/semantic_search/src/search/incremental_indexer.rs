// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Incremental Indexing for Real-time Updates - Lines 469-511 from doc

use crate::error::{Error, Result};
use crate::processors::file_watcher::FileWatcher;
use crate::search::semantic_search_engine::SemanticSearchEngine;
use crate::search::code_indexer::CodeIndexer;
use crate::search::semantic_search_engine::IndexStats;
use crate::search::improved_cache::ImprovedQueryCache;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
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

/// Incremental indexer for real-time file updates - Lines 470-474 from doc
#[derive(Clone)]
pub struct IncrementalIndexer {
    search_engine: Arc<SemanticSearchEngine>,
    code_indexer: Arc<CodeIndexer>,
    query_cache: Arc<ImprovedQueryCache>,
    change_buffer: Arc<Mutex<Vec<FileChange>>>,
    shutdown_tx: broadcast::Sender<()>,
    debounce_duration: Duration,
}

impl IncrementalIndexer {
    /// Create new incremental indexer
    pub fn new(
        search_engine: Arc<SemanticSearchEngine>,
        query_cache: Arc<ImprovedQueryCache>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let code_indexer = Arc::new(CodeIndexer::new(search_engine.clone()));
        
        Self {
            search_engine,
            code_indexer,
            query_cache,
            change_buffer: Arc::new(Mutex::new(Vec::new())),
            shutdown_tx,
            debounce_duration: Duration::from_millis(500),
        }
    }
    
    /// Set debounce duration
    pub fn with_debounce(mut self, duration: Duration) -> Self {
        self.debounce_duration = duration;
        self
    }
    
    /// Start monitoring for file changes - Lines 477-490 from doc
    pub async fn start(&self, watch_path: PathBuf) -> Result<()> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let change_buffer = self.change_buffer.clone();
        let code_indexer = self.code_indexer.clone();
        let query_cache = self.query_cache.clone();
        let debounce = self.debounce_duration;
        
        // Spawn file watcher task
        let watcher_handle = tokio::spawn(async move {
            // In a real implementation, we'd integrate with the FileWatcher
            // For now, this is a placeholder for the monitoring loop
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        log::info!("Shutting down file watcher");
                        break;
                    }
                    _ = sleep(Duration::from_secs(5)) => {
                        // Check for changes and process them
                    }
                }
            }
        });
        
        // Clone what we need for the async block
        let shutdown_tx_clone = self.shutdown_tx.clone();
        let indexer_clone = self.clone();
        
        // Spawn processor task
        let processor_handle = tokio::spawn(async move {
            let mut shutdown_rx = shutdown_tx_clone.subscribe();
            
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        log::info!("Shutting down incremental indexer");
                        break;
                    }
                    _ = sleep(debounce) => {
                        // Process buffered changes
                        if let Err(e) = indexer_clone.flush_changes().await {
                            log::error!("Error processing changes: {}", e);
                        }
                    }
                }
            }
        });
        
        // Wait for tasks to complete
        tokio::try_join!(watcher_handle, processor_handle)
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
                        // Invalidate cache entries for this path
                        self.query_cache.invalidate_by_path(
                            &change.path.to_string_lossy()
                        ).await;
                    }
                    
                    // Parse and index the file
                    if change.path.exists() {
                        let chunks = self.parse_file(&change.path).await?;
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
                }
                ChangeKind::Delete => {
                    // Remove from index
                    log::debug!("Removing from index: {}", change.path.display());
                    self.search_engine.delete_by_path(&change.path).await?;
                    
                    // Invalidate cache entries for this path
                    self.query_cache.invalidate_by_path(
                        &change.path.to_string_lossy()
                    ).await;
                }
            }
        }
        
        // Optimize index periodically
        if stats.files_indexed > 10 {
            self.search_engine.optimize_index().await?;
        }
        
        Ok(stats)
    }
    
    /// Parse file into chunks (delegate to CodeIndexer)
    async fn parse_file(&self, path: &Path) -> Result<Vec<crate::search::semantic_search_engine::ChunkMetadata>> {
        // Read file content
        let content = tokio::fs::read_to_string(path).await.map_err(|e| Error::Runtime {
            message: format!("Failed to read file {}: {}", path.display(), e)
        })?;
        
        // Use the code parser to get chunks
        let parser = crate::processors::parser::CodeParser::new();
        let blocks = parser.parse_file(path, Some(&content), None).await?;
        
        // Convert to ChunkMetadata
        let chunks = blocks.into_iter().map(|block| {
            crate::search::semantic_search_engine::ChunkMetadata {
                path: path.to_path_buf(),
                content: block.content,
                start_line: block.start_line,
                end_line: block.end_line,
                language: path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s.to_string()),
                metadata: std::collections::HashMap::new(),
            }
        }).collect();
        
        Ok(chunks)
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
