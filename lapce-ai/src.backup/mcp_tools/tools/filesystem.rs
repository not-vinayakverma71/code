use std::sync::Arc;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use anyhow::Result;
use serde_json::json;
use serde_json::Value;
use tokio::fs;
use memmap2::MmapOptions;
use std::fs::File;

use crate::mcp_tools::{
    core::{ToolContext, ToolResult, ToolParameter, ResourceLimits},
    permissions::Permission,
    filesystem_guard::FileSystemGuard,
    cache::FileCache,
};

// Tool placeholder implementations
pub struct ReadFileTool;
impl ReadFileTool {
    pub fn new() -> Self { Self }
}

pub struct WriteFileTool;
impl WriteFileTool {
    pub fn new() -> Self { Self }
}

pub struct EditFileTool;
impl EditFileTool {
    pub fn new() -> Self { Self }
}

pub struct ListFilesTool;
impl ListFilesTool {
    pub fn new() -> Self { Self }
}

pub struct SearchFilesTool;
impl SearchFilesTool {
    pub fn new() -> Self { Self }
}

pub struct ExecuteCommandTool;
impl ExecuteCommandTool {
    pub fn new() -> Self { Self }
}

pub struct InsertContentTool;
impl InsertContentTool {
    pub fn new() -> Self { Self }
}

pub struct SearchAndReplaceTool;
impl SearchAndReplaceTool {
    pub fn new() -> Self { Self }
}

pub struct ListCodeDefinitionsTool;
impl ListCodeDefinitionsTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl crate::mcp_tools::core::Tool for ListCodeDefinitionsTool {
    fn name(&self) -> &str {
        "list_code_definitions"
    }
    
    fn description(&self) -> &str {
        "List code definitions in a file"
    }
    
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            }
        })
    }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> Result<ToolResult> {
        Ok(crate::mcp_tools::core::ToolResult {
            success: true,
            error: None,
            data: Some(serde_json::json!({
                "definitions": []
            })),
            metadata: None,
        })
    }
}

pub struct NewTaskTool;
impl NewTaskTool {
    pub fn new() -> Self { Self }
}

pub struct UpdateTodoListTool;
impl UpdateTodoListTool {
    pub fn new() -> Self { Self }
}

pub struct AttemptCompletionTool;
impl AttemptCompletionTool {
    pub fn new() -> Self { Self }
}

pub struct AskFollowupQuestionTool;
impl AskFollowupQuestionTool {
    pub fn new() -> Self { Self }
}

pub struct FileSystemTool {
    fs_guard: Arc<FileSystemGuard>,
    cache: Arc<FileCache>,
}

