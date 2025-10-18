// REAL MCP Tools Integration Tests
use lapce_ai_rust::mcp_tools::{
    core::{McpToolSystem, McpConfig, ToolContext, ToolResult},
    tools::{
        filesystem_tool::{ReadFileTool, WriteFileTool, ListFilesTool},
        terminal_tool::ExecuteCommandTool,
        search_files::SearchFilesTool,
    },
};
use std::path::PathBuf;
use std::time::Duration;
use serde_json::json;
use tokio;

#[tokio::test]
async fn test_mcp_tool_system_compiles() {
    // Test that we can create an McpToolSystem instance
    let config = McpConfig {
        max_memory_mb: 3,
        max_cpu_seconds: 10,
        enable_sandboxing: true,
        enable_rate_limiting: true,
        enable_caching: false,
        cache_ttl: Duration::from_secs(60),
        rate_limit_per_minute: 100,
        max_file_size: 10 * 1024 * 1024,
        allowed_paths: vec!["/tmp".into()],
    };
    
    let system = McpToolSystem::new(config).await;
    assert!(system.is_ok(), "Failed to create McpToolSystem");
}

#[tokio::test]
async fn test_read_file_tool() {
    // Create a test file
    let test_file = "/tmp/test_read.txt";
    std::fs::write(test_file, "Hello, MCP Tools!").unwrap();
    
    // Create tool and context
    let tool = ReadFileTool;
    let context = ToolContext {
        user_id: "test_user".to_string(),
        workspace: PathBuf::from("/tmp"),
        permissions: vec!["read".into()],
        env_vars: Default::default(),
        timeout: Duration::from_secs(10),
    };
    
    // Execute tool
    let args = json!({
        "path": test_file
    });
    
    let result = tool.execute(args, context).await;
    assert!(result.is_ok(), "ReadFileTool failed: {:?}", result);
    
    let tool_result = result.unwrap();
    assert!(tool_result.success, "Tool execution failed");
    
    // Check content
    let content = tool_result.output.as_str().unwrap();
    assert!(content.contains("Hello, MCP Tools!"), "Content mismatch");
    
    // Cleanup
    std::fs::remove_file(test_file).ok();
}

#[tokio::test]
async fn test_write_file_tool() {
    let test_file = "/tmp/test_write.txt";
    
    // Create tool and context
    let tool = WriteFileTool;
    let context = ToolContext {
        user_id: "test_user".to_string(),
        workspace: PathBuf::from("/tmp"),
        permissions: vec!["write".into()],
        env_vars: Default::default(),
        timeout: Duration::from_secs(10),
    };
    
    // Execute tool
    let args = json!({
        "path": test_file,
        "content": "Written by MCP Tools!"
    });
    
    let result = tool.execute(args, context).await;
    assert!(result.is_ok(), "WriteFileTool failed: {:?}", result);
    
    let tool_result = result.unwrap();
    assert!(tool_result.success, "Tool execution failed");
    
    // Verify file was written
    let content = std::fs::read_to_string(test_file).unwrap();
    assert_eq!(content, "Written by MCP Tools!");
    
    // Cleanup
    std::fs::remove_file(test_file).ok();
}

#[tokio::test]
async fn test_execute_command_tool() {
    // Create tool and context
    let tool = ExecuteCommandTool;
    let context = ToolContext {
        user_id: "test_user".to_string(),
        workspace: PathBuf::from("/tmp"),
        permissions: vec!["execute".into()],
        env_vars: Default::default(),
        timeout: Duration::from_secs(10),
    };
    
    // Execute tool
    let args = json!({
        "command": "echo",
        "args": ["Hello from MCP Tools"]
    });
    
    let result = tool.execute(args, context).await;
    assert!(result.is_ok(), "ExecuteCommandTool failed: {:?}", result);
    
    let tool_result = result.unwrap();
    assert!(tool_result.success, "Tool execution failed");
    
    // Check output
    let output = tool_result.output.as_str().unwrap();
    assert!(output.contains("Hello from MCP Tools"), "Output mismatch");
}

#[tokio::test]
async fn test_list_files_tool() {
    // Create test files
    std::fs::write("/tmp/test1.txt", "file1").unwrap();
    std::fs::write("/tmp/test2.txt", "file2").unwrap();
    
    // Create tool and context
    let tool = ListFilesTool;
    let context = ToolContext {
        user_id: "test_user".to_string(),
        workspace: PathBuf::from("/tmp"),
        permissions: vec!["read".into()],
        env_vars: Default::default(),
        timeout: Duration::from_secs(10),
    };
    
    // Execute tool
    let args = json!({
        "path": "/tmp"
    });
    
    let result = tool.execute(args, context).await;
    assert!(result.is_ok(), "ListFilesTool failed: {:?}", result);
    
    let tool_result = result.unwrap();
    assert!(tool_result.success, "Tool execution failed");
    
    // Check files are listed
    let output_str = tool_result.output.as_str().unwrap();
    assert!(output_str.contains("test1.txt"), "Missing test1.txt");
    assert!(output_str.contains("test2.txt"), "Missing test2.txt");
    
    // Cleanup
    std::fs::remove_file("/tmp/test1.txt").ok();
    std::fs::remove_file("/tmp/test2.txt").ok();
}

