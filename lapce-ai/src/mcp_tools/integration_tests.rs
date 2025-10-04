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

    async fn setup() -> (McpToolSystem, PathBuf) {
        let config = McpServerConfig::default();
        let workspace = tempdir().unwrap().path().to_path_buf();
        let system = McpToolSystem::new(config, workspace.clone());
        (system, workspace)
    }

    #[tokio::test]
    async fn test_all_29_tools() {
        let (system, workspace) = setup().await;
        
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
        
        // Test 7: ApplyDiffTool
        let diff = "--- a/test.txt\n+++ b/test.txt\n@@ -1 +1 @@\n-Hi MCP\n+Hello MCP";
        let result = system.execute_tool("applyDiff", json!({
            "path": "test.txt",
            "diff": diff
        })).await.unwrap();
        assert!(result.success);
        
        // Test 8: InsertContentTool
        let result = system.execute_tool("insertContent", json!({
            "path": "test.txt",
            "content": "Inserted",
            "position": "start"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 9: CodebaseSearchTool
        let result = system.execute_tool("codebaseSearch", json!({
            "query": "test",
            "path": "."
        })).await.unwrap();
        assert!(result.success);
        
        // Test 10: ListCodeDefinitionsTool
        std::fs::write(workspace.join("code.rs"), "fn main() {}").unwrap();
        let result = system.execute_tool("listCodeDefinitions", json!({
            "path": "code.rs"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 11: SearchAndReplaceTool
        let result = system.execute_tool("searchAndReplace", json!({
            "search": "Hello",
            "replace": "Hi",
            "path": ".",
            "dry_run": true
        })).await.unwrap();
        assert!(result.success);
        
        // Test 12: NewTaskTool
        let result = system.execute_tool("newTask", json!({
            "description": "Test task",
            "priority": "high"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 13: UpdateTodoListTool
        let result = system.execute_tool("updateTodoList", json!({
            "todos": [
                {"id": "1", "task": "Test", "status": "pending"}
            ]
        })).await.unwrap();
        assert!(result.success);
        
        // Test 14: AttemptCompletionTool
        let result = system.execute_tool("attemptCompletion", json!({
            "result": "Task completed"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 15: AskFollowupQuestionTool
        let result = system.execute_tool("askFollowupQuestion", json!({
            "question": "Need more info?"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 16: FetchInstructionsTool
        let result = system.execute_tool("fetchInstructions", json!({})).await.unwrap();
        assert!(result.success);
        
        // Test 17: CondenseTool
        let result = system.execute_tool("condense", json!({
            "text": "This is a very long text that needs to be condensed",
            "max_length": 20
        })).await.unwrap();
        assert!(result.success);
        
        // Test 18: BrowserActionTool
        let result = system.execute_tool("browserAction", json!({
            "action": "navigate",
            "url": "https://example.com"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 19: SwitchModeTool
        let result = system.execute_tool("switchMode", json!({
            "mode": "edit"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 20: AccessMcpResourceTool
        let result = system.execute_tool("accessMcpResource", json!({
            "resource": "test"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 21: UseMcpToolTool
        let result = system.execute_tool("useMcpTool", json!({
            "tool": "readFile",
            "args": {"path": "test.txt"}
        })).await.unwrap();
        assert!(result.success);
        
        // Test 22: NewRuleTool
        let result = system.execute_tool("newRule", json!({
            "rule": "Always test"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 23: ReportBugTool
        let result = system.execute_tool("reportBug", json!({
            "description": "Test bug"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 24: FileWatcherTool
        let result = system.execute_tool("fileWatcher", json!({
            "path": ".",
            "pattern": "*.txt"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 25: MultiApplyDiffTool
        let result = system.execute_tool("multiApplyDiff", json!({
            "diffs": [
                {"path": "test.txt", "diff": diff}
            ]
        })).await.unwrap();
        assert!(result.success);
        
        // Test 26: SimpleReadFileTool
        let result = system.execute_tool("simpleReadFile", json!({
            "path": "test.txt"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 27: TerminalTool
        let result = system.execute_tool("terminal", json!({
            "command": "ls"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 28: GitTool
        let result = system.execute_tool("git", json!({
            "command": "status"
        })).await.unwrap();
        assert!(result.success);
        
        // Test 29: BackupTool
        let result = system.execute_tool("backup", json!({
            "path": "test.txt"
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
        let system = McpToolSystem::new(config, workspace);
        
        // First two should succeed
        system.execute_tool("readFile", json!({"path": "test.txt"})).await.ok();
        system.execute_tool("readFile", json!({"path": "test.txt"})).await.ok();
        
        // Third should fail
        let result = system.execute_tool("readFile", json!({"path": "test.txt"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_execution() {
        let (system, workspace) = setup().await;
        
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
        let (system, workspace) = setup().await;
        
        // Create large file
        let large_content = "x".repeat(10_000_000); // 10MB
        std::fs::write(workspace.join("large.txt"), &large_content).unwrap();
        
        let initial_mem = get_current_memory();
        
        // Read large file
        let _ = system.execute_tool("readFile", json!({
            "path": "large.txt"
        })).await;
        
        let final_mem = get_current_memory();
        let delta = final_mem - initial_mem;
        
        // Should not leak more than 3MB
        assert!(delta < 3_000_000);
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
        let (system, workspace) = setup().await;
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
        assert!(per_op.as_millis() < 10);
    }
}
