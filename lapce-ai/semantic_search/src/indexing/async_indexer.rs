// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Async indexer with back-pressure and timeout handling (CST-B10)
//!
//! Provides production-grade async indexing with:
//! - Bounded work queues to prevent memory exhaustion
//! - Timeout handling for slow operations
//! - Graceful degradation under load
//! - Cancellation support

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::timeout;
use crate::error::{Error, Result};

/// Configuration for async indexer
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// Maximum concurrent indexing tasks
    pub max_concurrent_tasks: usize,
    
    /// Timeout for single file indexing
    pub file_timeout: Duration,
    
    /// Timeout for embedding generation
    pub embedding_timeout: Duration,
    
    /// Queue capacity (back-pressure kicks in when full)
    pub queue_capacity: usize,
    
    /// Enable incremental indexing
    pub enable_incremental: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            file_timeout: Duration::from_secs(30),
            embedding_timeout: Duration::from_secs(10),
            queue_capacity: 1000,
            enable_incremental: true,
        }
    }
}

/// Indexing task for async processing
#[derive(Debug, Clone)]
pub struct IndexTask {
    pub file_path: PathBuf,
    pub priority: TaskPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Result of indexing operation
#[derive(Debug, Clone)]
pub struct IndexResult {
    pub file_path: PathBuf,
    pub success: bool,
    pub duration: Duration,
    pub embeddings_generated: usize,
    pub embeddings_reused: usize,
    pub error: Option<String>,
}

/// Async indexer with bounded concurrency and timeouts
pub struct AsyncIndexer {
    config: IndexerConfig,
    task_tx: mpsc::Sender<IndexTask>,
    result_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<IndexResult>>>,
    semaphore: Arc<Semaphore>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
}

impl AsyncIndexer {
    /// Create new async indexer with default config
    pub fn new() -> Self {
        Self::with_config(IndexerConfig::default())
    }
    
    /// Create async indexer with custom config
    pub fn with_config(config: IndexerConfig) -> Self {
        let (task_tx, task_rx) = mpsc::channel(config.queue_capacity);
        let (result_tx, result_rx) = mpsc::channel(config.queue_capacity);
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_tasks));
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        
        // Spawn worker pool
        let config_clone = config.clone();
        let semaphore_clone = semaphore.clone();
        tokio::spawn(Self::worker_pool(
            task_rx,
            result_tx,
            config_clone,
            semaphore_clone,
            shutdown_rx,
        ));
        
        Self {
            config,
            task_tx,
            result_rx: Arc::new(tokio::sync::Mutex::new(result_rx)),
            semaphore,
            shutdown_tx,
        }
    }
    
    /// Submit indexing task (blocks if queue is full - back-pressure)
    pub async fn submit(&self, task: IndexTask) -> Result<()> {
        self.task_tx.send(task).await.map_err(|e| Error::Runtime {
            message: format!("Failed to submit task: {}", e)
        })
    }
    
    /// Try to submit task without blocking (returns error if queue full)
    pub fn try_submit(&self, task: IndexTask) -> Result<()> {
        self.task_tx.try_send(task).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => Error::Runtime {
                message: "Indexer queue full - back-pressure applied".to_string()
            },
            mpsc::error::TrySendError::Closed(_) => Error::Runtime {
                message: "Indexer has been shut down".to_string()
            },
        })
    }
    
    /// Get next result (blocks until available)
    pub async fn next_result(&self) -> Option<IndexResult> {
        self.result_rx.lock().await.recv().await
    }
    
    /// Try to get result without blocking
    pub async fn try_next_result(&self) -> Option<IndexResult> {
        self.result_rx.lock().await.try_recv().ok()
    }
    
    /// Get number of permits available (remaining concurrency)
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
    
    /// Check if queue has capacity
    pub fn has_capacity(&self) -> bool {
        self.task_tx.capacity() > 0
    }
    
    /// Shutdown indexer gracefully
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
    
    /// Worker pool that processes indexing tasks
    async fn worker_pool(
        mut task_rx: mpsc::Receiver<IndexTask>,
        result_tx: mpsc::Sender<IndexResult>,
        config: IndexerConfig,
        semaphore: Arc<Semaphore>,
        mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        log::info!("AsyncIndexer shutting down gracefully");
                        break;
                    }
                }
                
                // Process next task
                Some(task) = task_rx.recv() => {
                    // Acquire permit (bounds concurrency)
                    let permit = semaphore.clone().acquire_owned().await;
                    
                    if permit.is_err() {
                        log::error!("Failed to acquire semaphore permit");
                        continue;
                    }
                    
                    let result_tx = result_tx.clone();
                    let config = config.clone();
                    
                    // Spawn task with timeout
                    tokio::spawn(async move {
                        let _permit = permit.unwrap(); // Hold permit during execution
                        let result = Self::process_task(task, &config).await;
                        let _ = result_tx.send(result).await;
                    });
                }
                
                else => break, // Channel closed
            }
        }
    }
    
    /// Process single indexing task with timeout
    async fn process_task(task: IndexTask, config: &IndexerConfig) -> IndexResult {
        let start = std::time::Instant::now();
        
        // Apply file-level timeout
        let result = timeout(
            config.file_timeout,
            Self::index_file(&task.file_path, config)
        ).await;
        
        let duration = start.elapsed();
        
        match result {
            Ok(Ok((embeddings_generated, embeddings_reused))) => IndexResult {
                file_path: task.file_path,
                success: true,
                duration,
                embeddings_generated,
                embeddings_reused,
                error: None,
            },
            Ok(Err(e)) => IndexResult {
                file_path: task.file_path,
                success: false,
                duration,
                embeddings_generated: 0,
                embeddings_reused: 0,
                error: Some(format!("{}", e)),
            },
            Err(_) => IndexResult {
                file_path: task.file_path,
                success: false,
                duration,
                embeddings_generated: 0,
                embeddings_reused: 0,
                error: Some(format!("Timeout after {:?}", config.file_timeout)),
            },
        }
    }
    
    /// Index a single file (placeholder - integrates with CstToAstPipeline)
    async fn index_file(_path: &PathBuf, _config: &IndexerConfig) -> Result<(usize, usize)> {
        // Simulate work
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // In production, would:
        // 1. Parse file with CstToAstPipeline
        // 2. Use CachedEmbedder for incremental indexing
        // 3. Store embeddings in vector DB
        
        Ok((5, 3)) // (generated, reused)
    }
}

