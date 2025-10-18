// List files tool implementation
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput};

pub struct ListFilesTool;

#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &'static str {
        "listFiles"
    }
    
    fn description(&self) -> &'static str {
        "List files in a directory"
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        let path = args["path"].as_str().unwrap_or(".");
        let recursive = args["recursive"].as_bool().unwrap_or(false);
        let max_depth = args["max_depth"].as_u64().unwrap_or(3) as usize;
        
        let full_path = context.workspace.join(path);
        
        let mut files = Vec::new();
        
        if recursive {
            for entry in WalkDir::new(&full_path)
                .max_depth(max_depth)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    let relative_path = entry.path()
                        .strip_prefix(&context.workspace)
                        .unwrap_or(entry.path());
                    files.push(relative_path.display().to_string());
                }
            }
        } else {
            if let Ok(entries) = std::fs::read_dir(&full_path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                        files.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }
        }
        
        Ok(ToolOutput {
            success: true,
            result: json!({
                "files": files,
                "count": files.len(),
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}
