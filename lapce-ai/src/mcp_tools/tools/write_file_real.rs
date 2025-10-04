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
pub struct WriteFileTool;
impl WriteFileTool {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "writeFile"
    
    fn description(&self) -> &str {
        "Write content to a file"
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("path").is_none() || args.get("content").is_none() {
            bail!("Missing required parameters: path and content");
        }
        Ok(())
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        // REAL IMPLEMENTATION - Actually write files to disk
        let path_str = args.get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path"))?;
        let content = args.get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: content"))?;
        
        // Build full path  
        let path = Path::new(path_str);
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            context.workspace.join(path)
        };
        // Security check - prevent writing to sensitive system files
        if full_path.starts_with("/etc") || 
           full_path.starts_with("/sys") || 
           full_path.starts_with("/proc") ||
           full_path.starts_with("/boot") ||
           path_str.contains("..") {
            return Ok(ToolResult {
                success: false,
                data: Some(json!({ "error": "Access denied: cannot write to system directories" })),
                error: Some("Permission denied".to_string()),
            });
        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                match tokio::fs::create_dir_all(parent).await {
                    Ok(_) => {},
                    Err(e) => {
                        return Ok(ToolResult {
                            success: false,
                            data: Some(json!({ "error": format!("Failed to create directory: {}", e) })),
                            error: Some(e.to_string()),
                        });
                    }
            }
        // Check if file exists and get its size for reporting
        let file_existed = full_path.exists();
        let old_size = if file_existed {
            std::fs::metadata(&full_path).ok().map(|m| m.len())
            None
        // ACTUALLY WRITE THE FILE
        match tokio::fs::write(&full_path, content.as_bytes()).await {
            Ok(_) => {
                // Get new file size
                let new_size = content.len();
                
                Ok(ToolResult::success(json!({
                    "path": full_path.display().to_string(),
                    "bytes_written": new_size,
                    "created": !file_existed,
                    "old_size": old_size,
                    "new_size": new_size,
                })))
            Err(e) => {
                Ok(ToolResult {
                    success: false,
                    data: Some(json!({ 
                        "error": format!("Failed to write file: {}", e),
                        "path": full_path.display().to_string()
                    })),
                    error: Some(e.to_string()),
                })
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileWrite("*".to_string())]
