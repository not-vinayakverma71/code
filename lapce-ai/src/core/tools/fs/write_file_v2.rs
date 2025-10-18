// WriteFile tool implementation v2 - Hardened version with production features
// Part of Core FS tools hardening - pre-IPC TODO

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::xml_util::XmlParser;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use super::utils::{
    self, SymlinkPolicy, FileEncoding, LineEnding,
    MAX_WRITE_SIZE, write_file_safe, get_file_info,
    normalize_line_endings, detect_line_ending
};

pub struct WriteFileToolV2;

#[async_trait]
impl Tool for WriteFileToolV2 {
    fn name(&self) -> &'static str {
        "writeFile"
    }
    
    fn description(&self) -> &'static str {
        "Write content to a file with encoding preservation and safety checks (requires approval)"
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
        
        // Extract arguments
        let tool_data = super::extract_tool_data(&parsed);
        
        let path = tool_data["path"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;
            
        let content = tool_data["content"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' argument".to_string()))?;
            
        let create_dirs = tool_data.get("createDirs")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<bool>().ok()))
            })
            .unwrap_or(false);
            
        // New options for v2
        let preserve_encoding = tool_data.get("preserveEncoding")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
        let preserve_line_endings = tool_data.get("preserveLineEndings")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
        let force_line_ending = tool_data.get("forceLineEnding")
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "lf" | "unix" => Some(LineEnding::Lf),
                "crlf" | "windows" => Some(LineEnding::CrLf),
                "cr" => Some(LineEnding::Cr),
                _ => None
            });
            
        let max_size = tool_data.get("maxSize")
            .and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
            })
            .unwrap_or(MAX_WRITE_SIZE);
            
        let backup_if_exists = tool_data.get("backupIfExists")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<bool>().ok()))
            })
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
        if !context.permissions.file_write {
            return Err(ToolError::PermissionDenied(format!(
                "Permission denied to write file: {}",
                file_path.display()
            )));
        }
        
        // Check for symlinks BEFORE canonicalization
        let original_path = if file_path.is_absolute() {
            file_path.clone()
        } else {
            context.workspace.join(&file_path)
        };
        if utils::is_symlink(&original_path).unwrap_or(false) {
            return Err(ToolError::Other(format!(
                "Cannot write to symlink: {}. Please write to the target file directly.",
                path
            )));
        }
        
        // Ensure path is within workspace
        let safe_path = super::ensure_workspace_path(&context.workspace, &file_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Check if file exists and get its info
        let (exists, original_info) = if safe_path.exists() {
            match get_file_info(&safe_path) {
                Ok(info) => (true, Some(info)),
                Err(_) => (true, None)
            }
        } else {
            (false, None)
        };
        
        let operation = if exists { "overwrite" } else { "create" };
        
        // Check content size
        if content.len() as u64 > max_size {
            return Err(ToolError::Other(format!(
                "Content exceeds size limit: {} bytes > {} bytes",
                content.len(),
                max_size
            )));
        }
        
        // Pre-process content
        let processed_content = preprocess_content_v2(
            content,
            &original_info,
            preserve_line_endings,
            preserve_encoding,
            force_line_ending
        );
        
        // Request approval if required
        if context.require_approval && !context.dry_run {
            let approval_id = context.log_approval("writeFile", "write", path);
            return Err(ToolError::ApprovalRequired(format!(
                "Approval required to {} file: {} (approval_id: {})",
                operation, path, approval_id
            )));
        }
        
        // Dry run mode
        if context.dry_run {
            return Ok(ToolOutput::success(json!({
                "path": path,
                "operation": operation,
                "dryRun": true,
                "wouldWrite": processed_content.len(),
                "preserveEncoding": preserve_encoding,
                "preserveLineEndings": preserve_line_endings,
                "originalExists": exists,
            })));
        }
        
        // Create backup if requested and file exists
        if backup_if_exists && exists {
            let backup_path = format!("{}.backup", safe_path.display());
            fs::copy(&safe_path, &backup_path)
                .map_err(|e| ToolError::Io(e))?;
        }
        
        // Create parent directories if requested
        if create_dirs {
            if let Some(parent) = safe_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| ToolError::Io(e))?;
            }
        }
        
        // Write the file with safety checks
        // Don't preserve encoding if we've forced line endings (already applied in preprocessing)
        let should_preserve = preserve_encoding && force_line_ending.is_none();
        match write_file_safe(&safe_path, &processed_content, should_preserve, max_size) {
            Ok(_) => {
                // Log successful file operation
                context.log_file_op("write", path, true, None);
                
                // Get new file info
                let new_info = get_file_info(&safe_path).ok();
                
                Ok(ToolOutput::success(json!({
                    "path": path,
                    "operation": operation,
                    "bytesWritten": processed_content.len(),
                    "backup": if backup_if_exists && exists { 
                        Some(format!("{}.backup", path)) 
                    } else { 
                        None 
                    },
                    "metadata": new_info.map(|info| json!({
                        "encoding": format!("{:?}", info.encoding),
                        "lineEnding": info.line_ending.map(|le| format!("{:?}", le)),
                        "size": info.size,
                    }))
                })))
            }
            Err(e) => {
                // Log failed file operation
                context.log_file_op("write", path, false, Some(&e.to_string()));
                Err(ToolError::Io(e))
            }
        }
    }
}

