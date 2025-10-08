// Diff tool implementation - P0-7

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::xml_util::XmlParser;
use crate::core::tools::diff_engine::{DiffEngine, UnifiedPatch};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use std::io::Write;

pub struct DiffTool;

#[async_trait]
impl Tool for DiffTool {
    fn name(&self) -> &'static str {
        "diff"
    }
    
    fn description(&self) -> &'static str {
        "Generate and preview diffs, apply patches with approval"
    }
    
    fn requires_approval(&self) -> bool {
        true // Applying patches requires approval
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        let parser = XmlParser::new();
        let parsed = parser.parse(args.as_str().ok_or_else(|| {
            ToolError::InvalidArguments("Expected XML string".to_string())
        })?).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        
        // Extract tool data - handle both flat and nested structures
        let tool_data = if parsed.get("tool").is_some() {
            &parsed["tool"]
        } else {
            &parsed
        };
        
        let operation = tool_data.get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("preview");
        
        match operation {
            "preview" => self.preview_diff(tool_data, &context).await,
            "apply" => self.apply_diff(tool_data, &context).await,
            "search_replace" => self.search_replace(tool_data, &context).await,
            "edit_range" => self.edit_line_range(tool_data, &context).await,
            _ => Err(ToolError::InvalidArguments(format!("Unknown operation: {}", operation))),
        }
    }
}

