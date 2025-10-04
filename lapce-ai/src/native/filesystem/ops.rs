use std::sync::Arc;
use std::time::SystemTime;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use tokio::fs;
use std::path::Path;
use memmap2::MmapOptions;
use std::fs::File;
use dashmap::DashMap;
use tokio::sync::RwLock;
use walkdir::WalkDir;
use regex::Regex;

use std::path::PathBuf;
// Native filesystem operations - no longer depends on MCP tools
use std::fmt;
use serde::{Serialize, Deserialize};

// Define types locally to avoid MCP dependencies
pub type JsonSchema = Value;

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub workspace: PathBuf,
    pub user_id: String,
    pub session_id: String,
}
// FileSystemTool from lines 103-201
#[derive(Clone)]
pub struct FileContent {
    pub content: String,
    pub modified: SystemTime,
}
pub struct FileCache {
    cache: Arc<DashMap<PathBuf, FileContent>>,
    max_size: usize,
}
impl FileCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_size,
        }
    }
    
    pub async fn get(&self, path: &Path) -> Option<FileContent> {
        self.cache.get(path).map(|entry| entry.clone())
    }
    
    pub async fn put(&self, path: PathBuf, content: FileContent) {
        // Simple LRU: if cache is too big, remove oldest
        if self.cache.len() >= self.max_size {
            // Find oldest entry
            let mut oldest_path = None;
            let mut oldest_time = SystemTime::now();
            
            for entry in self.cache.iter() {
                if entry.value().modified < oldest_time {
                    oldest_time = entry.value().modified;
                    oldest_path = Some(entry.key().clone());
                }
            }
            if let Some(path) = oldest_path {
                self.cache.remove(&path);
            }
        }
        
        self.cache.insert(path, content);
    }
}

// Removed duplicate - FileSystemOperations is defined as an alias below

pub struct FileSystemGuard {
    allowed_paths: Vec<PathBuf>,
}

impl FileSystemGuard {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            allowed_paths: vec![workspace],
        }
    }
    
    pub fn check_read_permission(&self, path: &Path, _user: &str) -> Result<()> {
        // Check if path is within allowed paths
        for allowed in &self.allowed_paths {
            if path.starts_with(allowed) {
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("Access denied to path: {:?}", path))
    }
}

pub struct FileSystemTool {
    fs_guard: Arc<FileSystemGuard>,
    cache: Arc<FileCache>,
}

impl FileSystemTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            fs_guard: Arc::new(FileSystemGuard::new(workspace)),
            cache: Arc::new(FileCache::new(100)), // Cache up to 100 files
        }
    }
    
    fn resolve_path(&self, path: &Value, workspace: &Path) -> Result<PathBuf> {
        let path_str = path.as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path parameter"))?;
        let path = if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            workspace.join(path_str)
        };
        Ok(path)
    }
    
    async fn read_file(&self, args: Value, context: ToolContext) -> Result<Value> {
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        // Check permissions
        self.fs_guard.check_read_permission(&path, &context.user_id)?;
        // Try cache first
        if let Some(cached) = self.cache.get(&path).await {
            return Ok(json!({
                "content": cached.content,
                "cached": true
            }));
        }
        
        // Read file efficiently
        let metadata = fs::metadata(&path).await?;
        let content = if metadata.len() > 1_000_000 {
            // Use memory-mapped I/O for large files
            let file = File::open(&path)?;
            let mmap = unsafe { MmapOptions::new().map(&file)? };
            String::from_utf8_lossy(&mmap).into_owned()
        } else {
            tokio::fs::read_to_string(&path).await?
        };
        
        // Cache the content
        self.cache.put(path.clone(), FileContent {
            content: content.clone(),
            modified: SystemTime::now(),
        }).await;
        Ok(json!({
            "content": content,
            "path": path.display().to_string()
        }))
    }
    
    async fn write_file(&self, args: Value, context: ToolContext) -> Result<Value> {
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        let content = args["content"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid content parameter"))?;
        // Write file
        tokio::fs::write(&path, content).await?;
        // Update cache
        self.cache.put(path.clone(), FileContent {
            content: content.to_string(),
            modified: SystemTime::now(),
        }).await;
        
        Ok(json!({
            "path": path.display().to_string(),
            "bytes_written": content.len()
        }))
    }
    
    async fn list_directory(&self, args: Value, context: ToolContext) -> Result<Value> {
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            entries.push(json!({
                "name": entry.file_name().to_string_lossy(),
                "path": entry.path().display().to_string(),
                "is_file": metadata.is_file(),
                "is_dir": metadata.is_dir(),
                "size": metadata.len(),
            }));
        }
        
        Ok(json!({
            "path": path.display().to_string(),
            "entries": entries
        }))
    }
    
    async fn search_files(&self, args: Value, context: ToolContext) -> Result<Value> {
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        let pattern = args["pattern"].as_str()
            .ok_or_else(|| anyhow::anyhow!("pattern required"))?;
        let regex = Regex::new(pattern)?;
        let mut matches = Vec::new();
        // Walk directory with ignore rules
        for entry in WalkDir::new(&path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                    for (line_num, line) in content.lines().enumerate() {
                        if regex.is_match(line) {
                            matches.push(json!({
                                "file": entry.path().display().to_string(),
                                "line": line_num,
                                "content": line
                            }));
                        }
                    }
                }
            }
        }
        
        Ok(json!({ "matches": matches }))
    }
    
    async fn watch_changes(&self, _args: Value, _context: ToolContext) -> Result<Value> {
        // This would use FileWatcher in production
        Ok(json!({
            "status": "watching",
            "message": "File watching started"
        }))
    }
}

// Type alias for compatibility
pub type FileSystemOperations = FileSystemTool;

// Native implementation - no MCP Tool trait needed
impl FileSystemTool {
    pub async fn execute(&self, args: Value, context: ToolContext) -> Result<Value> {
        let operation = args["operation"].as_str()
            .ok_or_else(|| anyhow::anyhow!("operation required"))?;
        match operation {
            "read" => self.read_file(args, context).await,
            "write" => self.write_file(args, context).await,
            "list" => self.list_directory(args, context).await,
            "search" => self.search_files(args, context).await,
            "watch" => self.watch_changes(args, context).await,
            _ => Err(anyhow::anyhow!("Unknown operation: {}", operation)),
        }
    }
}