/// Enhanced content preprocessing with encoding and line ending handling
fn preprocess_content_v2(
    content: &str,
    original_info: &Option<super::utils::FileInfo>,
    preserve_line_endings: bool,
    preserve_encoding: bool,
    force_line_ending: Option<LineEnding>,
) -> String {
    let mut processed = content.to_string();
    
    // First, remove any code fence markers and line numbers
    processed = remove_code_artifacts(&processed);
    
    // Handle line endings
    if let Some(forced) = force_line_ending {
        // Force specific line ending
        processed = normalize_line_endings(&processed, forced);
    } else if preserve_line_endings {
        // Preserve original line endings if file exists
        if let Some(ref info) = original_info {
            if let Some(ending) = info.line_ending {
                processed = normalize_line_endings(&processed, ending);
            }
        }
    }
    // If neither forced nor preserved, keep as-is (typically LF)
    
    processed
}

/// Remove code fences and line numbers from content
fn remove_code_artifacts(content: &str) -> String {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut skip_next_empty = false;
    
    for line in content.lines() {
        // Check for code fence
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            skip_next_empty = true; // Skip empty line after closing fence
            continue; // Skip fence lines
        }
        
        // Skip single empty line after code fence
        if skip_next_empty && line.trim().is_empty() {
            skip_next_empty = false;
            continue;
        }
        skip_next_empty = false;
        
        // Remove line numbers (multiple formats)
        let processed = remove_line_number(line);
        lines.push(processed);
    }
    
    lines.join("\n")
}

