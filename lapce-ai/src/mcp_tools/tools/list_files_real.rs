use async_trait::async_trait;
use std::path::Path;
use serde_json::{json, Value};
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ToolParameter},
    permissions::Permission,
};
pub struct ListFilesTool;
impl ListFilesTool {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "listFiles"
    }
    
    fn description(&self) -> &str {
        "List files in a directory"
    }
    
    fn parameters(&self) -> Vec<ToolParameter> { vec![] }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list (defaults to current directory)"
                }
            }
        })
    }
    
    async fn validate(&self, _args: &Value) -> Result<()> {
        // Path is optional, defaults to "."
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        // REAL IMPLEMENTATION - Actually list directories
        let path_str = args.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or(".");
        
        // Build full path
        let path = Path::new(path_str);
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            context.workspace.join(path)
        };
        // Security check
        if full_path.starts_with("/etc") || 
           full_path.starts_with("/sys") || 
           full_path.starts_with("/proc") ||
           path_str.contains("..") {
            return Ok(ToolResult {
                success: false,
                output: json!({ "error": "Access denied: cannot list system directories" }),
                error: Some(json!("Permission denied")),
            });
        }
        
        // Check if path exists
        if !full_path.exists() {
            return Ok(ToolResult {
                success: false,
                output: json!({ "error": format!("Path not found: {}", full_path.display()) }),
                error: Some(json!("Path not found")),
            });
        }
        
        // Check if it's a directory
        if !full_path.is_dir() {
            return Ok(ToolResult {
                success: false,
                output: json!({ "error": "Path is not a directory" }),
                error: Some(json!("Not a directory")),
            });
        }
        // ACTUALLY LIST THE DIRECTORY
        let mut entries = Vec::new();
        match tokio::fs::read_dir(&full_path).await {
            Ok(mut dir) => {
                while let Some(entry) = dir.next_entry().await? {
                    match entry.metadata().await {
                        Ok(metadata) => {
                            let file_type = if metadata.is_dir() {
                                "directory"
                            } else if metadata.is_symlink() {
                                "symlink"
                            } else {
                                "file"
                            };
                            
                            entries.push(json!({
                                "name": entry.file_name().to_string_lossy(),
                                "path": entry.path().display().to_string(),
                                "type": file_type,
                                "size": metadata.len(),
                                "modified": metadata.modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs()),
                            }));
                        },
                        Err(e) => {
                            // Include files we can't stat with error info
                            entries.push(json!({
                                "name": entry.file_name().to_string_lossy(),
                                "path": entry.path().display().to_string(),
                                "type": "unknown",
                                "error": format!("Cannot read metadata: {}", e)
                            }));
                        }
                    }
                }
                // Sort entries: directories first, then files, alphabetically
                entries.sort_by(|a, b| {
                    let a_type = a.get("type").and_then(|t| t.as_str()).unwrap_or("file");
                    let b_type = b.get("type").and_then(|t| t.as_str()).unwrap_or("file");
                    let a_name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let b_name = b.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    
                    if a_type == "directory" && b_type != "directory" {
                        std::cmp::Ordering::Less
                    } else if a_type != "directory" && b_type == "directory" {
                        std::cmp::Ordering::Greater
                    } else {
                        a_name.cmp(b_name)
                    }
                });
                
                Ok(ToolResult::success(json!({
                    "directory": full_path.display().to_string(),
                    "count": entries.len(),
                    "files": entries
                })))
            },
            Err(e) => {
                Ok(ToolResult {
                    success: false,
                    output: json!({ 
                        "error": format!("Failed to list directory: {}", e),
                        "path": full_path.display().to_string()
                    }),
                    error: Some(json!(e.to_string())),
                })
            }
        }
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileRead("*".to_string())]
    }
}
