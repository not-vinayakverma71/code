/// File System Watcher (LSP-016)
/// Keep SymbolIndex up to date with file changes, renames, and deletes

use anyhow::{Result, anyhow};
use notify::{Watcher, RecursiveMode, Event, EventKind, event::*};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashSet;

/// File system watcher for workspace
pub struct FileSystemWatcher {
    doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
    symbol_index: Arc<parking_lot::Mutex<super::SymbolIndex>>,
    
    // Debounce: avoid reacting to rapid changes
    pending_changes: Arc<parking_lot::Mutex<HashSet<PathBuf>>>,
    last_flush: Arc<parking_lot::Mutex<Instant>>,
    debounce_ms: u64,
    
    // Backoff: handle large repo changes
    event_count: Arc<parking_lot::Mutex<usize>>,
    backoff_threshold: usize,
    in_backoff: Arc<parking_lot::Mutex<bool>>,
}

impl FileSystemWatcher {
    pub fn new(
        doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
        symbol_index: Arc<parking_lot::Mutex<super::SymbolIndex>>,
    ) -> Self {
        Self {
            doc_sync,
            symbol_index,
            pending_changes: Arc::new(parking_lot::Mutex::new(HashSet::new())),
            last_flush: Arc::new(parking_lot::Mutex::new(Instant::now())),
            debounce_ms: 500, // 500ms debounce
            event_count: Arc::new(parking_lot::Mutex::new(0)),
            backoff_threshold: 100, // Backoff after 100 events in quick succession
            in_backoff: Arc::new(parking_lot::Mutex::new(false)),
        }
    }
    
    /// Start watching a workspace directory
    pub fn watch_workspace(&self, workspace_path: &Path) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;
        
        watcher.watch(workspace_path, RecursiveMode::Recursive)?;
        
        tracing::info!("Started watching workspace: {:?}", workspace_path);
        
        // Clone Arcs for the event handler thread
        let doc_sync = self.doc_sync.clone();
        let symbol_index = self.symbol_index.clone();
        let pending_changes = self.pending_changes.clone();
        let last_flush = self.last_flush.clone();
        let event_count = self.event_count.clone();
        let in_backoff = self.in_backoff.clone();
        let debounce_ms = self.debounce_ms;
        let backoff_threshold = self.backoff_threshold;
        
