// Insert Content Tool - P0-5

use crate::core::tools::traits::{Tool, ToolContext, ToolOutput, ToolResult, ToolError};
use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};
use super::ensure_workspace_path;
use async_trait::async_trait;

pub struct InsertContentTool;

#[async_trait]
impl Tool for InsertContentTool {
    fn name(&self) -> &'static str {
        "insertContent"
    }
    
    fn description(&self) -> &'static str {
        "Insert content at a specific position in a file (requires approval)"
    }
    
    fn requires_approval(&self) -> bool {
        true
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        // Parse XML/JSON arguments
        let parsed = crate::core::tools::util::xml::parse_tool_xml(args.as_str().unwrap_or(""))
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        
        let path = parsed.arguments["path"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;
            
        let content = parsed.arguments["content"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' argument".to_string()))?;
            
        let position = parsed.arguments.get("position")
            .and_then(|v| v.as_str())
            .unwrap_or("end");
            
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
        let safe_path = ensure_workspace_path(&context.workspace, &file_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // File must exist for insert operation
        if !safe_path.exists() {
            return Err(ToolError::InvalidArguments(format!(
                "File does not exist: {}",
                path
            )));
        }
        
        // Request approval if required
        if context.require_approval && !context.dry_run {
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required to insert content into file: {}",
                path
            )));
        }
        
        // Dry-run mode
        if context.dry_run {
            return Ok(ToolOutput::success(json!({
                "file": path,
                "position": position,
                "bytes_to_insert": content.len(),
                "dry_run": true,
            })));
        }
        
        // Read existing content
        let mut existing_content = String::new();
        {
            let mut file = fs::File::open(&safe_path)
                .map_err(|e| ToolError::Io(e))?;
            file.read_to_string(&mut existing_content)
                .map_err(|e| ToolError::Io(e))?;
        }
        
        // Compute insert position
        let insert_byte_pos = match position {
            "start" | "beginning" => 0,
            "end" => existing_content.len(),
            _ => {
                // Try to parse as line number or byte offset
                if let Some(line_str) = position.strip_prefix("line:") {
                    let line_num: usize = line_str.parse()
                        .map_err(|_| ToolError::InvalidArguments(format!("Invalid line number: {}", line_str)))?;
                    
                    // Find byte offset for line
                    existing_content.lines()
                        .take(line_num.saturating_sub(1))
                        .map(|l| l.len() + 1) // +1 for newline
                        .sum()
                } else if let Some(byte_str) = position.strip_prefix("byte:") {
                    byte_str.parse()
                        .map_err(|_| ToolError::InvalidArguments(format!("Invalid byte offset: {}", byte_str)))?
                } else {
                    // Default to line number
                    let line_num: usize = position.parse()
                        .map_err(|_| ToolError::InvalidArguments(format!("Invalid position: {}", position)))?;
                    
                    existing_content.lines()
                        .take(line_num.saturating_sub(1))
                        .map(|l| l.len() + 1)
                        .sum()
                }
            }
        };
        
        // Clamp to valid range
        let insert_pos = insert_byte_pos.min(existing_content.len());
        
        // Insert content
        let mut new_content = String::with_capacity(existing_content.len() + content.len() + 1);
        new_content.push_str(&existing_content[..insert_pos]);
        new_content.push_str(content);
        // For start position, ensure newline after content if it doesn't have one
        if (position == "start" || position == "beginning") && insert_pos == 0 {
            if !content.ends_with('\n') && !existing_content.is_empty() {
                new_content.push('\n');
            }
        }
        new_content.push_str(&existing_content[insert_pos..]);
        
        // Write back to file
        {
            let mut file = fs::File::create(&safe_path)
                .map_err(|e| ToolError::Io(e))?;
            file.write_all(new_content.as_bytes())
                .map_err(|e| ToolError::Io(e))?;
        }
        
        Ok(ToolOutput::success(json!({
            "file": path,
            "position": position,
            "inserted_at_byte": insert_pos,
            "bytes_inserted": content.len(),
            "new_file_size": new_content.len(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[tokio::test]
    async fn test_insert_at_start() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();
        
        let tool = InsertContentTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>HEADER
</content>
                <position>start</position>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let content = fs::read_to_string(&file_path).unwrap();
        println!("DEBUG: Content after insert at start: {:?}", content);
        assert!(content.starts_with("HEADER\n"));
        assert!(content.contains("line 1"));
    }
    
    #[tokio::test]
    async fn test_insert_at_end() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "line 1\nline 2").unwrap();
        
        let tool = InsertContentTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>line 3</content>
                <position>end</position>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.ends_with("line 3"));
    }
    
    #[tokio::test]
    async fn test_insert_at_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();
        
        let tool = InsertContentTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>INSERTED</content>
                <position>line:2</position>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let content = fs::read_to_string(&file_path).unwrap();
        // Content inserted at line 2 position (after "line 1\n")
        assert!(content.contains("line 1\nINSERTED"));
    }
    
    #[tokio::test]
    async fn test_insert_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "original").unwrap();
        
        let tool = InsertContentTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        context.dry_run = true;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>NEW CONTENT</content>
                <position>end</position>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["dry_run"], true);
        
        // File should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "original");
    }
    
    #[tokio::test]
    async fn test_insert_requires_approval() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "content").unwrap();
        
        let tool = InsertContentTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = true;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>more</content>
                <position>end</position>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(matches!(result, Err(ToolError::ApprovalRequired(_))));
        
        // File should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "content");
    }
}
