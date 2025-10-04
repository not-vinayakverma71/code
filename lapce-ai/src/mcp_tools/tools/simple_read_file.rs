use std::sync::Arc;
use std::path::Path;
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use tokio::fs;

use crate::types_tool::ToolParameter;
use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits}, permissions::Permission};
// SimpleReadFileTool - basic file reading without caching
pub struct SimpleReadFileTool;
impl SimpleReadFileTool {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl Tool for SimpleReadFileTool {
    fn name(&self) -> &str {
        "simpleReadFileTool"
    }
    
    fn description(&self) -> &str {
        "Simple file reading without caching"
    }
    
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if args["path"].is_null() {
            return Err(anyhow::anyhow!("path is required"));
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let path_str = args["path"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path parameter"))?;
        
        let path = if std::path::Path::new(path_str).is_absolute() {
            std::path::PathBuf::from(&path_str)
        } else {
            context.workspace.join(path_str)
        };
        // Check if file exists
        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {:?}", path));
        }
        
        if !path.is_file() {
            return Err(anyhow::anyhow!("Path is not a file: {:?}", path));
        }
        
        // Read file content
        let content = fs::read_to_string(&path).await?;
        Ok(ToolResult::success(json!({
            "content": content,
            "path": path.display().to_string(),
            "size": content.len()
        })))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileRead("*".to_string())]
    }
}
