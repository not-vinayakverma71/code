use std::path::{Path, PathBuf};
use std::path::Path;
use anyhow::{Result, bail};
use serde_json::{json, Value};
use async_trait::async_trait;
use tokio::fs;
use walkdir::WalkDir;

use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ToolParameter},
    permissions::Permission,
};
pub struct SearchFilesTool;
impl SearchFilesTool {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "searchFiles"
    }
    
    fn description(&self) -> &str {
        "Search for patterns in files"
    }
    
    fn parameters(&self) -> Vec<ToolParameter> { vec![] }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "pattern": {"type": "string"},
                "case_sensitive": {"type": "boolean"},
                "max_results": {"type": "integer"}
            },
            "required": ["pattern"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("pattern").is_none() {
            bail!("Missing required parameter: pattern");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        // REAL IMPLEMENTATION - Actually search files with pattern matching
        let pattern = args.get("pattern")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: pattern"))?;
        
        let path_str = args.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or(".");
            
        let case_sensitive = args.get("case_sensitive")
            .and_then(|c| c.as_bool())
            .unwrap_or(true);
        let max_results = args.get("max_results")
            .and_then(|m| m.as_u64())
            .unwrap_or(1000) as usize;
        // Build full path
        let path = Path::new(path_str);
        let search_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            context.workspace.join(path)
        };
        // Security check
        if search_path.starts_with("/etc") || 
           search_path.starts_with("/sys") || 
           search_path.starts_with("/proc") ||
           path_str.contains("..") {
            return Ok(ToolResult {
                success: false,
                output: json!({ "error": "Access denied: cannot search system directories" }),
                error: Some(json!("Permission denied")),
            });
        }
        
        // Check if path exists
        if !search_path.exists() {
            return Ok(ToolResult {
                success: false,
                output: json!({ "error": format!("Path not found: {}", search_path.display()) }),
                error: Some(json!("Path not found")),
            });
        }
        // ACTUALLY SEARCH FILES
        let mut matches = Vec::new();
        let mut files_searched = 0;
        let mut files_matched = 0;
        // Use walkdir to traverse directory
        let walker = WalkDir::new(&search_path)
            .follow_links(false)
            .max_depth(10); // Limit depth to prevent infinite recursion
        for entry in walker {
            if matches.len() >= max_results {
                break;
            }
            match entry {
                Ok(entry) => {
                    // Skip directories and non-files
                    if !entry.file_type().is_file() {
                        continue;
                    }
                    
                    let file_path = entry.path();
                    // Skip binary files and very large files
                    if let Ok(metadata) = file_path.metadata() {
                        if metadata.len() > 10_000_000 { // Skip files > 10MB
                            continue;
                        }
                    }
                    
                    // Read and search file
                    match tokio::fs::read_to_string(file_path).await {
                        Ok(content) => {
                            files_searched += 1;
                            let mut file_has_match = false;
                            
                            for (line_num, line) in content.lines().enumerate() {
                                if matches.len() >= max_results {
                                    break;
                                }
                                
                                let contains_match = if case_sensitive {
                                    line.contains(pattern)
                                } else {
                                    line.to_lowercase().contains(&pattern.to_lowercase())
                                };
                                if contains_match {
                                    if !file_has_match {
                                        file_has_match = true;
                                        files_matched += 1;
                                    }
                                    
                                    // Highlight the match in the line
                                    let highlighted = if case_sensitive {
                                        line.replace(pattern, &format!("**{}**", pattern))
                                    } else {
                                        // Case-insensitive highlighting is more complex
                                        line.to_string()
                                    };
                                    matches.push(json!({
                                        "file": file_path.display().to_string(),
                                        "line_number": line_num + 1,
                                        "content": line.trim(),
                                        "highlighted": highlighted.trim()
                                    }));
                                }
                            }
                        },
                        Err(_) => {
                            // Skip files we can't read (binary, permissions, etc.)
                            continue;
                        }
                    }
                },
                Err(_) => {
                    // Skip entries we can't access
                    continue;
                }
            }
        }
        
        Ok(ToolResult::success(json!({
            "pattern": pattern,
            "path": search_path.display().to_string(),
            "case_sensitive": case_sensitive,
            "files_searched": files_searched,
            "files_matched": files_matched,
            "matches": matches,
            "match_count": matches.len(),
            "truncated": matches.len() >= max_results
        })))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileRead("*".to_string())]
    }
}
