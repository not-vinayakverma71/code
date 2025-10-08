// EditFile tool implementation - P0-5

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::xml_util::XmlParser;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;

pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &'static str {
        "editFile"
    }
    
    fn description(&self) -> &'static str {
        "Edit a file by replacing content (requires approval)"
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
            
        let old_content = tool_data["oldContent"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'oldContent' argument".to_string()))?;
            
        let new_content = tool_data["newContent"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'newContent' argument".to_string()))?;
            
        let replace_all = tool_data.get("replaceAll")
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
                "Permission denied to edit file: {}",
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
        
        // Read current content
        let current_content = fs::read_to_string(&safe_path)
            .map_err(|e| ToolError::Io(e))?;
        
        // Check if old content exists
        if !current_content.contains(old_content) {
            return Err(ToolError::Other(format!(
                "Content to replace not found in file '{}'",
                path
            )));
        }
        
        // Request approval if required
        if context.require_approval && !context.dry_run {
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required to edit file: {}",
                path
            )));
        }
        
        // Perform replacement
        let updated_content = if replace_all {
            current_content.replace(old_content, new_content)
        } else {
            current_content.replacen(old_content, new_content, 1)
        };
        
        // Count replacements
        let replacements = if replace_all {
            current_content.matches(old_content).count()
        } else {
            1
        };
        
        // Dry run mode - don't actually write
        if context.dry_run {
            return Ok(ToolOutput::success(json!({
                "path": path,
                "dryRun": true,
                "replacements": replacements,
                "oldLength": current_content.len(),
                "newLength": updated_content.len(),
            })));
        }
        
        // Write the updated content
        fs::write(&safe_path, &updated_content)
            .map_err(|e| ToolError::Io(e))?;
        
        Ok(ToolOutput::success(json!({
            "path": path,
            "replacements": replacements,
            "bytesWritten": updated_content.len(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};
    use std::io::Write;
    
    #[tokio::test]
    async fn test_edit_file_single_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create initial file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello world").unwrap();
        writeln!(file, "Hello again").unwrap();
        writeln!(file, "Goodbye").unwrap();
        
        let tool = EditFileTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false; // Disable approval for test
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <oldContent>Hello</oldContent>
                <newContent>Hi</newContent>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements"], 1);
        
        // Check file content
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.starts_with("Hi world"));
        assert!(content.contains("Hello again")); // Only first occurrence replaced
    }
    
    #[tokio::test]
    async fn test_edit_file_replace_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create initial file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "foo bar foo").unwrap();
        writeln!(file, "foo baz").unwrap();
        
        let tool = EditFileTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false; // Disable approval for test
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <oldContent>foo</oldContent>
                <newContent>replaced</newContent>
                <replaceAll>true</replaceAll>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements"], 3);
        
        // Check file content
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(!content.contains("foo"));
        assert_eq!(content.matches("replaced").count(), 3);
    }
    
    #[tokio::test]
    async fn test_edit_file_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create initial file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Original content").unwrap();
        
        let tool = EditFileTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.dry_run = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <oldContent>Original</oldContent>
                <newContent>Modified</newContent>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["dryRun"], true);
        
        // File should not be modified in dry run
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Original"));
        assert!(!content.contains("Modified"));
    }
    
    #[tokio::test]
    async fn test_edit_file_content_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create initial file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Some content").unwrap();
        
        let tool = EditFileTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <oldContent>Not present</oldContent>
                <newContent>New</newContent>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        
        if let Err(ToolError::Other(msg)) = result {
            assert!(msg.contains("not found"));
        } else {
            panic!("Expected content not found error");
        }
    }
}