/// Remove line numbers from a single line
fn remove_line_number(line: &str) -> &str {
    // Format: "   123 | actual content"
    if let Some(pos) = line.find(" | ") {
        let prefix = &line[..pos];
        if prefix.trim().parse::<u32>().is_ok() {
            return &line[pos + 3..]; // Skip the line number and " | "
        }
    }
    
    // Format: "123: actual content" 
    if let Some(pos) = line.find(": ") {
        let prefix = &line[..pos];
        if prefix.trim().parse::<u32>().is_ok() && pos < 10 {
            return &line[pos + 2..]; // Skip the line number and ": "
        }
    }
    
    // Format: "123. actual content"
    if let Some(pos) = line.find(". ") {
        let prefix = &line[..pos];
        if prefix.trim().parse::<u32>().is_ok() && pos < 10 {
            return &line[pos + 2..]; // Skip the line number and ". "
        }
    }
    
    // Format: "[123] actual content"
    if line.starts_with('[') {
        if let Some(end) = line.find("] ") {
            let num_part = &line[1..end];
            if num_part.parse::<u32>().is_ok() && end < 10 {
                return &line[end + 2..];
            }
        }
    }
    
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_remove_code_artifacts() {
        let input = r#"```rust
fn main() {
    println!("Hello");
}
```

Normal text here
   1 | fn test() {
   2 |     let x = 5;
   3 | }

123: first line
456. second line
[789] third line"#;
        
        let output = remove_code_artifacts(input);
        
        // Code fence should be removed
        assert!(!output.contains("```"));
        
        // Content should remain
        assert!(output.contains("fn main() {"));
        assert!(output.contains("Normal text here"));
        
        // Line numbers should be removed
        assert!(output.contains("fn test() {"));
        assert!(output.contains("let x = 5;"));
        assert!(output.contains("first line"));
        assert!(output.contains("second line")); 
        assert!(output.contains("third line"));
        
        // Line numbers themselves should not be present
        assert!(!output.contains("   1 |"));
        assert!(!output.contains("123:"));
        assert!(!output.contains("456."));
        assert!(!output.contains("[789]"));
    }
    
    #[tokio::test]
    async fn test_write_file_preserve_line_endings() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("preserve.txt");
        
        // Create file with CRLF endings
        fs::write(&file_path, "line1\r\nline2\r\nline3").unwrap();
        
        let tool = WriteFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        // Write new content, preserving line endings
        let args = json!(format!(r#"
            <tool>
                <path>preserve.txt</path>
                <content>new line1
new line2</content>
                <preserveLineEndings>true</preserveLineEndings>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        // Check that CRLF was preserved
        let content = fs::read(&file_path).unwrap();
        let content_str = String::from_utf8_lossy(&content);
        assert!(content_str.contains("\r\n"));
    }
    
    #[tokio::test]
    async fn test_write_file_force_line_ending() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("force.txt");
        
        let tool = WriteFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        // Force Unix line endings
        let args = json!(format!(r#"
            <tool>
                <path>force.txt</path>
                <content>line1
line2
line3</content>
                <forceLineEnding>lf</forceLineEnding>
                <createDirs>true</createDirs>
            </tool>
        "#));
        
        let result = tool.execute(args, context.clone()).await.unwrap();
        assert!(result.success);
        
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(!content.contains("\r\n"));
        assert!(content.contains("\n"));
        
        // Force Windows line endings
        let args = json!(format!(r#"
            <tool>
                <path>force.txt</path>
                <content>new line1
new line2</content>
                <forceLineEnding>crlf</forceLineEnding>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        let content = fs::read(&file_path).unwrap();
        let content_str = String::from_utf8_lossy(&content);
        assert!(content_str.contains("\r\n"));
    }
    
    #[tokio::test]
    async fn test_write_file_backup() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("backup.txt");
        
        // Create original file
        fs::write(&file_path, "original content").unwrap();
        
        let tool = WriteFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let args = json!(format!(r#"
            <tool>
                <path>backup.txt</path>
                <content>new content</content>
                <backupIfExists>true</backupIfExists>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert!(result.result["backup"].is_string());
        
        // Check backup was created
        let backup_path = temp_dir.path().join("backup.txt.backup");
        assert!(backup_path.exists());
        
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, "original content");
        
        // Check new content
        let new_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(new_content, "new content");
    }
    
    #[tokio::test]
    async fn test_write_file_symlink_rejection() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("target.txt");
        let link_path = temp_dir.path().join("link.txt");
        
        fs::write(&target_path, "target content").unwrap();
        
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_path, &link_path).unwrap();
            
            let tool = WriteFileToolV2;
            let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
            context.permissions.file_write = true;
            context.require_approval = false;
            
            let args = json!(format!(r#"
                <tool>
                    <path>link.txt</path>
                    <content>new content</content>
                </tool>
            "#));
            
            let result = tool.execute(args, context).await;
            assert!(result.is_err());
            
            if let Err(ToolError::Other(msg)) = result {
                assert!(msg.contains("Cannot write to symlink"));
            } else {
                panic!("Expected symlink error");
            }
            
            // Original target should be unchanged
            let content = fs::read_to_string(&target_path).unwrap();
            assert_eq!(content, "target content");
        }
    }
    
    #[tokio::test]
    async fn test_write_file_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("size.txt");
        
        let tool = WriteFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        // Try to write content larger than limit
        let large_content = "x".repeat(1024);
        let args = json!(format!(r#"
            <tool>
                <path>size.txt</path>
                <content>{}</content>
                <maxSize>512</maxSize>
            </tool>
        "#, large_content));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        
        if let Err(ToolError::Other(msg)) = result {
            assert!(msg.contains("exceeds size limit"));
        } else {
            panic!("Expected size limit error");
        }
    }
}
