// Search and Replace Tool - P0-5

use crate::core::tools::traits::{Tool, ToolContext, ToolOutput, ToolResult, ToolError};
use crate::core::tools::xml_util::XmlParser;
use serde_json::{json, Value};
use std::fs;
use regex::Regex;
use super::ensure_workspace_path;
use async_trait::async_trait;

pub struct SearchAndReplaceTool;

#[async_trait]
impl Tool for SearchAndReplaceTool {
    fn name(&self) -> &'static str {
        "searchAndReplace"
    }
    
    fn description(&self) -> &'static str {
        "Search and replace text in a file with regex/literal support (requires approval)"
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
        
        // Extract tool data
        let tool_data = if parsed.get("tool").is_some() {
            &parsed["tool"]
        } else {
            &parsed
        };
        
        let path = tool_data["path"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;
            
        let search = tool_data["search"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'search' argument".to_string()))?;
            
        let replace = tool_data["replace"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'replace' argument".to_string()))?;
            
        let mode = tool_data.get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("literal");
            
        let multiline = tool_data.get("multiline")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let preview_only = tool_data.get("preview")
            .and_then(|v| v.as_bool())
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
        
        // Check permissions (skip for preview-only)
        if !preview_only && !context.can_write_file(&file_path).await {
            return Err(ToolError::PermissionDenied(format!(
                "Permission denied to write file: {}",
                file_path.display()
            )));
        }
        
        // Ensure path is within workspace
        let safe_path = ensure_workspace_path(&context.workspace, &file_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // File must exist
        if !safe_path.exists() {
            return Err(ToolError::InvalidArguments(format!(
                "File does not exist: {}",
                path
            )));
        }
        
        // Read file content
        let content = fs::read_to_string(&safe_path)
            .map_err(|e| ToolError::Io(e))?;
        
        // Perform search and replace
        let (new_content, match_count) = match mode {
            "regex" => {
                let regex = if multiline {
                    Regex::new(&format!("(?m){}", search))
                } else {
                    Regex::new(search)
                }.map_err(|e| ToolError::InvalidArguments(format!("Invalid regex: {}", e)))?;
                
                let matches = regex.find_iter(&content).count();
                let result = regex.replace_all(&content, replace).to_string();
                (result, matches)
            }
            "literal" => {
                let matches = content.matches(search).count();
                let result = content.replace(search, replace);
                (result, matches)
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Invalid mode '{}'. Must be 'literal' or 'regex'",
                    mode
                )));
            }
        };
        
        // Preview mode: just return the diff
        if preview_only {
            return Ok(ToolOutput::success(json!({
                "file": path,
                "mode": mode,
                "matches_found": match_count,
                "preview": true,
                "original_size": content.len(),
                "new_size": new_content.len(),
                "changes": format!("-{} +{}", content.len(), new_content.len()),
            })));
        }
        
        // Request approval if required
        if context.require_approval && !context.dry_run {
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required to replace {} occurrence(s) in file: {}",
                match_count, path
            )));
        }
        
        // Dry-run mode
        if context.dry_run {
            return Ok(ToolOutput::success(json!({
                "file": path,
                "mode": mode,
                "matches_found": match_count,
                "dry_run": true,
                "would_modify": match_count > 0,
            })));
        }
        
        // Only write if there were changes
        if match_count > 0 {
            fs::write(&safe_path, &new_content)
                .map_err(|e| ToolError::Io(e))?;
        }
        
        Ok(ToolOutput::success(json!({
            "file": path,
            "mode": mode,
            "replacements_made": match_count,
            "original_size": content.len(),
            "new_size": new_content.len(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[tokio::test]
    async fn test_literal_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "foo bar foo baz").unwrap();
        
        let tool = SearchAndReplaceTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <search>foo</search>
                <replace>qux</replace>
                <mode>literal</mode>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements_made"], 2);
        
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "qux bar qux baz");
    }
    
    #[tokio::test]
    async fn test_regex_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "test123 test456 test789").unwrap();
        
        let tool = SearchAndReplaceTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <search>test\d+</search>
                <replace>item</replace>
                <mode>regex</mode>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements_made"], 3);
        
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "item item item");
    }
    
    #[tokio::test]
    async fn test_multiline_regex() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "line 1\n  indented\nline 2").unwrap();
        
        let tool = SearchAndReplaceTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <search>^\s+</search>
                <replace></replace>
                <mode>regex</mode>
                <multiline>true</multiline>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements_made"], 1);
        
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "line 1\nindented\nline 2");
    }
    
    #[tokio::test]
    async fn test_preview_mode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "original content").unwrap();
        
        let tool = SearchAndReplaceTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <search>original</search>
                <replace>modified</replace>
                <mode>literal</mode>
                <preview>true</preview>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["preview"], true);
        assert_eq!(result.result["matches_found"], 1);
        
        // File should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "original content");
    }
    
    #[tokio::test]
    async fn test_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "foo bar").unwrap();
        
        let tool = SearchAndReplaceTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        context.dry_run = true;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <search>foo</search>
                <replace>qux</replace>
                <mode>literal</mode>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["dry_run"], true);
        assert_eq!(result.result["matches_found"], 1);
        
        // File should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "foo bar");
    }
    
    #[tokio::test]
    async fn test_requires_approval() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "content").unwrap();
        
        let tool = SearchAndReplaceTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = true;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <search>content</search>
                <replace>modified</replace>
                <mode>literal</mode>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(matches!(result, Err(ToolError::ApprovalRequired(_))));
        
        // File should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "content");
    }
    
    #[tokio::test]
    async fn test_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        fs::write(&file_path, "foo bar").unwrap();
        
        let tool = SearchAndReplaceTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <search>notfound</search>
                <replace>whatever</replace>
                <mode>literal</mode>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["replacements_made"], 0);
        
        // File should be unchanged
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "foo bar");
    }
}
