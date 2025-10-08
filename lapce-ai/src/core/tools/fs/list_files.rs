// ListFiles tool implementation - P0-4

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::xml_util::XmlParser;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use globset::{Glob, GlobSetBuilder};

pub struct ListFilesTool;

#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &'static str {
        "listFiles"
    }
    
    fn description(&self) -> &'static str {
        "List files and directories in a given path with optional glob filtering"
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        // Parse XML arguments
        let parser = XmlParser::new();
        let parsed = parser.parse(args.as_str().ok_or_else(|| {
            ToolError::InvalidArguments("Expected XML string".to_string())
        })?).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        
        // Extract arguments - handle both flat and nested structures
        let tool_data = super::extract_tool_data(&parsed);
        
        let path = tool_data.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        
        let pattern = tool_data.get("pattern")
            .and_then(|v| v.as_str());
            
        let recursive = tool_data.get("recursive")
            .and_then(|v| v.as_str())
            .map(|s| s == "true")
            .unwrap_or(false);
        
        // Resolve path
        let dir_path = context.resolve_path(path);
        
        // Check .rooignore
        if !context.is_path_allowed(&dir_path) {
            return Err(ToolError::RooIgnoreBlocked(format!(
                "Path '{}' is blocked by .rooignore",
                path
            )));
        }
        
        // Ensure path is within workspace
        let safe_path = super::ensure_workspace_path(&context.workspace, &dir_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Check if directory exists
        if !safe_path.exists() {
            return Err(ToolError::Other(format!(
                "Directory '{}' does not exist",
                path
            )));
        }
        
        if !safe_path.is_dir() {
            return Err(ToolError::Other(format!(
                "'{}' is not a directory",
                path
            )));
        }
        
        // Build glob matcher if pattern provided
        let glob_matcher = if let Some(pat) = pattern {
            let glob = Glob::new(pat)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid pattern: {}", e)))?;
            let mut builder = GlobSetBuilder::new();
            builder.add(glob);
            Some(builder.build().map_err(|e| ToolError::InvalidArguments(e.to_string()))?)
        } else {
            None
        };
        
        // List files
        let mut files = Vec::new();
        list_directory(&safe_path, &context, recursive, &glob_matcher, &mut files)?;
        
        // Sort files for consistent output
        files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        
        Ok(ToolOutput::success(json!({
            "path": path,
            "files": files,
            "count": files.len(),
        })))
    }
}

fn list_directory(
    dir: &std::path::Path,
    context: &ToolContext,
    recursive: bool,
    glob_matcher: &Option<globset::GlobSet>,
    files: &mut Vec<Value>,
) -> Result<(), ToolError> {
    let entries = fs::read_dir(dir).map_err(|e| ToolError::Io(e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| ToolError::Io(e))?;
        let path = entry.path();
        
        // Check .rooignore
        if !context.is_path_allowed(&path) {
            continue;
        }
        
        // Get relative path from workspace
        let rel_path = path.strip_prefix(&context.workspace)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        
        // Check glob pattern if provided
        if let Some(ref matcher) = glob_matcher {
            if !matcher.is_match(&rel_path) {
                // If recursive and this is a directory, still traverse it
                if recursive && path.is_dir() {
                    list_directory(&path, context, recursive, glob_matcher, files)?;
                }
                continue;
            }
        }
        
        let metadata = entry.metadata().map_err(|e| ToolError::Io(e))?;
        
        files.push(json!({
            "path": rel_path,
            "type": if metadata.is_dir() { "directory" } else { "file" },
            "size": if metadata.is_file() { metadata.len() } else { 0 },
        }));
        
        // Recurse into directories if requested
        if recursive && metadata.is_dir() {
            list_directory(&path, context, recursive, glob_matcher, files)?;
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};
    use std::io::Write;
    
    #[tokio::test]
    async fn test_list_files_basic() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        File::create(temp_dir.path().join("file1.txt")).unwrap();
        File::create(temp_dir.path().join("file2.rs")).unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        File::create(temp_dir.path().join("subdir/file3.txt")).unwrap();
        
        let tool = ListFilesTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(r#"
            <tool>
                <path>.</path>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let files = result.result["files"].as_array().unwrap();
        assert_eq!(files.len(), 3); // 2 files + 1 directory
    }
    
    #[tokio::test]
    async fn test_list_files_recursive() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create nested structure
        File::create(temp_dir.path().join("file1.txt")).unwrap();
        fs::create_dir(temp_dir.path().join("dir1")).unwrap();
        File::create(temp_dir.path().join("dir1/file2.txt")).unwrap();
        fs::create_dir(temp_dir.path().join("dir1/dir2")).unwrap();
        File::create(temp_dir.path().join("dir1/dir2/file3.txt")).unwrap();
        
        let tool = ListFilesTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(r#"
            <tool>
                <path>.</path>
                <recursive>true</recursive>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let files = result.result["files"].as_array().unwrap();
        assert_eq!(files.len(), 5); // 3 files + 2 directories
    }
    
    #[tokio::test]
    async fn test_list_files_with_pattern() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        File::create(temp_dir.path().join("file1.txt")).unwrap();
        File::create(temp_dir.path().join("file2.rs")).unwrap();
        File::create(temp_dir.path().join("test.txt")).unwrap();
        File::create(temp_dir.path().join("data.json")).unwrap();
        
        let tool = ListFilesTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(r#"
            <tool>
                <path>.</path>
                <pattern>*.txt</pattern>
            </tool>
        "#);
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let files = result.result["files"].as_array().unwrap();
        assert_eq!(files.len(), 2); // Only .txt files
        
        for file in files {
            assert!(file["path"].as_str().unwrap().ends_with(".txt"));
        }
    }
}