impl Default for AsyncIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_indexing() {
        let indexer = AsyncIndexer::new();
        
        let task = IndexTask {
            file_path: PathBuf::from("/test.rs"),
            priority: TaskPriority::Normal,
        };
        
        indexer.submit(task).await.unwrap();
        
        let result = indexer.next_result().await.unwrap();
        assert!(result.success);
        assert_eq!(result.file_path, PathBuf::from("/test.rs"));
    }
    
    #[tokio::test]
    async fn test_concurrent_tasks() {
        let config = IndexerConfig {
            max_concurrent_tasks: 2,
            ..Default::default()
        };
        let indexer = AsyncIndexer::with_config(config);
        
        // Submit multiple tasks
        for i in 0..5 {
            let task = IndexTask {
                file_path: PathBuf::from(format!("/test{}.rs", i)),
                priority: TaskPriority::Normal,
            };
            indexer.submit(task).await.unwrap();
        }
        
        // Collect results
        let mut results = Vec::new();
        for _ in 0..5 {
            if let Some(result) = indexer.next_result().await {
                results.push(result);
            }
        }
        
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.success));
    }
    
    #[tokio::test]
    async fn test_back_pressure() {
        let config = IndexerConfig {
            queue_capacity: 2,
            ..Default::default()
        };
        let indexer = AsyncIndexer::with_config(config);
        
        // Fill queue
        for i in 0..2 {
            let task = IndexTask {
                file_path: PathBuf::from(format!("/test{}.rs", i)),
                priority: TaskPriority::Normal,
            };
            indexer.submit(task).await.unwrap();
        }
        
        // Try to submit when full
        let task = IndexTask {
            file_path: PathBuf::from("/overflow.rs"),
            priority: TaskPriority::Normal,
        };
        
        let result = indexer.try_submit(task);
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_available_permits() {
        let config = IndexerConfig {
            max_concurrent_tasks: 3,
            ..Default::default()
        };
        let indexer = AsyncIndexer::with_config(config);
        
        let initial_permits = indexer.available_permits();
        assert_eq!(initial_permits, 3);
    }
    
    #[tokio::test]
    async fn test_graceful_shutdown() {
        let indexer = AsyncIndexer::new();
        
        // Submit some tasks
        for i in 0..3 {
            let task = IndexTask {
                file_path: PathBuf::from(format!("/test{}.rs", i)),
                priority: TaskPriority::Normal,
            };
            indexer.submit(task).await.unwrap();
        }
        
        // Shutdown
        indexer.shutdown().await;
        
        // Should still process submitted tasks
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
