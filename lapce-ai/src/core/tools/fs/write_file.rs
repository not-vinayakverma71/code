// WriteFile tool implementation - P0-5

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError, ApprovalRequired};
use crate::core::tools::xml_util::XmlParser;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "writeFile"
    }
    
    fn description(&self) -> &'static str {
        "Write content to a file (requires approval)"
    }
    
    fn requires_approval(&self) -> bool {
        true
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
            
        let content = tool_data["content"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' argument".to_string()))?;
            
        let create_dirs = tool_data.get("createDirs")
            .and_then(|v| v.as_str())
            .map(|s| s == "true")
            .unwrap_or(false);
        
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
        if !context.can_write_file(&file_path).await {
            return Err(ToolError::PermissionDenied(format!(
                "Permission denied to write file: {}",
                file_path.display()
            )));
        }
        
        // Ensure path is within workspace
        let safe_path = super::ensure_workspace_path(&context.workspace, &file_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Check if file exists for approval message
        let exists = safe_path.exists();
        let operation = if exists { "overwrite" } else { "create" };
        
        // Request approval if required
        if context.require_approval && !context.dry_run {
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required to {} file: {}",
                operation, path
            )));
        }
        
        // Dry run mode - don't actually write
        if context.dry_run {
            return Ok(ToolOutput::success(json!({
                "path": path,
                "operation": operation,
                "dryRun": true,
                "wouldWrite": content.len(),
            })));
        }
        
        // Create parent directories if requested
        if create_dirs {
            if let Some(parent) = safe_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| ToolError::Io(e))?;
            }
        }
        
        // Pre-process content (strip fence markers, line numbers)
        let processed_content = preprocess_content(content);
        
        // Write the file
        fs::write(&safe_path, &processed_content)
            .map_err(|e| ToolError::Io(e))?;
        
        Ok(ToolOutput::success(json!({
            "path": path,
            "operation": operation,
            "bytesWritten": processed_content.len(),
        })))
    }
}

/// Pre-process content to remove code fences and line numbers
fn preprocess_content(content: &str) -> String {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    
    for line in content.lines() {
        // Check for code fence
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            continue; // Skip fence lines
        }
        
        // Remove line numbers (format: "   123 | actual content" or "123: actual content")
        let processed = if let Some(pos) = line.find(" | ") {
            // Check if before " | " is a line number
            let prefix = &line[..pos];
            if prefix.trim().parse::<u32>().is_ok() {
                &line[pos + 3..] // Skip the line number and " | "
            } else {
                line
            }
        } else if let Some(pos) = line.find(": ") {
            let prefix = &line[..pos];
            if prefix.trim().parse::<u32>().is_ok() && pos < 10 {
                &line[pos + 2..] // Skip the line number and ": "
            } else {
                line
            }
        } else {
            line
        };
        
        lines.push(processed);
    }
    
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_preprocess_content() {
        let input = r#"```rust
fn main() {
    println!("Hello");
}
```

Here's some code:
   1 | fn test() {
   2 |     let x = 5;
   3 | }

1: first line
2: second line"#;
        
        let output = preprocess_content(input);
        
        assert!(!output.contains("```"));
        assert!(output.contains("fn test() {"));
        assert!(output.contains("let x = 5;"));
        assert!(output.contains("first line"));
        assert!(!output.contains("   1 |"));
        assert!(!output.contains("2:"));
    }
    
    #[tokio::test]
    async fn test_write_file_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let tool = WriteFileTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.dry_run = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>Hello, world!</content>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["dryRun"], true);
        
        // File should not be created in dry run
        assert!(!file_path.exists());
    }
    
    #[tokio::test]
    async fn test_write_file_create_dirs() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = WriteFileTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false; // Disable approval for test
        
        let args = json!(format!(r#"
            <tool>
                <path>nested/dir/test.txt</path>
                <content>Content here</content>
                <createDirs>true</createDirs>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        // Check file was created
        let file_path = temp_dir.path().join("nested/dir/test.txt");
        assert!(file_path.exists());
        
        let content = fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "Content here");
    }
    
    #[tokio::test]
    async fn test_write_file_requires_approval() {
        let temp_dir = TempDir::new().unwrap();
        
        let tool = WriteFileTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>Content</content>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        
        if let Err(ToolError::ApprovalRequired(msg)) = result {
            assert!(msg.contains("Approval required"));
        } else {
            panic!("Expected ApprovalRequired error");
        }
    }
}
