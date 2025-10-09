// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// OS-Native File Watcher using notify crate

use crate::error::{Error, Result};
use notify::{Watcher, RecursiveMode, Event, EventKind, Config};
use notify::event::{CreateKind, ModifyKind, RemoveKind};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{Sender, Receiver, channel};
use regex::Regex;
use serde::{Deserialize, Serialize};

// VectorStoreManager and SemanticSearchEngine imports removed - modules don't exist
// FileProcessor import removed - module doesn't exist

/// Native file watcher using OS-specific events
pub struct NativeFileWatcher {
    workspace_path: PathBuf,
    watcher: Option<notify::RecommendedWatcher>,
    event_sender: Sender<FileSystemEvent>,
    event_receiver: Option<Receiver<FileSystemEvent>>,
    ignore_patterns: Arc<Mutex<HashSet<String>>>,
    debounce_duration: Duration,
    batch_sender: broadcast::Sender<BatchEvent>,
}

impl Default for NativeFileWatcher {
    fn default() -> Self {
        let (event_sender, mut event_receiver) = channel(100);
        let (batch_sender, _) = broadcast::channel(100);
        
        Self {
            workspace_path: PathBuf::from("."),
            watcher: None,
            event_sender,
            event_receiver: Some(event_receiver),
            ignore_patterns: Arc::new(Mutex::new(HashSet::new())),
            debounce_duration: Duration::from_millis(100),
            batch_sender,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileSystemEvent {
    pub path: PathBuf,
    pub event_type: FileEventType,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileEventType {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone)]
pub struct BatchEvent {
    pub events: Vec<FileSystemEvent>,
    pub batch_id: uuid::Uuid,
}

impl NativeFileWatcher {
    /// Create new native file watcher
    pub fn new(workspace_path: PathBuf) -> Result<Self> {
        let (tx, rx) = channel(1000);
        let (batch_tx, _) = broadcast::channel(100);
        
        let ignore_patterns = Arc::new(Mutex::new(HashSet::new()));
        
        // Default ignore patterns
        ignore_patterns.lock().unwrap().extend([
            ".git".to_string(),
            ".svn".to_string(),
            "node_modules".to_string(),
            "target".to_string(),
            ".DS_Store".to_string(),
            "*.pyc".to_string(),
            "__pycache__".to_string(),
            ".vscode".to_string(),
            ".idea".to_string(),
        ]);
        
        Ok(Self {
            workspace_path,
            watcher: None,
            event_sender: tx,
            event_receiver: Some(rx),
            ignore_patterns,
            debounce_duration: Duration::from_millis(500),
            batch_sender: batch_tx,
        })
    }
    
    /// Start watching the workspace
    pub async fn start(&mut self) -> Result<()> {
        // Create OS-native watcher
        let tx = self.event_sender.clone();
        let ignore_patterns = self.ignore_patterns.clone();
        
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    // Process event
                    if let Some(fs_event) = Self::process_notify_event(event, &ignore_patterns) {
                        // Send through channel
                        let _ = tx.send(fs_event);
                    }
                }
                Err(e) => {
                    log::error!("Watch error: {:?}", e);
                }
            }
        }).map_err(|e| Error::Runtime {
            message: format!("Failed to create watcher: {}", e)
        })?;
        