impl DiffTool {
    async fn preview_diff(&self, parsed: &Value, context: &ToolContext) -> ToolResult {
        let file_path = parsed["file"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'file' argument".to_string()))?;
            
        let new_content = parsed["newContent"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'newContent' argument".to_string()))?;
        
        let path = context.resolve_path(file_path);
        
        // Check .rooignore
        if !context.is_path_allowed(&path) {
            return Err(ToolError::RooIgnoreBlocked(format!("Path '{}' is blocked", file_path)));
        }
        
        // Ensure within workspace
        let safe_path = super::fs::ensure_workspace_path(&context.workspace, &path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Read original content
        let original_content = fs::read_to_string(&safe_path)
            .map_err(|e| ToolError::Io(e))?;
        
        // Generate diff
        let engine = DiffEngine::new();
        let hunks = engine.generate_diff(&original_content, new_content);
        
        // Create temp files for preview
        let mut temp_original = NamedTempFile::new()
            .map_err(|e| ToolError::Other(format!("Failed to create temp file: {}", e)))?;
        temp_original.write_all(original_content.as_bytes())
            .map_err(|e| ToolError::Io(e))?;
        
        let mut temp_modified = NamedTempFile::new()
            .map_err(|e| ToolError::Other(format!("Failed to create temp file: {}", e)))?;
        temp_modified.write_all(new_content.as_bytes())
            .map_err(|e| ToolError::Io(e))?;
        
        Ok(ToolOutput::success(json!({
            "operation": "preview",
            "file": file_path,
            "hunks": hunks.len(),
            "originalFile": temp_original.path().to_string_lossy(),
            "modifiedFile": temp_modified.path().to_string_lossy(),
            "message": "Use OpenDiffFiles IPC to view in editor",
        })))
    }
    
    async fn apply_diff(&self, parsed: &Value, context: &ToolContext) -> ToolResult {
        let file_path = parsed["file"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'file' argument".to_string()))?;
            
        let new_content = parsed["newContent"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'newContent' argument".to_string()))?;
        
        let path = context.resolve_path(file_path);
        
        // Check .rooignore
        if !context.is_path_allowed(&path) {
            return Err(ToolError::RooIgnoreBlocked(format!("Path '{}' is blocked", file_path)));
        }
        
        // Ensure within workspace
        let safe_path = super::fs::ensure_workspace_path(&context.workspace, &path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Request approval
        if context.require_approval && !context.dry_run {
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required to apply diff to: {}",
                file_path
            )));
        }
        
        // Read original content
        let original_content = fs::read_to_string(&safe_path)
            .map_err(|e| ToolError::Io(e))?;
        
        // Dry run
        if context.dry_run {
            let engine = DiffEngine::new();
            let hunks = engine.generate_diff(&original_content, new_content);
            
            return Ok(ToolOutput::success(json!({
                "operation": "apply",
                "file": file_path,
                "dryRun": true,
                "hunks": hunks.len(),
                "wouldApply": true,
            })));
        }
        
        // Apply the changes
        fs::write(&safe_path, new_content)
            .map_err(|e| ToolError::Io(e))?;
        
        Ok(ToolOutput::success(json!({
            "operation": "apply",
            "file": file_path,
            "bytesWritten": new_content.len(),
            "success": true,
        })))
    }
    
    async fn search_replace(&self, parsed: &Value, context: &ToolContext) -> ToolResult {
        let file_path = parsed["file"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'file' argument".to_string()))?;
            
        let search = parsed["search"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'search' argument".to_string()))?;
            
        let replace = parsed["replace"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'replace' argument".to_string()))?;
            
        let replace_all = parsed.get("replaceAll")
            .and_then(|v| v.as_str())
            .map(|s| s == "true")
            .unwrap_or(false);
        
        let path = context.resolve_path(file_path);
        
        // Check .rooignore
        if !context.is_path_allowed(&path) {
            return Err(ToolError::RooIgnoreBlocked(format!("Path '{}' is blocked", file_path)));
        }
        
        // Ensure within workspace
        let safe_path = super::fs::ensure_workspace_path(&context.workspace, &path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Request approval
        if context.require_approval && !context.dry_run {
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required for search/replace in: {}",
                file_path
            )));
        }
        
        // Read content
        let content = fs::read_to_string(&safe_path)
            .map_err(|e| ToolError::Io(e))?;
        
        // Perform search/replace
        let engine = DiffEngine::new();
        let new_content = engine.search_replace(&content, search, replace, replace_all);
        
        let replacements = if replace_all {
            content.matches(search).count()
        } else {
            if content.contains(search) { 1 } else { 0 }
        };
        
        // Dry run
        if context.dry_run {
            return Ok(ToolOutput::success(json!({
                "operation": "search_replace",
                "file": file_path,
                "dryRun": true,
                "replacements": replacements,
            })));
        }
        
        // Write changes
        fs::write(&safe_path, &new_content)
            .map_err(|e| ToolError::Io(e))?;
        
        Ok(ToolOutput::success(json!({
            "operation": "search_replace",
            "file": file_path,
            "replacements": replacements,
            "bytesWritten": new_content.len(),
        })))
    }
    
    async fn edit_line_range(&self, parsed: &Value, context: &ToolContext) -> ToolResult {
        let file_path = parsed["file"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'file' argument".to_string()))?;
            
        let start_line = parsed["startLine"].as_str()
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| ToolError::InvalidArguments("Missing or invalid 'startLine'".to_string()))?;
            
        let end_line = parsed["endLine"].as_str()
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| ToolError::InvalidArguments("Missing or invalid 'endLine'".to_string()))?;
            
        let new_content = parsed["newContent"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'newContent' argument".to_string()))?;
        
        let path = context.resolve_path(file_path);
        
        // Check .rooignore
        if !context.is_path_allowed(&path) {
            return Err(ToolError::RooIgnoreBlocked(format!("Path '{}' is blocked", file_path)));
        }
        
        // Ensure within workspace
        let safe_path = super::fs::ensure_workspace_path(&context.workspace, &path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Request approval
        if context.require_approval && !context.dry_run {
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required to edit lines {}-{} in: {}",
                start_line, end_line, file_path
            )));
        }
        
        // Read content
        let content = fs::read_to_string(&safe_path)
            .map_err(|e| ToolError::Io(e))?;
        
        // Apply edit
        let engine = DiffEngine::new();
        let result = engine.edit_line_range(&content, start_line, end_line, new_content)
            .map_err(|e| ToolError::Other(e.to_string()))?;
        
        // Dry run
        if context.dry_run {
            return Ok(ToolOutput::success(json!({
                "operation": "edit_range",
                "file": file_path,
                "dryRun": true,
                "startLine": start_line,
                "endLine": end_line,
            })));
        }
        
        // Write changes
        fs::write(&safe_path, &result)
            .map_err(|e| ToolError::Io(e))?;
        
        Ok(ToolOutput::success(json!({
            "operation": "edit_range",
            "file": file_path,
            "startLine": start_line,
            "endLine": end_line,
            "bytesWritten": result.len(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    
    #[tokio::test]
    async fn test_diff_preview() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "original line 1").unwrap();
        writeln!(file, "original line 2").unwrap();
        
        let tool = DiffTool;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <operation>preview</operation>
                <file>test.txt</file>
                <newContent>modified line 1
modified line 2</newContent>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["operation"], "preview");
    }
    
    #[tokio::test]
    async fn test_search_replace_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "foo bar foo").unwrap();
        
        let tool = DiffTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.dry_run = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <operation>search_replace</operation>
                <file>test.txt</file>
                <search>foo</search>
                <replace>bar</replace>
                <replaceAll>true</replaceAll>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["dryRun"], true);
        assert_eq!(result.result["replacements"], 2);
        
        // File should not be modified
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("foo"));
    }
}
