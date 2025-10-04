/// Simple MCP Tools Test - Verify core functionality
use lapce_ai_rust::mcp_tools::{
    McpToolSystem,
    ToolContext,
    tools::{
        read_file::ReadFileTool,
        write_file::WriteFileTool,
        list_files::ListFilesTool,
        execute_command::ExecuteCommandTool,
        search_files::SearchFilesTool,
    },
};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_basic_mcp_tools() {
    println!("\n=== SIMPLE MCP TOOLS TEST ===\n");
    
    // Create temp directory for testing
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    
    // Create tool system with basic config
    let config = json!({
        "enable_sandboxing": false,
        "enable_rate_limiting": false,
        "enable_caching": false,
        "max_file_size": 10485760,
    });
    
    let mut system = McpToolSystem::new(config);
    
    // Register core tools
    system.register_tool(Box::new(ReadFileTool::new()));
    system.register_tool(Box::new(WriteFileTool::new()));
    system.register_tool(Box::new(ListFilesTool::new()));
    system.register_tool(Box::new(ExecuteCommandTool::new()));
    system.register_tool(Box::new(SearchFilesTool::new()));
    
    println!("✅ Registered {} tools", system.list_tools().len());
    
    // Test 1: Write a file
    println!("\n📝 TEST 1: Write File");
    let write_args = json!({
        "path": "test.txt",
        "content": "Hello from MCP Tools!"
    });
    
    let context = ToolContext {
        workspace: workspace.clone(),
        user_id: "test_user".to_string(),
        session_id: "test_session".to_string(),
        cancellation_token: tokio_util::sync::CancellationToken::new(),
    };
    
    let write_result = system.execute_tool("writeFile", write_args, context.clone()).await;
    assert!(write_result.is_ok());
    println!("✅ File written successfully");
    
    // Test 2: Read the file back
    println!("\n📖 TEST 2: Read File");
    let read_args = json!({
        "path": "test.txt"
    });
    
    let read_result = system.execute_tool("readFile", read_args, context.clone()).await;
    assert!(read_result.is_ok());
    let result = read_result.unwrap();
    assert!(result.success);
    println!("✅ File read successfully: {:?}", result.data);
    
    // Test 3: List directory
    println!("\n📁 TEST 3: List Directory");
    let list_args = json!({
        "path": "."
    });
    
    let list_result = system.execute_tool("listFiles", list_args, context.clone()).await;
    assert!(list_result.is_ok());
    println!("✅ Directory listed successfully");
    
    // Test 4: Execute command (simple echo)
    println!("\n⚡ TEST 4: Execute Command");
    let exec_args = json!({
        "command": "echo 'MCP Tools Test'"
    });
    
    let exec_result = system.execute_tool("executeCommand", exec_args, context.clone()).await;
    assert!(exec_result.is_ok());
    println!("✅ Command executed successfully");
    
    // Test 5: Search files
    println!("\n🔍 TEST 5: Search Files");
    let search_args = json!({
        "path": ".",
        "pattern": "Hello"
    });
    
    let search_result = system.execute_tool("searchFiles", search_args, context.clone()).await;
    assert!(search_result.is_ok());
    println!("✅ Search completed successfully");
    
    // Get metrics
    let metrics = system.get_metrics();
    println!("\n📊 METRICS:");
    println!("  Total calls: {}", metrics.total_calls);
    println!("  Total errors: {}", metrics.total_errors);
    println!("  Cache hits: {}", metrics.cache_hits);
    println!("  Cache misses: {}", metrics.cache_misses);
    
    println!("\n✅ ALL TESTS PASSED!");
}

#[tokio::test]
async fn test_mcp_security() {
    println!("\n=== MCP SECURITY TEST ===\n");
    
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    
    let config = json!({
        "enable_sandboxing": true,
        "enable_rate_limiting": true,
        "enable_caching": false,
    });
    
    let mut system = McpToolSystem::new(config);
    system.register_tool(Box::new(WriteFileTool::new()));
    system.register_tool(Box::new(ExecuteCommandTool::new()));
    
    let context = ToolContext {
        workspace,
        user_id: "test_user".to_string(),
        session_id: "test_session".to_string(),
        cancellation_token: tokio_util::sync::CancellationToken::new(),
    };
    
    // Test 1: Try to write to /etc (should fail)
    println!("🔒 Testing write to /etc (should fail)");
    let write_args = json!({
        "path": "/etc/test.txt",
        "content": "should not work"
    });
    
    let result = system.execute_tool("writeFile", write_args, context.clone()).await;
    if let Ok(tool_result) = result {
        assert!(!tool_result.success, "Should not be able to write to /etc");
        println!("✅ Correctly blocked write to /etc");
    }
    
    // Test 2: Try dangerous command (should fail)
    println!("\n🔒 Testing dangerous command (should fail)");
    let exec_args = json!({
        "command": "sudo rm -rf /"
    });
    
    let result = system.execute_tool("executeCommand", exec_args, context.clone()).await;
    if let Ok(tool_result) = result {
        assert!(!tool_result.success, "Should not execute dangerous commands");
        println!("✅ Correctly blocked dangerous command");
    }
    
    println!("\n✅ SECURITY TESTS PASSED!");
}

#[tokio::test]
async fn test_mcp_performance() {
    use std::time::Instant;
    
    println!("\n=== MCP PERFORMANCE TEST ===\n");
    
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    
    let config = json!({
        "enable_sandboxing": false,
        "enable_rate_limiting": false,
        "enable_caching": true,
    });
    
    let mut system = McpToolSystem::new(config);
    system.register_tool(Box::new(ReadFileTool::new()));
    system.register_tool(Box::new(WriteFileTool::new()));
    
    let context = ToolContext {
        workspace: workspace.clone(),
        user_id: "test_user".to_string(),
        session_id: "test_session".to_string(),
        cancellation_token: tokio_util::sync::CancellationToken::new(),
    };
    
    // Create test file
    let write_args = json!({
        "path": "perf_test.txt",
        "content": "Performance test content"
    });
    let _ = system.execute_tool("writeFile", write_args, context.clone()).await;
    
    // Test dispatch timing
    println!("⏱️ Testing dispatch timing (<10ms target)");
    let read_args = json!({
        "path": "perf_test.txt"
    });
    
    let start = Instant::now();
    let _ = system.execute_tool("readFile", read_args.clone(), context.clone()).await;
    let elapsed = start.elapsed();
    
    println!("  First read: {:?}", elapsed);
    assert!(elapsed.as_millis() < 100, "First read should be < 100ms");
    
    // Test cached read (should be faster)
    let start = Instant::now();
    let _ = system.execute_tool("readFile", read_args, context.clone()).await;
    let elapsed = start.elapsed();
    
    println!("  Cached read: {:?}", elapsed);
    assert!(elapsed.as_millis() < 10, "Cached read should be < 10ms");
    
    println!("\n✅ PERFORMANCE TESTS PASSED!");
}
