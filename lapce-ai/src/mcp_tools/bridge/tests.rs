// Comprehensive integration tests for the MCP bridge
// Tests real core tools exposed via MCP with full safety/streaming/approvals

#[cfg(test)]
mod tests {
    use crate::mcp_tools::{
        dispatcher::McpToolSystem,
        config::McpServerConfig,
    };
    use serde_json::json;
    use tempfile::TempDir;
    use std::fs;
    
    #[tokio::test]
    async fn test_readfile_via_mcp() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Create test file with UTF-8 content
        let test_content = "Hello from core tool!\nLine 2 with émoji 😀";
        fs::write(workspace.join("test.txt"), test_content).unwrap();
        
        // Create MCP system (will register bridged core tools)
        let config = McpServerConfig::default();
        let system = McpToolSystem::new(config, workspace);
        
        // Execute readFile via MCP
        let args = json!(r#"
            <tool>
                <path>test.txt</path>
            </tool>
        "#);
        
        let result = system.execute_tool("readFile", args).await.unwrap();
        
        assert!(result.success);
        let data = result.data.unwrap();
        assert_eq!(data["content"], test_content);
        assert_eq!(data["metadata"]["encoding"], "Utf8");
    }
    
    #[tokio::test]
    async fn test_writefile_via_mcp() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let config = McpServerConfig::default();
        let system = McpToolSystem::new(config, workspace.clone());
        
        // Write file via MCP
        let args = json!(r#"
            <tool>
                <path>output.txt</path>
                <content>Written via MCP bridge!</content>
            </tool>
        "#);
        
        let result = system.execute_tool("writeFile", args).await.unwrap();
        
        assert!(result.success);
        
        // Verify file was written
        let content = fs::read_to_string(workspace.join("output.txt")).unwrap();
        assert_eq!(content, "Written via MCP bridge!");
    }
    
    #[tokio::test]
    async fn test_rooignore_enforcement() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Create .rooignore blocking *.secret
        fs::write(workspace.join(".rooignore"), "*.secret\n").unwrap();
        
        // Create a secret file
        fs::write(workspace.join("api.secret"), "secret_token_123").unwrap();
        
        let config = McpServerConfig::default();
        let system = McpToolSystem::new(config, workspace);
        
        // Try to read blocked file
        let args = json!(r#"
            <tool>
                <path>api.secret</path>
            </tool>
        "#);
        
        let result = system.execute_tool("readFile", args).await;
        
        // Should fail due to .rooignore
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("rooignore") || err_msg.contains("blocked"));
    }
    
    #[tokio::test]
    async fn test_dangerous_command_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let config = McpServerConfig::default();
        let system = McpToolSystem::new(config, workspace);
        
        // Try to execute dangerous rm command
        let args = json!(r#"
            <tool>
                <command>rm -rf /tmp/test</command>
            </tool>
        "#);
        
        let result = system.execute_tool("executeCommand", args).await;
        
        // Should be blocked with trash-put suggestion
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("trash-put") || err_msg.contains("blocked"));
    }
    
    #[tokio::test]
    async fn test_search_files() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Create test files
        fs::write(workspace.join("test1.txt"), "function hello() {}").unwrap();
        fs::write(workspace.join("test2.txt"), "const world = 42;").unwrap();
        fs::write(workspace.join("test3.txt"), "hello world").unwrap();
        
        let config = McpServerConfig::default();
        let system = McpToolSystem::new(config, workspace);
        
        // Search for "hello"
        let args = json!(r#"
            <tool>
                <pattern>hello</pattern>
                <path>.</path>
            </tool>
        "#);
        
        let result = system.execute_tool("searchFiles", args).await.unwrap();
        
        assert!(result.success);
        let data = result.data.unwrap();
        
        // Should find matches in test1.txt and test3.txt
        let matches = data["matches"].as_array().expect("Should have matches array");
        assert!(matches.len() >= 2, "Should find at least 2 matches");
    }
    
    #[tokio::test]
    async fn test_git_tools_via_aliases() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let config = McpServerConfig::default();
        let system = McpToolSystem::new(config, workspace);
        
        // Verify git tools are registered with aliases
        assert!(system.get_tool("git_status").is_some(), "git_status should be registered");
        assert!(system.get_tool("gitStatus").is_some(), "gitStatus alias should work");
        assert!(system.get_tool("git_diff").is_some(), "git_diff should be registered");
        assert!(system.get_tool("gitDiff").is_some(), "gitDiff alias should work");
    }
    
    #[tokio::test]
    async fn test_encoding_preservation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Write file with UTF-8 BOM
        let mut content = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
        content.extend_from_slice("Test content".as_bytes());
        fs::write(workspace.join("bom.txt"), content).unwrap();
        
        let config = McpServerConfig::default();
        let system = McpToolSystem::new(config, workspace);
        
        // Read file
        let args = json!(r#"
            <tool>
                <path>bom.txt</path>
            </tool>
        "#);
        
        let result = system.execute_tool("readFile", args).await.unwrap();
        
        assert!(result.success);
        let data = result.data.unwrap();
        
        // BOM should be detected and stripped from content
        assert_eq!(data["content"], "Test content");
        assert_eq!(data["metadata"]["encoding"], "Utf8Bom");
    }
    
    #[tokio::test]
    async fn test_line_range_support() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Create file with 10 lines
        let content = (1..=10)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(workspace.join("lines.txt"), content).unwrap();
        
        let config = McpServerConfig::default();
        let system = McpToolSystem::new(config, workspace);
        
        // Read lines 3-7
        let args = json!(r#"
            <tool>
                <path>lines.txt</path>
                <lineStart>3</lineStart>
                <lineEnd>7</lineEnd>
            </tool>
        "#);
        
        let result = system.execute_tool("readFile", args).await.unwrap();
        
        assert!(result.success);
        let data = result.data.unwrap();
        let content = data["content"].as_str().unwrap();
        
        assert!(content.contains("Line 3"));
        assert!(content.contains("Line 7"));
        assert!(!content.contains("Line 2"));
        assert!(!content.contains("Line 8"));
    }
    
    #[tokio::test]
    async fn test_all_core_tools_registered() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let config = McpServerConfig::default();
        let system = McpToolSystem::new(config, workspace);
        
        let tools = system.list_tools();
        
        // Verify key production tools are present
        let expected_tools = vec![
            "readFile",
            "writeFile",
            "searchFiles",
            "listFiles",
            "editFile",
            "insertContent",
            "searchAndReplace",
            "executeCommand",
            "terminal",
            "applyDiff",
            "git_status",
            "git_diff",
            "base64",
            "curl",
        ];
        
        for tool_name in expected_tools {
            assert!(
                tools.iter().any(|t| t.name == tool_name),
                "Tool '{}' should be registered",
                tool_name
            );
        }
        
        // Should have at least 19 core tools
        assert!(tools.len() >= 19, "Should have at least 19 tools, got {}", tools.len());
    }
}