        // Spawn event handler thread
        std::thread::spawn(move || {
            // Keep watcher alive
            let _watcher = watcher;
            
            loop {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(event) => {
                        Self::handle_event(
                            &event,
                            &doc_sync,
                            &symbol_index,
                            &pending_changes,
                            &last_flush,
                            &event_count,
                            &in_backoff,
                            debounce_ms,
                            backoff_threshold,
                        );
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Check if we should flush pending changes
                        Self::maybe_flush_pending(
                            &doc_sync,
                            &symbol_index,
                            &pending_changes,
                            &last_flush,
                            debounce_ms,
                        );
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        tracing::warn!("File watcher channel disconnected");
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    fn handle_event(
        event: &Event,
        doc_sync: &Arc<parking_lot::Mutex<super::DocumentSync>>,
        symbol_index: &Arc<parking_lot::Mutex<super::SymbolIndex>>,
        pending_changes: &Arc<parking_lot::Mutex<HashSet<PathBuf>>>,
        last_flush: &Arc<parking_lot::Mutex<Instant>>,
        event_count: &Arc<parking_lot::Mutex<usize>>,
        in_backoff: &Arc<parking_lot::Mutex<bool>>,
        debounce_ms: u64,
        backoff_threshold: usize,
    ) {
        // Increment event count
        {
            let mut count = event_count.lock();
            *count += 1;
            
            // Check if we should enter backoff mode
            if *count > backoff_threshold {
                let mut backoff = in_backoff.lock();
                if !*backoff {
                    tracing::warn!("Entering backoff mode due to high event rate");
                    *backoff = true;
                    
                    // Clear pending changes and reset
                    pending_changes.lock().clear();
                }
            }
        }
        
        // Skip processing if in backoff
        if *in_backoff.lock() {
            return;
        }
        
        match &event.kind {
            EventKind::Create(CreateKind::File) | 
            EventKind::Modify(ModifyKind::Data(_)) => {
                // File created or modified
                for path in &event.paths {
                    if Self::is_source_file(path) {
                        pending_changes.lock().insert(path.clone());
                        tracing::debug!("Queued file for reindex: {:?}", path);
                    }
                }
            }
            EventKind::Remove(RemoveKind::File) => {
                // File deleted
                for path in &event.paths {
                    if Self::is_source_file(path) {
                        let uri = Self::path_to_uri(path);
                        
                        // Remove from index immediately
                        symbol_index.lock().remove_file(&uri);
                        tracing::debug!("Removed file from index: {:?}", path);
                    }
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                // File renamed (we get both old and new paths)
                if event.paths.len() >= 2 {
                    let old_path = &event.paths[0];
                    let new_path = &event.paths[1];
                    
                    if Self::is_source_file(old_path) && Self::is_source_file(new_path) {
                        let old_uri = Self::path_to_uri(old_path);
                        
                        // Remove old file from index
                        symbol_index.lock().remove_file(&old_uri);
                        
                        // Queue new file for indexing
                        pending_changes.lock().insert(new_path.clone());
                        tracing::debug!("File renamed: {:?} -> {:?}", old_path, new_path);
                    }
                }
            }
            _ => {
                // Ignore other events (directories, metadata changes, etc.)
            }
        }
    }
    
    fn maybe_flush_pending(
        doc_sync: &Arc<parking_lot::Mutex<super::DocumentSync>>,
        symbol_index: &Arc<parking_lot::Mutex<super::SymbolIndex>>,
        pending_changes: &Arc<parking_lot::Mutex<HashSet<PathBuf>>>,
        last_flush: &Arc<parking_lot::Mutex<Instant>>,
        debounce_ms: u64,
    ) {
        let should_flush = {
            let last = last_flush.lock();
            last.elapsed() >= Duration::from_millis(debounce_ms)
        };
        
        if should_flush {
            let changes: Vec<PathBuf> = {
                let mut pending = pending_changes.lock();
                let changes = pending.drain().collect();
                changes
            };
            
            if !changes.is_empty() {
                tracing::info!("Flushing {} pending file changes", changes.len());
                
                // Process each changed file
                for path in changes {
                    if let Err(e) = Self::reindex_file(doc_sync, symbol_index, &path) {
                        tracing::error!("Failed to reindex {:?}: {}", path, e);
                    }
                }
                
                *last_flush.lock() = Instant::now();
            }
        }
    }
    
    fn reindex_file(
        doc_sync: &Arc<parking_lot::Mutex<super::DocumentSync>>,
        symbol_index: &Arc<parking_lot::Mutex<super::SymbolIndex>>,
        path: &Path,
    ) -> Result<()> {
        #[cfg(feature = "cst_integration")]
        {
            use lapce_tree_sitter::LanguageRegistry;
            
            let uri = Self::path_to_uri(path);
            
            // Read file content
            let content = std::fs::read_to_string(path)?;
            
            // Detect language
            let language_id = LanguageRegistry::instance()
                .for_path(path)
                .map(|lang| lang.name.to_string())
                .unwrap_or_else(|_| "plaintext".to_string());
            
            // Parse with tree-sitter
            let mut doc_sync_guard = doc_sync.lock();
            
            // Open document (this will parse it)
            let result = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(doc_sync_guard.did_open(&uri, &language_id, &content));
            
            if let Err(e) = result {
                return Err(anyhow!("Failed to parse {}: {}", uri, e));
            }
            
            // Get the parsed tree
            if let (Some(tree), Some(text)) = (doc_sync_guard.get_tree(&uri), doc_sync_guard.get_text(&uri)) {
                // Update symbol index
                let mut index = symbol_index.lock();
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(index.update_file(&uri, &language_id, tree, text.as_bytes()))?;
                
                tracing::debug!("Reindexed file: {}", uri);
            }
            
            Ok(())
        }
        
        #[cfg(not(feature = "cst_integration"))]
        {
            tracing::warn!("File reindexing not available without cst_integration feature");
            Ok(())
        }
    }
    
    fn is_source_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            matches!(
                ext_str.as_str(),
                "rs" | "js" | "ts" | "tsx" | "jsx" | "py" | "go" | "java" | 
                "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "cs" | "rb" | 
                "php" | "swift" | "kt" | "scala" | "lua" | "vim" | "el" | 
                "hs" | "ml" | "fs" | "clj" | "ex" | "exs" | "erl" | "hrl" |
                "toml" | "yaml" | "yml" | "json" | "xml" | "html" | "css"
            )
        } else {
            // Check for files without extensions (Makefile, Dockerfile, etc.)
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy().to_lowercase();
                matches!(
                    name_str.as_str(),
                    "makefile" | "dockerfile" | "cmakelists.txt" | "rakefile" | 
                    "gemfile" | "podfile" | "brewfile"
                )
            } else {
                false
            }
        }
    }
    
    fn path_to_uri(path: &Path) -> String {
        format!("file://{}", path.to_string_lossy())
    }
    
    /// Reset backoff mode (call this after period of low activity)
    pub fn reset_backoff(&self) {
        *self.in_backoff.lock() = false;
        *self.event_count.lock() = 0;
        tracing::info!("Backoff mode reset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_source_file() {
        assert!(FileSystemWatcher::is_source_file(Path::new("test.rs")));
        assert!(FileSystemWatcher::is_source_file(Path::new("test.js")));
        assert!(FileSystemWatcher::is_source_file(Path::new("Makefile")));
        assert!(FileSystemWatcher::is_source_file(Path::new("Dockerfile")));
        assert!(!FileSystemWatcher::is_source_file(Path::new("test.txt")));
        assert!(!FileSystemWatcher::is_source_file(Path::new("test.md")));
    }
    
    #[test]
    fn test_path_to_uri() {
        let path = Path::new("/home/user/project/file.rs");
        let uri = FileSystemWatcher::path_to_uri(path);
        assert!(uri.starts_with("file://"));
        assert!(uri.contains("file.rs"));
    }
}
