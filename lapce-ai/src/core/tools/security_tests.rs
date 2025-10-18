// Security hardening tests - P0-SEC

#[cfg(test)]
mod tests {
    use super::super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_workspace_escape_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Test various escape attempts
        let escape_attempts = vec![
            "../../../etc/passwd",
            "/etc/passwd",
            "../../sensitive.txt",
            "/home/user/.ssh/id_rsa",
            "~/../../etc/shadow",
            "./../../../../../../tmp/evil",
        ];
        
        for path in escape_attempts {
            let result = super::super::fs::ensure_workspace_path(&workspace, &PathBuf::from(path));
            assert!(result.is_err(), "Should block escape attempt: {}", path);
        }
    }
    
    #[tokio::test]
    async fn test_rooignore_enforcement() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create .rooignore file
        let rooignore_content = r#"
*.secret
.env
node_modules/
*.key
private/
"#;
        fs::write(temp_dir.path().join(".rooignore"), rooignore_content).unwrap();
        
        // Create test files
        fs::write(temp_dir.path().join("test.secret"), "secret data").unwrap();
        fs::write(temp_dir.path().join(".env"), "API_KEY=secret").unwrap();
        fs::create_dir(temp_dir.path().join("private")).unwrap();
        fs::write(temp_dir.path().join("private/data.txt"), "private").unwrap();
        
        // Test that rooignore blocks access
        let context = traits::ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        let blocked_paths = vec![
            "test.secret",
            ".env",
            "private/data.txt",
            "api.key",
        ];
        
        for path in blocked_paths {
            let file_path = context.resolve_path(path);
            assert!(!context.is_path_allowed(&file_path), 
                "Should block rooignored path: {}", path);
        }
        
        // Test allowed paths
        let allowed_paths = vec![
            "normal.txt",
            "src/main.rs",
            "README.md",
        ];
        
        for path in allowed_paths {
            let file_path = context.resolve_path(path);
            assert!(context.is_path_allowed(&file_path), 
                "Should allow non-ignored path: {}", path);
        }
    }
    
    #[tokio::test]
    async fn test_command_injection_prevention() {
        use crate::core::tools::execute_command::ExecuteCommandTool;
        use crate::core::tools::traits::{Tool, ToolContext, ToolError};
        
        let temp_dir = TempDir::new().unwrap();
        let tool = ExecuteCommandTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false;
        context.permissions.execute = true; // Enable execute permission to test injection checks
        
        // Test command injection attempts (with XML-escaped ampersands)
        let injection_attempts = vec![
            ("ls; rm -rf /", "ls; rm -rf /"),
            ("echo test &amp;&amp; dd if=/dev/zero of=/dev/sda", "echo test && dd if=/dev/zero of=/dev/sda"),
            ("cat file.txt | sudo rm -rf /*", "cat file.txt | sudo rm -rf /*"),
            ("test`rm -rf /`", "test`rm -rf /`"),
            ("$(rm -rf /)", "$(rm -rf /)"),
            ("test;mkfs.ext4 /dev/sda", "test;mkfs.ext4 /dev/sda"),
        ];
        
        for (xml_cmd, display_cmd) in injection_attempts {
            let args = serde_json::json!(format!(r#"
                <tool>
                    <command>{}</command>
                </tool>
            "#, xml_cmd));
            
            let result = tool.execute(args, context.clone()).await;
            assert!(matches!(result, Err(ToolError::PermissionDenied(_))), 
                "Should block dangerous command: {}", display_cmd);
        }
    }
    
    #[tokio::test]
    async fn test_symlink_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Create a symlink pointing outside workspace
        let target = "/etc/passwd";
        let link = temp_dir.path().join("evil_link");
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let _ = symlink(target, &link);
            
            // Test that following symlink is blocked
            let result = super::super::fs::ensure_workspace_path(&workspace, &link);
            
            // Should either error or resolve to a path within workspace
            match result {
                Ok(path) => {
                    assert!(path.starts_with(&workspace), 
                        "Resolved path should be within workspace");
                }
                Err(_) => {
                    // Good - symlink traversal blocked
                }
            }
        }
    }
    
    #[tokio::test]
    async fn test_file_size_limits() {
        use crate::core::tools::fs::ReadFileTool;
        use crate::core::tools::traits::{Tool, ToolContext};
        
        let temp_dir = TempDir::new().unwrap();
        
        // Create a large file (simulate)
        let large_file = temp_dir.path().join("large.txt");
        let size = 150 * 1024 * 1024; // 150MB (larger than default 100MB limit)
        fs::write(&large_file, vec![b'a'; size]).unwrap();
        
        let tool = ReadFileTool;
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = false;
        
        // Configure max file size
        let max_size = context.max_file_size();
        assert!(max_size < size, "Max file size should be less than test file");
        
        let args = serde_json::json!(format!(r#"
            <tool>
                <path>large.txt</path>
            </tool>
        "#));
        
        // Should handle large file appropriately (error or truncate)
        let result = tool.execute(args, context).await;
        
        match result {
            Ok(output) => {
                // If successful, check that output is truncated
                let content_len = output.result["content"]
                    .as_str()
                    .map(|s| s.len())
                    .unwrap_or(0);
                assert!(content_len <= max_size, "Content should be truncated");
            }
            Err(_) => {
                // Good - large file rejected
            }
        }
    }
    
    #[tokio::test]
    async fn test_approval_bypass_prevention() {
        use crate::core::tools::fs::WriteFileTool;
        use crate::core::tools::traits::{Tool, ToolContext, ToolError};
        
        let temp_dir = TempDir::new().unwrap();
        let tool = WriteFileTool;
        
        // Create context with approval required
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.require_approval = true;
        context.permissions.file_write = true; // Enable write permission to test approval logic
        
        // Try to write without approval
        let args = serde_json::json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>malicious content</content>
            </tool>
        "#));
        
        let result = tool.execute(args.clone(), context.clone()).await;
        assert!(matches!(result, Err(ToolError::ApprovalRequired(_))), 
            "Should require approval for write operation");
        
        // Verify file was not created
        assert!(!temp_dir.path().join("test.txt").exists(), 
            "File should not be created without approval");
        
        // Test dry-run doesn't bypass approval for actual write
        context.dry_run = true;
        let result = tool.execute(args, context).await;
        assert!(result.is_ok(), "Dry run should succeed");
        assert!(!temp_dir.path().join("test.txt").exists(), 
            "File should not be created in dry-run mode");
    }
    
    #[tokio::test]
    async fn test_permission_downgrade_prevention() {
        use crate::core::tools::traits::ToolContext;
        
        let temp_dir = TempDir::new().unwrap();
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        
        // Set restrictive permissions
        context.permissions.file_write = false;
        context.permissions.execute = false;
        
        // Verify permissions cannot be bypassed
        let file_path = temp_dir.path().join("test.txt");
        assert!(!context.can_write_file(&file_path).await, 
            "Write should be denied with permissions.file_write = false");
        
        assert!(!context.can_execute_command(), 
            "Execute should be denied with permissions.execute = false");
    }
    
    #[test]
    fn test_path_normalization() {
        let workspace = PathBuf::from("/workspace");
        
        // Test path normalization
        let test_cases = vec![
            ("./file.txt", Ok("/workspace/file.txt")),
            ("subdir/../file.txt", Ok("/workspace/file.txt")),
            ("./subdir/./file.txt", Ok("/workspace/subdir/file.txt")),
            ("/absolute/path", Err("outside workspace")),
            ("../escape", Err("outside workspace")),
        ];
        
        for (input, expected) in test_cases {
            let result = super::super::fs::ensure_workspace_path(&workspace, &PathBuf::from(input));
            
            match expected {
                Ok(expected_path) => {
                    assert!(result.is_ok(), "Should succeed for: {}", input);
                    assert_eq!(result.unwrap().to_str().unwrap(), expected_path);
                }
                Err(_) => {
                    assert!(result.is_err(), "Should fail for: {}", input);
                }
            }
        }
    }
}