impl FileSystemTool {
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            fs_guard: Arc::new(FileSystemGuard::new(workspace.clone())),
            cache: Arc::new(FileCache::new()),
        }
    }
    
    fn resolve_path(&self, path_value: &Value, workspace: &Path) -> Result<PathBuf> {
        let path_str = path_value.as_str()
            .ok_or_else(|| anyhow::anyhow!("path must be a string"))?;
        Ok(workspace.join(path_str))
    }
    
    async fn read_file(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        
        // Check permissions
        self.fs_guard.check_read_permission(&path, &context.user)?;
        
        // Try cache first
        if let Some(cached) = self.cache.get(&path).await {
            return Ok(ToolResult::success(json!({
                "content": cached.content,
                "cached": true
            })));
        }
        
        // Read file efficiently
        let content = if path.metadata()?.len() > 1_000_000 {
            // Use memory-mapped I/O for large files
            let file = File::open(&path)?;
            let mmap = unsafe { MmapOptions::new().map(&file)? };
            String::from_utf8_lossy(&mmap).into_owned()
        } else {
            tokio::fs::read_to_string(&path).await?
        };
        
        // Cache the content
        self.cache.put(path.clone(), crate::mcp_tools::cache::FileContent {
            content: content.clone(),
            modified: std::time::SystemTime::now(),
        }).await;
        
        Ok(ToolResult::success(json!({
            "content": content,
            "path": path.display().to_string()
        })))
    }
    
    async fn write_file(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        let content = args["content"].as_str()
            .ok_or_else(|| anyhow::anyhow!("content required"))?;
        
        // Check permissions
        self.fs_guard.check_write_permission(&path, &context.user)?;
        
        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Write file
        fs::write(&path, content).await?;
        
        // Invalidate cache
        self.cache.invalidate(&path).await;
        
        Ok(ToolResult::success(json!({
            "path": path.display().to_string(),
            "bytes_written": content.len()
        })))
    }
    
    async fn list_directory(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        
        // Check permissions
        self.fs_guard.check_read_permission(&path, &context.user)?;
        
        let mut entries = Vec::new();
        let mut dir_stream = fs::read_dir(&path).await?;
        
        while let Some(entry) = dir_stream.next_entry().await? {
            let file_type = entry.file_type().await?;
            let metadata = entry.metadata().await?;
            
            entries.push(json!({
                "name": entry.file_name().to_string_lossy(),
                "path": entry.path().display().to_string(),
                "is_file": file_type.is_file(),
                "is_dir": file_type.is_dir(),
                "size": metadata.len(),
            }));
        }
        
        Ok(ToolResult::success(json!({
            "path": path.display().to_string(),
            "entries": entries
        })))
    }
    
    async fn search_files(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let pattern = args["pattern"].as_str()
            .ok_or_else(|| anyhow::anyhow!("pattern required"))?;
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        
        // Use ripgrep integration
        let searcher = crate::mcp_tools::ripgrep_search::RipgrepSearch::new();
        let results = searcher.search(
            pattern,
            &path,
            args["case_sensitive"].as_bool().unwrap_or(false),
            args["whole_word"].as_bool().unwrap_or(false),
            None,
        ).await?;
        
        let matches: Vec<_> = results.iter()
            .map(|r| r.to_json())
            .collect();
        
        Ok(ToolResult::success(json!({
            "pattern": pattern,
            "path": path.display().to_string(),
            "matches": matches
        })))
    }
    
    async fn watch_changes(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let path = self.resolve_path(&args["path"], &context.workspace)?;
        
        // Check permissions
        self.fs_guard.check_read_permission(&path, &context.user)?;
        
        Ok(ToolResult::success(json!({
            "watching": path.display().to_string(),
            "status": "File watching registered"
        })))
    }
    
    fn resolve_path_v2(&self, path_value: &Value, workspace: &Path) -> Result<PathBuf> {
        let path_str = path_value.as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path parameter"))?;
        
        let path = if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            workspace.join(path_str)
        };
        
        Ok(path)
    }
}

#[async_trait]
impl crate::mcp_tools::core::Tool for FileSystemTool {
    fn name(&self) -> &str {
        "fs_operations"
    }
    
    fn description(&self) -> &str {
        "File system operations with caching and permission checks"
    }
    
    fn parameters(&self) -> Vec<ToolParameter> {
        vec![]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "list", "search", "watch"]
                },
                "path": {
                    "type": "string"
                },
                "content": {
                    "type": "string"
                },
                "pattern": {
                    "type": "string"
                },
                "case_sensitive": {
                    "type": "boolean"
                },
                "whole_word": {
                    "type": "boolean"
                }
            },
            "required": ["operation"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        let operation = args["operation"].as_str()
            .ok_or_else(|| anyhow::anyhow!("operation required"))?;
        
        match operation {
            "read" | "list" | "watch" => {
                if args["path"].is_null() {
                    return Err(anyhow::anyhow!("path required for {}", operation));
                }
            }
            "write" => {
                if args["path"].is_null() || args["content"].is_null() {
                    return Err(anyhow::anyhow!("path and content required for write"));
                }
            }
            "search" => {
                if args["path"].is_null() || args["pattern"].is_null() {
                    return Err(anyhow::anyhow!("path and pattern required for search"));
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown operation: {}", operation)),
        }
        
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
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
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![
            Permission::FileRead("*".to_string()),
            Permission::FileWrite("*".to_string()),
        ]
    }
    
    fn resource_limits(&self) -> ResourceLimits {
        ResourceLimits {
            max_memory_mb: 100,
            max_cpu_seconds: 10,
            max_file_size_mb: 100,
            max_concurrent_ops: 10,
        }
    }
}