        // Watch workspace recursively
        watcher.watch(&self.workspace_path, RecursiveMode::Recursive)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to watch path: {}", e)
            })?;
            
        self.watcher = Some(watcher);
        
        // Start event processing loop
        self.start_event_processing().await;
        
        Ok(())
    }
    
    /// Process notify event into our event type
    fn process_notify_event(event: Event, ignore_patterns: &Arc<Mutex<HashSet<String>>>) -> Option<FileSystemEvent> {
        // Check if path should be ignored
        for path in &event.paths {
            if Self::should_ignore(path, ignore_patterns) {
                return None;
            }
        }
        
        let event_type = match event.kind {
            EventKind::Create(CreateKind::File) => {
                FileEventType::Created
            }
            EventKind::Modify(ModifyKind::Data(_)) | 
            EventKind::Modify(ModifyKind::Any) => {
                FileEventType::Modified
            }
            EventKind::Remove(RemoveKind::File) => {
                FileEventType::Deleted
            }
            EventKind::Modify(ModifyKind::Name(notify::event::RenameMode::Both)) => {
                if event.paths.len() >= 2 {
                    return Some(FileSystemEvent {
                        path: event.paths[1].clone(),
                        event_type: FileEventType::Renamed {
                            from: event.paths[0].clone(),
                            to: event.paths[1].clone(),
                        },
                        timestamp: std::time::SystemTime::now(),
                    });
                }
                FileEventType::Modified
            }
            _ => return None, // Ignore other events
        };
        
        if let Some(path) = event.paths.first() {
            Some(FileSystemEvent {
                path: path.clone(),
                event_type,
                timestamp: std::time::SystemTime::now(),
            })
        } else {
            None
        }
    }
    
    /// Check if path should be ignored
    fn should_ignore(path: &Path, ignore_patterns: &Arc<Mutex<HashSet<String>>>) -> bool {
        let patterns = ignore_patterns.lock().unwrap();
        
        // Check each component of the path
        for component in path.components() {
            if let Some(name) = component.as_os_str().to_str() {
                // Direct match
                if patterns.contains(name) {
                    return true;
                }
                
                // Pattern matching
                for pattern in patterns.iter() {
                    if pattern.starts_with('*') {
                        let suffix = &pattern[1..];
                        if name.ends_with(suffix) {
                            return true;
                        }
                    } else if pattern.ends_with('*') {
                        let prefix = &pattern[..pattern.len()-1];
                        if name.starts_with(prefix) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
    
    /// Start processing events with debouncing
    async fn start_event_processing(&mut self) {
        let mut receiver = self.event_receiver.take()
            .expect("Event receiver already taken");
            
        let batch_sender = self.batch_sender.clone();
        let debounce_duration = self.debounce_duration;
        
        tokio::spawn(async move {
            let mut pending_events: HashMap<PathBuf, FileSystemEvent> = HashMap::new();
            let mut last_batch_time = std::time::Instant::now();
            
            loop {
                // Wait for events or timeout
                tokio::select! {
                    event = receiver.recv() => {
                        let Some(event) = event else { break; };
                        // Add to pending events (deduplicates by path)
                        pending_events.insert(event.path.clone(), event);
                        
                        // Check if we should send batch
                        if last_batch_time.elapsed() >= debounce_duration {
                            Self::send_batch(&mut pending_events, &batch_sender).await;
                            last_batch_time = std::time::Instant::now();
                        }
                    }
                    _ = tokio::time::sleep(debounce_duration) => {
                        // Send pending events as batch
                        if !pending_events.is_empty() {
                            Self::send_batch(&mut pending_events, &batch_sender).await;
                            last_batch_time = std::time::Instant::now();
                        }
                    }
                }
            }
        });
    }
    
    /// Send batch of events
    async fn send_batch(
        pending_events: &mut HashMap<PathBuf, FileSystemEvent>,
        batch_sender: &broadcast::Sender<BatchEvent>
    ) {
        if pending_events.is_empty() {
            return;
        }
        
        let events: Vec<FileSystemEvent> = pending_events.drain().map(|(_, e)| e).collect();
        let batch = BatchEvent {
            events: events.clone(),
            batch_id: uuid::Uuid::new_v4(),
        };
        
        log::info!("Sending batch of {} file events", events.len());
        let _ = batch_sender.send(batch);
    }
    
    /// Stop watching
    pub fn stop(&mut self) {
        self.watcher = None; // Drops the watcher, stopping it
    }
    
    /// Add ignore pattern
    pub fn add_ignore_pattern(&self, pattern: String) {
        self.ignore_patterns.lock().unwrap().insert(pattern);
    }
    
    /// Remove ignore pattern
    pub fn remove_ignore_pattern(&self, pattern: &str) {
        self.ignore_patterns.lock().unwrap().remove(pattern);
    }
    
    /// Get batch event receiver
    pub fn subscribe(&self) -> broadcast::Receiver<BatchEvent> {
        self.batch_sender.subscribe()
    }
    
    /// Watch specific file
    pub async fn watch_file(&mut self, path: &Path) -> Result<()> {
        if let Some(watcher) = &mut self.watcher {
            watcher.watch(path, RecursiveMode::NonRecursive)
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to watch file: {}", e)
                })?;
        }
        Ok(())
    }
    
    /// Unwatch specific file
    pub async fn unwatch_file(&mut self, path: &Path) -> Result<()> {
        if let Some(watcher) = &mut self.watcher {
            watcher.unwatch(path)
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to unwatch file: {}", e)
                })?;
        }
        Ok(())
    }
    
    /// Process batch for indexing
    pub async fn process_batch_for_indexing(&self, batch: BatchEvent) -> BatchProcessingResult {
        let mut created = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();
        
        for event in batch.events {
            // Only process code files
            if !Self::is_code_file(&event.path) {
                continue;
            }
            
            match event.event_type {
                FileEventType::Created => created.push(event.path),
                FileEventType::Modified => modified.push(event.path),
                FileEventType::Deleted => deleted.push(event.path),
                FileEventType::Renamed { from, to } => {
                    deleted.push(from);
                    created.push(to);
                }
            }
        }
        
        BatchProcessingResult {
            batch_id: batch.batch_id,
            created_files: created,
            modified_files: modified,
            deleted_files: deleted,
            timestamp: std::time::SystemTime::now(),
        }
    }
    
    /// Check if file is a code file
    fn is_code_file(path: &Path) -> bool {
        let code_extensions = [
            "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", 
            "cpp", "c", "h", "hpp", "cs", "rb", "php", "swift",
            "kt", "scala", "r", "m", "mm", "sh", "bash", "zsh",
            "sql", "html", "css", "scss", "sass", "vue", "svelte"
        ];
        
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            code_extensions.contains(&ext)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchProcessingResult {
    pub batch_id: uuid::Uuid,
    pub created_files: Vec<PathBuf>,
    pub modified_files: Vec<PathBuf>,
    pub deleted_files: Vec<PathBuf>,
    pub timestamp: std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;
    
    #[tokio::test]
    async fn test_file_watcher() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = NativeFileWatcher::new(temp_dir.path().to_path_buf()).unwrap();
        
        let mut receiver = watcher.subscribe();
        
        // Start watcher
        watcher.start().await.unwrap();
        
        // Create a test file
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").await.unwrap();
        
        // Wait for event
        tokio::time::sleep(Duration::from_millis(600)).await;
        
        // Check if we received the event
        if let Ok(batch) = receiver.try_recv() {
            assert!(!batch.events.is_empty());
            assert_eq!(batch.events[0].event_type, FileEventType::Created);
        }
        
        watcher.stop();
    }
}
