// ReadFile tool implementation v2 - Hardened version with production features
// Part of Core FS tools hardening - pre-IPC TODO

use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::xml_util::XmlParser;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::fs;
use anyhow::Result;
use super::utils::{
    self, FileInfo, SymlinkPolicy, MAX_FILE_SIZE,
    read_file_safe, extract_line_range
};

pub struct ReadFileToolV2;

#[async_trait]
impl Tool for ReadFileToolV2 {
    fn name(&self) -> &'static str {
        "readFile"
    }
    
    fn description(&self) -> &'static str {
        "Read contents of a file with enhanced safety, encoding detection, and line range support"
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
        
        let line_start = tool_data.get("lineStart")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<usize>().ok());
            
        let line_end = tool_data.get("lineEnd")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<usize>().ok());
            
        // Optional: follow symlinks flag (default: true for reads)
        let follow_symlinks = tool_data.get("followSymlinks")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<bool>().ok()))
            })
            .unwrap_or(true);
            
        // Optional: custom size limit (parse from string or u64)
        let max_size = tool_data.get("maxSize")
            .and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
            })
            .unwrap_or(MAX_FILE_SIZE);
        
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
        if !context.permissions.file_read {
            return Err(ToolError::PermissionDenied(format!(
                "Permission denied to read file: {}",
                file_path.display()
            )));
        }
        
        // Check if original path is a symlink BEFORE canonicalization
        let original_path = if file_path.is_absolute() {
            file_path.clone()
        } else {
            context.workspace.join(&file_path)
        };
        let is_symlink = utils::is_symlink(&original_path).unwrap_or(false);
        
        // If symlink and not following symlinks, reject immediately
        if is_symlink && !follow_symlinks {
            return Err(ToolError::Other(format!(
                "Path is a symlink (not allowed): {}",
                path
            )));
        }
        
        // Ensure path is within workspace (this may canonicalize)
        let safe_path = super::ensure_workspace_path(&context.workspace, &file_path)
            .map_err(|e| ToolError::PermissionDenied(e))?;
        
        // Check if file exists
        if !safe_path.exists() {
            return Err(ToolError::Other(format!(
                "File '{}' does not exist",
                path
            )));
        }
        
        // Resolve path (will follow symlinks if policy allows)
        let symlink_policy = if follow_symlinks {
            SymlinkPolicy::Follow
        } else {
            SymlinkPolicy::Error
        };
        
        let resolved_path = utils::resolve_symlink(&safe_path, symlink_policy)
            .map_err(|e| ToolError::Other(format!("Symlink error: {}", e)))?;
        
        // Get comprehensive file info
        let mut file_info = utils::get_file_info(&resolved_path)
            .map_err(|e| ToolError::Io(e))?;
        
        // Override is_symlink with the original check
        file_info.is_symlink = is_symlink;
        
        // Check file size
        if file_info.size > max_size {
            return Err(ToolError::Other(format!(
                "File '{}' exceeds size limit: {} bytes > {} bytes",
                path, file_info.size, max_size
            )));
        }
        
        // Handle binary files
        if file_info.is_binary {
            if super::is_image_file(&resolved_path) {
                return Ok(ToolOutput::success(json!({
                    "type": "image",
                    "path": path,
                    "encoding": format!("{:?}", file_info.encoding),
                    "size": file_info.size,
                    "is_symlink": file_info.is_symlink,
                    "message": "Image file detected. Use appropriate image viewer."
                })));
            }
            return Ok(ToolOutput::success(json!({
                "type": "binary",
                "path": path,
                "encoding": format!("{:?}", file_info.encoding),
                "size": file_info.size,
                "is_symlink": file_info.is_symlink,
                "message": "Binary file detected. Cannot display content."
            })));
        }
        
        // Read file content with safety checks
        let (mut content, info) = read_file_safe(&resolved_path, max_size)
            .map_err(|e| ToolError::Io(e))?;
        
        // Apply line range if specified
        if let (Some(start), Some(end)) = (line_start, line_end) {
            if start > end {
                return Err(ToolError::InvalidArguments(format!(
                    "Invalid line range: start ({}) > end ({})",
                    start, end
                )));
            }
            content = extract_line_range(&content, start, end);
        } else if line_start.is_some() || line_end.is_some() {
            return Err(ToolError::InvalidArguments(
                "Both lineStart and lineEnd must be specified for line range".to_string()
            ));
        }
        
        // Build response with comprehensive metadata
        Ok(ToolOutput::success(json!({
            "path": path,
            "content": content,
            "lineStart": line_start,
            "lineEnd": line_end,
            "metadata": {
                "size": info.size,
                "encoding": format!("{:?}", info.encoding),
                "lineEnding": info.line_ending.map(|le| format!("{:?}", le)),
                "isSymlink": is_symlink,
                "isReadonly": info.is_readonly,
            }
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{File, self};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    
    #[tokio::test]
    async fn test_read_file_with_encoding() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("utf8.txt");
        
        // Write UTF-8 with special characters
        let content = "Hello 世界\nLine 2 with émoji 😀";
        fs::write(&file_path, content).unwrap();
        
        let tool = ReadFileToolV2;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <path>utf8.txt</path>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.result["content"].as_str().unwrap(), content);
        assert_eq!(result.result["metadata"]["encoding"].as_str().unwrap(), "Utf8");
    }
    
    #[tokio::test]
    async fn test_read_file_with_bom() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("bom.txt");
        
        // Write UTF-8 with BOM
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"\xEF\xBB\xBF").unwrap(); // UTF-8 BOM
        file.write_all("Hello BOM".as_bytes()).unwrap();
        
        let tool = ReadFileToolV2;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <path>bom.txt</path>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        // BOM should be stripped
        assert_eq!(result.result["content"].as_str().unwrap(), "Hello BOM");
        assert_eq!(result.result["metadata"]["encoding"].as_str().unwrap(), "Utf8Bom");
    }
    
    #[tokio::test]
    async fn test_read_file_line_endings() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test different line endings
        let cases = vec![
            ("unix.txt", "line1\nline2\nline3", "Lf"),
            ("windows.txt", "line1\r\nline2\r\nline3", "CrLf"),
            ("mixed.txt", "line1\r\nline2\nline3", "Mixed"),
        ];
        
        for (filename, content, expected_ending) in cases {
            let file_path = temp_dir.path().join(filename);
            fs::write(&file_path, content).unwrap();
            
            let tool = ReadFileToolV2;
            let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
            
            let args = json!(format!(r#"
                <tool>
                    <path>{}</path>
                </tool>
            "#, filename));
            
            let result = tool.execute(args, context).await.unwrap();
            assert!(result.success);
            assert!(result.result["metadata"]["lineEnding"]
                .as_str()
                .unwrap()
                .contains(expected_ending));
        }
    }
    
    #[tokio::test]
    async fn test_read_file_size_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.txt");
        
        // Create a file larger than the limit
        let large_content = "x".repeat(1024);
        fs::write(&file_path, &large_content).unwrap();
        
        let tool = ReadFileToolV2;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        // Set a small max size
        let args = json!(format!(r#"
            <tool>
                <path>large.txt</path>
                <maxSize>512</maxSize>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        if let Err(ToolError::Other(msg)) = result {
            assert!(msg.contains("exceeds size limit"));
        } else {
            panic!("Expected size limit error");
        }
    }
    
    #[tokio::test]
    async fn test_read_file_symlink_handling() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("target.txt");
        let link_path = temp_dir.path().join("link.txt");
        
        fs::write(&target_path, "symlink target content").unwrap();
        
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_path, &link_path).unwrap();
            
            let tool = ReadFileToolV2;
            let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
            
            // Test following symlinks (default)
            let args = json!(format!(r#"
                <tool>
                    <path>link.txt</path>
                </tool>
            "#));
            
            let result = tool.execute(args, context.clone()).await.unwrap();
            assert!(result.success);
            assert_eq!(result.result["content"].as_str().unwrap(), "symlink target content");
            assert_eq!(result.result["metadata"]["isSymlink"], true);
            
            // Test not following symlinks
            let args = json!(format!(r#"
                <tool>
                    <path>link.txt</path>
                    <followSymlinks>false</followSymlinks>
                </tool>
            "#));
            
            let result = tool.execute(args, context).await;
            assert!(result.is_err());
        }
    }
    
    #[tokio::test]
    async fn test_read_file_readonly_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("readonly.txt");
        
        fs::write(&file_path, "readonly content").unwrap();
        
        // Make file readonly on Unix
        #[cfg(unix)]
        {
            let metadata = fs::metadata(&file_path).unwrap();
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o444); // Read-only for all
            fs::set_permissions(&file_path, permissions).unwrap();
        }
        
        let tool = ReadFileToolV2;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let args = json!(format!(r#"
            <tool>
                <path>readonly.txt</path>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await.unwrap();
        assert!(result.success);
        
        #[cfg(unix)]
        assert_eq!(result.result["metadata"]["isReadonly"], true);
    }
    
    #[tokio::test]
    async fn test_read_file_line_range_validation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("lines.txt");
        
        let mut content = String::new();
        for i in 1..=20 {
            content.push_str(&format!("Line {}\n", i));
        }
        fs::write(&file_path, content).unwrap();
        
        let tool = ReadFileToolV2;
        let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        // Test valid range
        let args = json!(format!(r#"
            <tool>
                <path>lines.txt</path>
                <lineStart>5</lineStart>
                <lineEnd>10</lineEnd>
            </tool>
        "#));
        
        let result = tool.execute(args, context.clone()).await.unwrap();
        assert!(result.success);
        let content = result.result["content"].as_str().unwrap();
        assert!(content.contains("Line 5"));
        assert!(content.contains("Line 10"));
        assert!(!content.contains("Line 4"));
        assert!(!content.contains("Line 11"));
        
        // Test invalid range (start > end)
        let args = json!(format!(r#"
            <tool>
                <path>lines.txt</path>
                <lineStart>10</lineStart>
                <lineEnd>5</lineEnd>
            </tool>
        "#));
        
        let result = tool.execute(args, context.clone()).await;
        assert!(result.is_err());
        
        // Test incomplete range (only start)
        let args = json!(format!(r#"
            <tool>
                <path>lines.txt</path>
                <lineStart>5</lineStart>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
    }
}
