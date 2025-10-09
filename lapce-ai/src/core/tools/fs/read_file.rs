// ReadFile tool implementation - P0-4

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::xml_util::{XmlParser};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::fs;
use std::io::{BufRead, BufReader};
use anyhow::Result;

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "readFile"
    }
    
    fn description(&self) -> &'static str {
        "Read contents of a file with optional line range"
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        // Parse XML arguments
        let parser = XmlParser::new();
        let parsed = parser.parse(args.as_str().ok_or_else(|| {
            ToolError::InvalidArguments("Expected XML string".to_string())
        })?).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        
        // Extract arguments - handle both flat and nested structures
        let tool_data = super::extract_tool_data(&parsed);
        
        let path = tool_data["path"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;
        
        let line_start = tool_data.get("lineStart")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<usize>().ok());
            
        let line_end = tool_data.get("lineEnd")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<usize>().ok());
        
        // Resolve path
        let file_path = context.resolve_path(path);
        
        // Check .rooignore
        if !context.is_path_allowed(&file_path) {
            return Err(ToolError::RooIgnoreBlocked(format!(
                "Path '{}' is blocked by .rooignore",
                file_path.display()
            )));
        }
        
        // Check permissions
        if !context.can_read_file(&file_path).await {
            return Err(ToolError::PermissionDenied(format!(
                "Permission denied to read file: {}",
                file_path.display()
            )));
        }
        
        // Ensure path is within workspace
        let safe_path = super::ensure_workspace_path(&context.workspace, &file_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Check if file exists
        if !safe_path.exists() {
            return Err(ToolError::Other(format!(
                "File '{}' does not exist",
                path
            )));
        }
        
        // Check if it's a binary file
        if super::is_binary_file(&safe_path) {
            if super::is_image_file(&safe_path) {
                return Ok(ToolOutput::success(json!({
                    "type": "image",
                    "path": path,
                    "message": "Image file detected. Use appropriate image viewer."
                })));
            }
            return Ok(ToolOutput::success(json!({
                "type": "binary",
                "path": path,
                "message": "Binary file detected. Cannot display content."
            })));
        }
        
        // Read file content
        let content = if let (Some(start), Some(end)) = (line_start, line_end) {
            read_lines(&safe_path, start, end)?
        } else {
            fs::read_to_string(&safe_path)
                .map_err(|e| ToolError::Io(e))?
        };
        
        Ok(ToolOutput::success(json!({
            "path": path,
            "content": content,
            "lineStart": line_start,
            "lineEnd": line_end,
        })))
    }
}

fn read_lines(path: &PathBuf, start: usize, end: usize) -> Result<String, ToolError> {
    let file = fs::File::open(path).map_err(|e| ToolError::Io(e))?;
    let reader = BufReader::new(file);
    
    let mut lines = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line_num = idx + 1;
        if line_num >= start && line_num <= end {
            let line = line.map_err(|e| ToolError::Io(e))?;
            lines.push(format!("{:4} | {}", line_num, line));
        }
        if line_num > end {
            break;
        }
    }
    
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    
    #[tokio::test]
    async fn test_read_file_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();
        writeln!(file, "Line 3").unwrap();
        
        let tool = ReadFileTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert!(result.result["content"].as_str().unwrap().contains("Line 1"));
        assert!(result.result["content"].as_str().unwrap().contains("Line 3"));
    }
    
    #[tokio::test]
    async fn test_read_file_with_line_range() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=10 {
            writeln!(file, "Line {}", i).unwrap();
        }
        
        let tool = ReadFileTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <lineStart>3</lineStart>
                <lineEnd>5</lineEnd>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let content = result.result["content"].as_str().unwrap();
        assert!(content.contains("Line 3"));
        assert!(content.contains("Line 5"));
        assert!(!content.contains("Line 1"));
        assert!(!content.contains("Line 6"));
    }
    #[tokio::test]
    async fn test_read_binary_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("binary.bin");
        
        let mut file = std::fs::File::create(&file_path).unwrap();
        use std::io::Write;
        file.write_all(&[0, 1, 2, 3, 255, 254]).unwrap();
        
        let tool = ReadFileTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <path>binary.bin</path>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["type"], "binary");
    }
}
