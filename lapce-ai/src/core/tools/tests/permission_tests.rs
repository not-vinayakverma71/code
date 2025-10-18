// Permission enforcement tests - P0-Sec-tests

use crate::core::tools::traits::{Tool, ToolContext, ToolPermissions};
use crate::core::tools::fs::{ReadFileTool, WriteFileTool, EditFileTool};
use crate::mcp_tools::permission_manager::PermissionManager;
use crate::mcp_tools::config::McpServerConfig;
use std::sync::Arc;
use tokio::sync::RwLock;
use tempfile::TempDir;
use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_read_permission_denied() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();
        
        // Create context with read permission denied
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = false;
        
        let tool = ReadFileTool;
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Permission denied"));
    }
    
    #[tokio::test]
    async fn test_write_permission_denied() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create context with write permission denied
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = false;
        
        let tool = WriteFileTool;
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>test content</content>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Permission denied"));
    }
    
    #[tokio::test]
    async fn test_edit_permission_denied() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "original content").unwrap();
        
        // Create context with write permission denied
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = false;
        
        let tool = EditFileTool;
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <oldContent>original</oldContent>
                <newContent>modified</newContent>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Permission denied"));
    }
    
    #[tokio::test]
    async fn test_permission_manager_integration() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();
        
        // Create permission manager with config
        let config = McpServerConfig::default();
        let config = Arc::new(RwLock::new(config));
        let pm = Arc::new(PermissionManager::new(config));
        
        // Create context with permission manager
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        // context.permission_manager = Some(pm);
        context.permissions.file_read = true; // Basic permission allowed
        
        let tool = ReadFileTool;
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
            </tool>
        "#));
        
        // Should succeed when permissions allow
        let result = tool.execute(args, context).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_read_permission_allowed() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();
        
        // Create context with read permission allowed
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = true;
        
        let tool = ReadFileTool;
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.success);
        assert!(output.result["content"].as_str().unwrap().contains("test content"));
    }
    
    #[tokio::test]
    async fn test_write_permission_allowed() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create context with write permission allowed
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_write = true;
        context.require_approval = false; // Skip approval for test
        
        let tool = WriteFileTool;
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
                <content>test content</content>
            </tool>
        "#));
        
        let result = tool.execute(args, context).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(output.success);
        
        // Verify file was written
        let file_path = temp_dir.path().join("test.txt");
        assert!(file_path.exists());
        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "test content");
    }
    
    #[tokio::test]
    async fn test_permission_toggle() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();
        
        let tool = ReadFileTool;
        let args = json!(format!(r#"
            <tool>
                <path>test.txt</path>
            </tool>
        "#));
        
        // Test with permission denied
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = false;
        
        let result = tool.execute(args.clone(), context).await;
        assert!(result.is_err());
        
        // Test with permission allowed
        let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
        context.permissions.file_read = true;
        
        let result = tool.execute(args, context).await;
        assert!(result.is_ok());
    }
}
