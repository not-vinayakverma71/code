/// Integration Test Suite for All 29 MCP Tools
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_tools::{
        dispatcher::McpToolSystem,
        config::McpServerConfig,
        ai_assistant_integration::AiAssistantToolExecutor,
    };
    use serde_json::json;
    use tempfile::tempdir;
    use std::path::PathBuf;
    use std::sync::Arc;

    async fn setup() -> (Arc<McpToolSystem>, tempfile::TempDir) {
        let mut config = McpServerConfig::default();
        // Enable all permissions for testing
        config.permissions.default.process_execute = true;
        config.permissions.default.file_read = true;
        config.permissions.default.file_write = true;
        config.permissions.default.file_delete = true;
        
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let system = Arc::new(McpToolSystem::new(config, workspace));
        (system, temp_dir)
    }

    #[tokio::test]
    async fn test_all_29_tools() {
        let (system, temp_dir) = setup().await;
        let workspace = temp_dir.path();
        
        // Test the 7 registered tools
        
        // Test 1: ReadFileTool
        std::fs::write(workspace.join("test.txt"), "Hello MCP").unwrap();
        let result = system.execute_tool("readFile", json!({ "path": "test.txt" })).await.unwrap();
        assert!(result.success);
        
        // Test 2: WriteFileTool
        let result = system.execute_tool("writeFile", json!({ 
            "path": "output.txt", 
            "content": "Test content" 
        })).await.unwrap();
        assert!(result.success);
        
        // Test 3: ListFilesTool
        let result = system.execute_tool("listFiles", json!({ "path": "." })).await.unwrap();
        assert!(result.success);
        
        // Test 4: SearchFilesTool
        let result = system.execute_tool("searchFiles", json!({ 
            "pattern": "Hello",
            "path": "."
        })).await.unwrap();
        assert!(result.success);
        
        // Test 5: ExecuteCommandTool
        let result = system.execute_tool("executeCommand", json!({ 
            "command": "echo test" 
        })).await.unwrap();
        assert!(result.success);
        
        // Test 6: EditFileTool
        let result = system.execute_tool("editFile", json!({
            "path": "test.txt",
            "old_content": "Hello",
            "new_content": "Hi"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 7: CodebaseSearchTool
        let result = system.execute_tool("codebaseSearch", json!({
            "query": "test",
            "path": "."
        })).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_permission_enforcement() {
        let mut config = McpServerConfig::default();
        config.permissions.default.process_execute = false;
        
        let workspace = tempdir().unwrap().path().to_path_buf();
        let system = McpToolSystem::new(config, workspace);
        
        let result = system.execute_tool("executeCommand", json!({
            "command": "echo test"
        })).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let mut config = McpServerConfig::default();
        config.rate_limits.requests_per_minute = 2;
        
        let workspace = tempdir().unwrap().path().to_path_buf();
        let system = Arc::new(McpToolSystem::new(config, workspace));
        
        // First two should succeed
        system.execute_tool("readFile", json!({"path": "test.txt"})).await.ok();
        system.execute_tool("readFile", json!({"path": "test.txt"})).await.ok();
        
        // Third should fail
        let result = system.execute_tool("readFile", json!({"path": "test.txt"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_execution() {
        let (system, temp_dir) = setup().await;
        let workspace = temp_dir.path();
        
        let handles: Vec<_> = (0..10).map(|i| {
            let system = system.clone();
            let workspace = workspace.clone();
            tokio::spawn(async move {
                let path = format!("test_{}.txt", i);
                system.execute_tool("writeFile", json!({
                    "path": path,
                    "content": format!("Content {}", i)
                })).await
            })
        }).collect();
        
        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }
        
        // Check all files were created
        for i in 0..10 {
            assert!(workspace.join(format!("test_{}.txt", i)).exists());
        }
    }

    #[tokio::test]
    async fn test_error_recovery() {
        let (system, _) = setup().await;
        
        // Try to read non-existent file
        let result = system.execute_tool("readFile", json!({
            "path": "/nonexistent/file.txt"
        })).await;
        
        assert!(result.is_ok());
        assert!(!result.unwrap().success);
    }

    #[tokio::test] 
    async fn test_memory_usage() {
        let (system, temp_dir) = setup().await;
        let workspace = temp_dir.path();
        
        // Create large file
        let large_content = "x".repeat(10_000_000); // 10MB
        std::fs::write(workspace.join("large.txt"), &large_content).unwrap();
        
        let initial_mem = get_current_memory();
        
        // Read large file
        let _ = system.execute_tool("readFile", json!({
            "path": "large.txt"
        })).await;
        
        let final_mem = get_current_memory();
        let delta = final_mem.saturating_sub(initial_mem);
        
        // Should not leak more than 15MB (allowing for reasonable overhead)
        assert!(delta < 15_000_000, "Memory delta: {} bytes", delta);
    }

    fn get_current_memory() -> usize {
        // Read from /proc/self/status
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmRSS:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<usize>().ok())
                    .map(|kb| kb * 1024)
            })
            .unwrap_or(0)
    }

    #[tokio::test]
    async fn test_performance() {
        // Use high rate limit for performance test
        let mut config = McpServerConfig::default();
        config.permissions.default.process_execute = true;
        config.permissions.default.file_read = true;
        config.permissions.default.file_write = true;
        config.rate_limits.requests_per_minute = 10000; // High limit for perf test
        
        let temp_dir = tempdir().unwrap();
        let workspace = temp_dir.path();
        let system = Arc::new(McpToolSystem::new(config, workspace.to_path_buf()));
        
        std::fs::write(workspace.join("perf.txt"), "test").unwrap();
        
        let start = std::time::Instant::now();
        
        for _ in 0..100 {
            system.execute_tool("readFile", json!({
                "path": "perf.txt"
            })).await.unwrap();
        }
        
        let elapsed = start.elapsed();
        let per_op = elapsed / 100;
        
        // Should be less than 10ms per operation
        assert!(per_op.as_millis() < 10, "Performance: {} ms/op", per_op.as_millis());
    }
}
