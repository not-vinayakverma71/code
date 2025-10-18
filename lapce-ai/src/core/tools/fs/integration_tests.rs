// Integration tests for hardened FS tools
// Part of Core FS tools hardening - pre-IPC TODO

#[cfg(test)]
mod integration_tests {
    use tempfile::TempDir;
    use std::fs;
    use std::io::Write;
    use serde_json::json;
    use crate::core::tools::traits::{Tool, ToolContext};
    use crate::core::tools::fs::{
        read_file_v2::ReadFileToolV2,
        write_file_v2::WriteFileToolV2,
        search_and_replace_v2::SearchAndReplaceToolV2,
    };
    
    #[tokio::test]
    async fn test_end_to_end_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("workflow.txt");
        
        // Step 1: Write a file with specific encoding and line endings
        let write_tool = WriteFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.permissions.file_read = true;
        context.require_approval = false;
        
        let content = "Line 1 with Unix ending\nLine 2 with data\nLine 3 with more data";
        let write_args = json!(format!(r#"
            <tool>
                <path>workflow.txt</path>
                <content>{}</content>
                <forceLineEnding>lf</forceLineEnding>
            </tool>
        "#, content));
        
        let write_result = write_tool.execute(write_args, context.clone()).await.unwrap();
        assert!(write_result.success);
        
        // Step 2: Read the file and verify metadata
        let read_tool = ReadFileToolV2;
        let read_args = json!(r#"
            <tool>
                <path>workflow.txt</path>
            </tool>
        "#);
        
        let read_result = read_tool.execute(read_args, context.clone()).await.unwrap();
        assert!(read_result.success);
        assert_eq!(read_result.result["content"].as_str().unwrap(), content);
        assert!(read_result.result["metadata"]["lineEnding"].as_str().unwrap().contains("Lf"));
        
        // Step 3: Search and replace with line range
        let sr_tool = SearchAndReplaceToolV2;
        let sr_args = json!(r#"
            <tool>
                <path>workflow.txt</path>
                <search>data</search>
                <replace>INFO</replace>
                <mode>literal</mode>
                <lineStart>2</lineStart>
                <lineEnd>3</lineEnd>
                <preserveLineEndings>true</preserveLineEndings>
            </tool>
        "#);
        
        let sr_result = sr_tool.execute(sr_args, context.clone()).await.unwrap();
        assert!(sr_result.success);
        assert_eq!(sr_result.result["replacements_made"], 2);
        
        // Step 4: Read again and verify changes
        let read_args2 = json!(r#"
            <tool>
                <path>workflow.txt</path>
            </tool>
        "#);
        
        let read_result2 = read_tool.execute(read_args2, context).await.unwrap();
        assert!(read_result2.success);
        
        let final_content = read_result2.result["content"].as_str().unwrap();
        assert!(final_content.contains("Line 1 with Unix ending"));
        assert!(final_content.contains("Line 2 with INFO"));
        assert!(final_content.contains("Line 3 with more INFO"));
    }
    
    #[tokio::test]
    async fn test_mixed_line_endings_preservation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("mixed.txt");
        
        // Create file with mixed line endings
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"Unix line\n").unwrap();
        file.write_all(b"Windows line\r\n").unwrap();
        file.write_all(b"Another Unix\n").unwrap();
        drop(file);
        
        // Read and check detection
        let read_tool = ReadFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = true;
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let read_args = json!(r#"
            <tool>
                <path>mixed.txt</path>
            </tool>
        "#);
        
        let read_result = read_tool.execute(read_args, context.clone()).await.unwrap();
        assert!(read_result.success);
        assert!(read_result.result["metadata"]["lineEnding"].as_str().unwrap().contains("Mixed"));
        
        // Write with preservation
        let write_tool = WriteFileToolV2;
        let write_args = json!(r#"
            <tool>
                <path>mixed.txt</path>
                <content>New Unix line
New Windows line
New Another Unix</content>
                <preserveLineEndings>true</preserveLineEndings>
            </tool>
        "#);
        
        let write_result = write_tool.execute(write_args, context).await.unwrap();
        assert!(write_result.success);
        
        // Verify line endings were preserved (normalized to LF for mixed)
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains('\n'));
    }
    
    #[tokio::test]
    async fn test_utf8_bom_handling() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("bom.txt");
        
        // Create UTF-8 file with BOM
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(&[0xEF, 0xBB, 0xBF]).unwrap(); // UTF-8 BOM
        file.write_all("Content with BOM".as_bytes()).unwrap();
        drop(file);
        
        // Read - BOM should be detected and stripped
        let read_tool = ReadFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = true;
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let read_args = json!(r#"
            <tool>
                <path>bom.txt</path>
            </tool>
        "#);
        
        let read_result = read_tool.execute(read_args, context.clone()).await.unwrap();
        assert!(read_result.success);
        assert_eq!(read_result.result["content"].as_str().unwrap(), "Content with BOM");
        assert_eq!(read_result.result["metadata"]["encoding"].as_str().unwrap(), "Utf8Bom");
        
        // Write with BOM preservation
        let write_tool = WriteFileToolV2;
        let write_args = json!(r#"
            <tool>
                <path>bom.txt</path>
                <content>New content with BOM preserved</content>
                <preserveEncoding>true</preserveEncoding>
            </tool>
        "#);
        
        let write_result = write_tool.execute(write_args, context).await.unwrap();
        assert!(write_result.success);
        
        // Verify BOM is still present
        let raw_content = fs::read(&file_path).unwrap();
        assert_eq!(&raw_content[..3], &[0xEF, 0xBB, 0xBF]);
    }
    
    #[tokio::test]
    async fn test_large_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.txt");
        
        // Create a file near the size limit
        let line = "This is a test line with some content.\n";
        let repeat_count = 1024; // Creates ~40KB file
        let content = line.repeat(repeat_count);
        fs::write(&file_path, &content).unwrap();
        
        let read_tool = ReadFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = true;
        
        // Test with adequate size limit
        let read_args = json!(r#"
            <tool>
                <path>large.txt</path>
                <maxSize>1048576</maxSize>
            </tool>
        "#);
        
        let read_result = read_tool.execute(read_args, context.clone()).await.unwrap();
        assert!(read_result.success);
        
        // Test with too small size limit
        let read_args_fail = json!(r#"
            <tool>
                <path>large.txt</path>
                <maxSize>1024</maxSize>
            </tool>
        "#);
        
        let read_result_fail = read_tool.execute(read_args_fail, context).await;
        assert!(read_result_fail.is_err());
    }
    
    #[tokio::test]
    async fn test_binary_file_detection() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create binary file
        let binary_path = temp_dir.path().join("binary.bin");
        fs::write(&binary_path, &[0, 1, 2, 3, 255, 254, 253]).unwrap();
        
        // Create image file
        let image_path = temp_dir.path().join("image.png");
        fs::write(&image_path, &[0x89, 0x50, 0x4E, 0x47]).unwrap(); // PNG header
        
        let read_tool = ReadFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = true;
        
        // Test binary file
        let binary_args = json!(r#"
            <tool>
                <path>binary.bin</path>
            </tool>
        "#);
        
        let binary_result = read_tool.execute(binary_args, context.clone()).await.unwrap();
        assert!(binary_result.success);
        assert_eq!(binary_result.result["type"], "binary");
        assert!(binary_result.result["message"].as_str().unwrap().contains("Binary file detected"));
        
        // Test image file
        let image_args = json!(r#"
            <tool>
                <path>image.png</path>
            </tool>
        "#);
        
        let image_result = read_tool.execute(image_args, context).await.unwrap();
        assert!(image_result.success);
        assert_eq!(image_result.result["type"], "image");
        assert!(image_result.result["message"].as_str().unwrap().contains("Image file detected"));
    }
    
    #[tokio::test]
    async fn test_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("deep/nested/dir/file.txt");
        
        let write_tool = WriteFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        // Write without createDirs should fail
        let write_args_fail = json!(r#"
            <tool>
                <path>deep/nested/dir/file.txt</path>
                <content>Content</content>
                <createDirs>false</createDirs>
            </tool>
        "#);
        
        let write_result_fail = write_tool.execute(write_args_fail, context.clone()).await;
        assert!(write_result_fail.is_err());
        
        // Write with createDirs should succeed
        let write_args = json!(r#"
            <tool>
                <path>deep/nested/dir/file.txt</path>
                <content>Content in nested dir</content>
                <createDirs>true</createDirs>
            </tool>
        "#);
        
        let write_result = write_tool.execute(write_args, context).await.unwrap();
        assert!(write_result.success);
        
        // Verify file was created
        assert!(nested_path.exists());
        assert_eq!(fs::read_to_string(&nested_path).unwrap(), "Content in nested dir");
    }
    
    #[tokio::test]
    async fn test_readonly_file_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("readonly.txt");
        
        fs::write(&file_path, "readonly content").unwrap();
        
        // Make file readonly on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&file_path).unwrap();
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o444); // Read-only for all
            fs::set_permissions(&file_path, permissions).unwrap();
        }
        
        let read_tool = ReadFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = true;
        
        let read_args = json!(r#"
            <tool>
                <path>readonly.txt</path>
            </tool>
        "#);
        
        let read_result = read_tool.execute(read_args, context).await.unwrap();
        assert!(read_result.success);
        
        #[cfg(unix)]
        assert_eq!(read_result.result["metadata"]["isReadonly"], true);
        
        // Restore permissions for cleanup
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&file_path).unwrap();
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o644);
            fs::set_permissions(&file_path, permissions).unwrap();
        }
    }
    
    #[tokio::test]
    async fn test_line_range_operations() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("lines.txt");
        
        // Create file with numbered lines
        let mut content = String::new();
        for i in 1..=100 {
            content.push_str(&format!("Line {} content\n", i));
        }
        fs::write(&file_path, &content).unwrap();
        
        // Test reading specific line range
        let read_tool = ReadFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = true;
        context.permissions.file_write = true;
        context.require_approval = false;
        
        let read_args = json!(r#"
            <tool>
                <path>lines.txt</path>
                <lineStart>10</lineStart>
                <lineEnd>15</lineEnd>
            </tool>
        "#);
        
        let read_result = read_tool.execute(read_args, context.clone()).await.unwrap();
        assert!(read_result.success);
        
        let range_content = read_result.result["content"].as_str().unwrap();
        assert!(range_content.contains("10 | Line 10"));
        assert!(range_content.contains("15 | Line 15"));
        assert!(!range_content.contains("Line 9"));
        assert!(!range_content.contains("Line 16"));
        
        // Test search/replace in line range
        let sr_tool = SearchAndReplaceToolV2;
        let sr_args = json!(r#"
            <tool>
                <path>lines.txt</path>
                <search>content</search>
                <replace>DATA</replace>
                <mode>literal</mode>
                <lineStart>20</lineStart>
                <lineEnd>25</lineEnd>
            </tool>
        "#);
        
        let sr_result = sr_tool.execute(sr_args, context).await.unwrap();
        assert!(sr_result.success);
        assert_eq!(sr_result.result["replacements_made"], 6); // Lines 20-25
        
        // Verify changes
        let final_content = fs::read_to_string(&file_path).unwrap();
        let lines: Vec<&str> = final_content.lines().collect();
        
        assert!(lines[18].contains("content")); // Line 19 unchanged
        assert!(lines[19].contains("DATA"));     // Line 20 changed
        assert!(lines[24].contains("DATA"));     // Line 25 changed
        assert!(lines[25].contains("content")); // Line 26 unchanged
    }
    
    #[tokio::test]
    async fn test_code_artifact_removal() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("code.txt");
        
        let write_tool = WriteFileToolV2;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false;
        
        // Write content with code fences and line numbers
        let content_with_artifacts = r#"```rust
fn main() {
    println!("Hello");
}
```

Normal text
   1 | fn test() {
   2 |     let x = 5;
   3 | }
123: numbered line
[456] bracketed line"#;
        
        let write_args = json!(format!(r#"
            <tool>
                <path>code.txt</path>
                <content>{}</content>
            </tool>
        "#, content_with_artifacts.replace('"', r#"\""#)));
        
        let write_result = write_tool.execute(write_args, context).await.unwrap();
        assert!(write_result.success);
        
        // Read the file
        let final_content = fs::read_to_string(&file_path).unwrap();
        
        // Verify artifacts were removed
        assert!(!final_content.contains("```"));
        assert!(final_content.contains("fn main()"));
        assert!(final_content.contains("fn test()"));
        assert!(!final_content.contains("   1 |"));
        assert!(!final_content.contains("123:"));
        assert!(!final_content.contains("[456]"));
        assert!(final_content.contains("numbered line"));
        assert!(final_content.contains("bracketed line"));
    }
}
