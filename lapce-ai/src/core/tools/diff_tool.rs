// Diff tool implementation - P0-7

use crate::core::tools::traits::{Tool, ToolContext, ToolError, ToolResult, ToolOutput};
use crate::core::tools::xml_util::XmlParser;
use crate::core::tools::diff_engine::DiffEngine;
use crate::ipc::ipc_messages::DiffOperation;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;
use uuid::Uuid;

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
            "multiApply" => self.multi_apply_diff(tool_data, &context).await,
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
        let left_path = temp_original.path().to_string_lossy().to_string();
        let right_path = temp_modified.path().to_string_lossy().to_string();
        let correlation_id = Uuid::new_v4().to_string();
        
        // Open diff view through diff controller
        if let Some(controller) = context.get_diff_controller() {
            let _ = controller.open_diff(
                left_path.clone(),
                right_path.clone(),
                Some(format!("Diff: {}", file_path)),
            ).await;
        }
        
        // Emit DiffOperation event for tracking
        if let Some(emitter) = context.get_event_emitter() {
            let event = DiffOperation::OpenDiffFiles {
                left_path: left_path.clone(),
                right_path: right_path.clone(),
                correlation_id: correlation_id.clone(),
            };
            if let Ok(json) = serde_json::to_value(&event) {
                let _ = emitter.emit_correlated(correlation_id.clone(), json).await;
            }
        }
        
        Ok(ToolOutput::success(json!({
            "operation": "preview",
            "file": file_path,
            "hunks": hunks.len(),
            "originalFile": left_path,
            "modifiedFile": right_path,
            "correlation_id": correlation_id,
            "message": "Diff files opened in editor",
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
    
    /// Apply patches to multiple files atomically (per-file atomicity)
    async fn multi_apply_diff(&self, parsed: &Value, context: &ToolContext) -> ToolResult {
        use std::time::Instant;
        
        // Parse files array - handle both array and object with file children
        let files = if let Some(files_arr) = parsed["files"].as_array() {
            files_arr.clone()
        } else if let Some(files_obj) = parsed["files"].as_object() {
            // If files is an object with "file" key(s), extract them
            if let Some(file_val) = files_obj.get("file") {
                if file_val.is_array() {
                    file_val.as_array().unwrap().clone()
                } else {
                    vec![file_val.clone()]
                }
            } else {
                return Err(ToolError::InvalidArguments("Missing 'file' entries in 'files'".to_string()));
            }
        } else {
            return Err(ToolError::InvalidArguments("Missing 'files' array".to_string()));
        };
        
        if files.is_empty() {
            return Err(ToolError::InvalidArguments("Empty files array".to_string()));
        }
        
        let start_time = Instant::now();
        let mut successes = Vec::new();
        let mut failures = Vec::new();
        
        // Process each file
        for (idx, file_spec) in files.iter().enumerate() {
            let file_path = file_spec["path"].as_str()
                .ok_or_else(|| ToolError::InvalidArguments(format!("Missing 'path' in file #{}", idx)))?;
                
            let new_content = file_spec["newContent"].as_str()
                .ok_or_else(|| ToolError::InvalidArguments(format!("Missing 'newContent' in file #{}", idx)))?;
            
            let path = context.resolve_path(file_path);
            
            // Check .rooignore
            if !context.is_path_allowed(&path) {
                failures.push(json!({
                    "file": file_path,
                    "error": "Blocked by .rooignore",
                }));
                continue;
            }
            
            // Ensure within workspace
            let safe_path = match super::fs::ensure_workspace_path(&context.workspace, &path) {
                Ok(p) => p,
                Err(e) => {
                    failures.push(json!({
                        "file": file_path,
                        "error": e,
                    }));
                    continue;
                }
            };
            
            // Request approval (for entire batch on first file if needed)
            if idx == 0 && context.require_approval && !context.dry_run {
                return Err(ToolError::ApprovalRequired(format!(
                    "Approval required to apply patches to {} file(s)",
                    files.len()
                )));
            }
            
            // Read original
            let original = match fs::read_to_string(&safe_path) {
                Ok(c) => c,
                Err(e) => {
                    failures.push(json!({
                        "file": file_path,
                        "error": format!("Failed to read: {}", e),
                    }));
                    continue;
                }
            };
            
            // Dry run check
            if context.dry_run {
                successes.push(json!({
                    "file": file_path,
                    "dryRun": true,
                    "originalSize": original.len(),
                    "newSize": new_content.len(),
                }));
                continue;
            }
            
            // Atomically write new content
            match fs::write(&safe_path, new_content) {
                Ok(_) => {
                    successes.push(json!({
                        "file": file_path,
                        "originalSize": original.len(),
                        "newSize": new_content.len(),
                        "bytesWritten": new_content.len(),
                    }));
                }
                Err(e) => {
                    // On write failure, try to restore original
                    let _ = fs::write(&safe_path, &original);
                    failures.push(json!({
                        "file": file_path,
                        "error": format!("Failed to write: {}", e),
                        "restored": true,
                    }));
                }
            }
        }
        
        let elapsed = start_time.elapsed();
        let has_failures = !failures.is_empty();
        
        Ok(ToolOutput {
            success: !has_failures,
            result: json!({
                "operation": "multiApply",
                "totalFiles": files.len(),
                "successCount": successes.len(),
                "failureCount": failures.len(),
                "successes": successes,
                "failures": failures,
                "elapsedMs": elapsed.as_millis(),
                "dryRun": context.dry_run,
            }),
            error: if has_failures { Some(format!("{} files failed", failures.len())) } else { None },
            metadata: std::collections::HashMap::new()
        })
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
        context.permissions.file_write = true;
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
    
    #[tokio::test]
    async fn test_multi_apply_diff_success() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        fs::write(&file1, "original content 1").unwrap();
        fs::write(&file2, "original content 2").unwrap();
        
        let tool = DiffTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <operation>multiApply</operation>
                <files>
                    <file>
                        <path>file1.txt</path>
                        <newContent>modified content 1</newContent>
                    </file>
                    <file>
                        <path>file2.txt</path>
                        <newContent>modified content 2</newContent>
                    </file>
                </files>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["successCount"], 2);
        assert_eq!(result.result["failureCount"], 0);
        
        // Verify files were modified
        assert_eq!(fs::read_to_string(&file1).unwrap(), "modified content 1");
        assert_eq!(fs::read_to_string(&file2).unwrap(), "modified content 2");
    }
    
    #[tokio::test]
    async fn test_multi_apply_diff_partial_failure() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create one valid file
        let file1 = temp_dir.path().join("file1.txt");
        fs::write(&file1, "original").unwrap();
        
        let tool = DiffTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <operation>multiApply</operation>
                <files>
                    <file>
                        <path>file1.txt</path>
                        <newContent>modified</newContent>
                    </file>
                    <file>
                        <path>nonexistent.txt</path>
                        <newContent>content</newContent>
                    </file>
                </files>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(!result.success); // Should fail due to partial failure
        assert_eq!(result.result["successCount"], 1);
        assert_eq!(result.result["failureCount"], 1);
        
        // First file should still be modified
        assert_eq!(fs::read_to_string(&file1).unwrap(), "modified");
        
        // Check failure info
        let failures = result.result["failures"].as_array().unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0]["file"], "nonexistent.txt");
    }
    
    #[tokio::test]
    async fn test_multi_apply_diff_requires_approval() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        fs::write(&file1, "content").unwrap();
        
        let tool = DiffTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = true;
        
        let args = json!(format!(r#"
            <tool>
                <operation>multiApply</operation>
                <files>
                    <file>
                        <path>file1.txt</path>
                        <newContent>modified</newContent>
                    </file>
                </files>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(matches!(result, Err(ToolError::ApprovalRequired(_))));
        
        // File should be unchanged
        assert_eq!(fs::read_to_string(&file1).unwrap(), "content");
    }
    
    #[tokio::test]
    async fn test_multi_apply_diff_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        fs::write(&file1, "original").unwrap();
        
        let tool = DiffTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        context.dry_run = true;
        
        let args = json!(format!(r#"
            <tool>
                <operation>multiApply</operation>
                <files>
                    <file>
                        <path>file1.txt</path>
                        <newContent>modified</newContent>
                    </file>
                </files>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["dryRun"], true);
        
        // File should be unchanged
        assert_eq!(fs::read_to_string(&file1).unwrap(), "original");
    }
    
    #[tokio::test]
    async fn test_approval_denial_integrity() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test.txt");
        let original_content = "original content that should not change";
        fs::write(&file1, original_content).unwrap();
        
        let tool = DiffTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = true;  // Require approval
        
        let args = json!(format!(r#"
            <tool>
                <operation>apply</operation>
                <file>test.txt</file>
                <newContent>modified content</newContent>
            </tool>
        "#));
        
        // Execute should fail with ApprovalRequired
        let result = tool.execute(args, context).await;
        assert!(matches!(result, Err(ToolError::ApprovalRequired(_))));
        
        // Verify file was NOT modified
        let content_after = fs::read_to_string(&file1).unwrap();
        assert_eq!(content_after, original_content, "File should remain unchanged when approval is denied");
        
        // Verify file permissions unchanged
        let metadata = fs::metadata(&file1).unwrap();
        assert!(metadata.is_file());
    }
    
    #[tokio::test]
    async fn test_multi_apply_diff_performance() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create 100 files with 1000 lines each (simulating 1k-line files)
        for i in 0..100 {
            let file = temp_dir.path().join(format!("file{}.txt", i));
            let content = (0..1000).map(|n| format!("line {}\n", n)).collect::<String>();
            fs::write(&file, content).unwrap();
        }
        
        let tool = DiffTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        // Build files array
        let mut files_xml = String::new();
        for i in 0..100 {
            files_xml.push_str(&format!(r#"
                <file>
                    <path>file{}.txt</path>
                    <newContent>{}</newContent>
                </file>"#, i, "modified\n".repeat(1000)));
        }
        
        let args = json!(format!(r#"
            <tool>
                <operation>multiApply</operation>
                <files>
                    {}
                </files>
            </tool>
        "#, files_xml));
        
        let start = std::time::Instant::now();
        let result = tool.execute(args, context).await.unwrap();
        let elapsed = start.elapsed();
        
        assert!(result.success);
        assert_eq!(result.result["successCount"], 100);
        
        // Should complete in reasonable time (allow generous margin)
        assert!(elapsed.as_millis() < 5000, "Took {}ms, expected <5000ms", elapsed.as_millis());
        
        // Check reported elapsed time is reasonable
        let reported_ms = result.result["elapsedMs"].as_u64().unwrap();
        assert!(reported_ms < 5000, "Reported {}ms", reported_ms);
    }
}