#[tokio::test]
async fn test_search_files_tool() {
    // Create test files with content
    std::fs::write("/tmp/search1.txt", "Find this pattern: MCP").unwrap();
    std::fs::write("/tmp/search2.txt", "Another file without pattern").unwrap();
    
    // Create tool and context
    let tool = SearchFilesTool;
    let context = ToolContext {
        user_id: "test_user".to_string(),
        workspace: PathBuf::from("/tmp"),
        permissions: vec!["read".into()],
        env_vars: Default::default(),
        timeout: Duration::from_secs(10),
    };
    
    // Execute tool
    let args = json!({
        "pattern": "MCP",
        "path": "/tmp"
    });
    
    let result = tool.execute(args, context).await;
    assert!(result.is_ok(), "SearchFilesTool failed: {:?}", result);
    
    let tool_result = result.unwrap();
    assert!(tool_result.success, "Tool execution failed");
    
    // Check search results
    let output_str = tool_result.output.as_str().unwrap();
    assert!(output_str.contains("search1.txt"), "Should find search1.txt");
    assert!(!output_str.contains("search2.txt"), "Should not find search2.txt");
    
    // Cleanup
    std::fs::remove_file("/tmp/search1.txt").ok();
    std::fs::remove_file("/tmp/search2.txt").ok();
}

#[tokio::test]
async fn test_sandbox_blocks_etc_access() {
    // Create tool and context
    let tool = ReadFileTool;
    let context = ToolContext {
        user_id: "test_user".to_string(),
        workspace: PathBuf::from("/tmp"),
        permissions: vec!["read".into()],
        env_vars: Default::default(),
        timeout: Duration::from_secs(10),
    };
    
    // Try to read /etc/passwd (should be blocked)
    let args = json!({
        "path": "/etc/passwd"
    });
    
    let result = tool.execute(args, context).await;
    
    // Should either fail or return error in result
    if let Ok(tool_result) = result {
        assert!(!tool_result.success, "Should not be able to read /etc/passwd");
        let error_str = tool_result.error.as_ref().unwrap().as_str().unwrap();
        assert!(error_str.contains("blocked") || error_str.contains("denied"), 
            "Should indicate access denied");
    } else {
        // Error is also acceptable
        assert!(true, "Access to /etc/passwd blocked as expected");
    }
}

#[tokio::test]
async fn test_memory_usage() {
    use std::fs;
    
    // Get initial memory
    let status = fs::read_to_string("/proc/self/status").unwrap();
    let initial_memory = status.lines()
        .find(|l| l.starts_with("VmRSS:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    
    println!("Initial memory: {} KB", initial_memory);
    
    // Create MCP Tool System
    let config = McpConfig {
        max_memory_mb: 3,
        max_cpu_seconds: 10,
        enable_sandboxing: true,
        enable_rate_limiting: true,
        enable_caching: false,
        cache_ttl: Duration::from_secs(60),
        rate_limit_per_minute: 100,
        max_file_size: 10 * 1024 * 1024,
        allowed_paths: vec!["/tmp".into()],
    };
    
    let _system = McpToolSystem::new(config).await.unwrap();
    
    // Get memory after creating system
    let status = fs::read_to_string("/proc/self/status").unwrap();
    let after_memory = status.lines()
        .find(|l| l.starts_with("VmRSS:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    
    println!("After creating MCP system: {} KB", after_memory);
    let memory_increase = after_memory - initial_memory;
    println!("Memory increase: {} KB", memory_increase);
    
    // Check memory usage is under 3MB (3072 KB)
    assert!(memory_increase < 3072, "Memory usage exceeded 3MB: {} KB", memory_increase);
}

#[tokio::test]
async fn test_dispatch_performance() {
    use std::time::Instant;
    
    // Create tool and context
    let tool = ReadFileTool;
    let context = ToolContext {
        user_id: "test_user".to_string(),
        workspace: PathBuf::from("/tmp"),
        permissions: vec!["read".into()],
        env_vars: Default::default(),
        timeout: Duration::from_secs(10),
    };
    
    // Create test file
    std::fs::write("/tmp/perf_test.txt", "Performance test").unwrap();
    
    // Measure dispatch time
    let start = Instant::now();
    
    let args = json!({
        "path": "/tmp/perf_test.txt"
    });
    
    let _result = tool.execute(args, context).await;
    
    let dispatch_time = start.elapsed();
    
    println!("Dispatch time: {:?}", dispatch_time);
    
    // Check dispatch time is under 10ms
    assert!(dispatch_time.as_millis() < 10, "Dispatch time exceeded 10ms: {:?}", dispatch_time);
    
    // Cleanup
    std::fs::remove_file("/tmp/perf_test.txt").ok();
}
