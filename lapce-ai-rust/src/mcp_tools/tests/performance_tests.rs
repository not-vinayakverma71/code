use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use anyhow::Result;
use serde_json::json;

use crate::mcp_tools::{
    core::{ToolContext, Tool},
    tool_registry::ToolRegistry,
    mcp_system::McpToolSystem,
    tools::browser_action::BrowserActionTool,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    // Memory tracking allocator
    struct TrackingAllocator;
    
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    
    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ret = System.alloc(layout);
            if !ret.is_null() {
                ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            }
            ret
        }
        
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout);
            ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        }
    }
    
    #[global_allocator]
    static GLOBAL: TrackingAllocator = TrackingAllocator;
    
    fn get_memory_usage() -> usize {
        ALLOCATED.load(Ordering::Relaxed)
    }
    
    #[tokio::test]
    async fn test_memory_footprint_under_3mb() {
        let initial_mem = get_memory_usage();
        
        // Initialize tool registry
        let registry = Arc::new(ToolRegistry::new());
        
        // Register all 29 tools
        registry.register(Arc::new(ReadFileTool::new()));
        registry.register(Arc::new(WriteFileTool::new()));
        registry.register(Arc::new(EditFileTool::new()));
        registry.register(Arc::new(InsertContentTool::new()));
        registry.register(Arc::new(SearchAndReplaceTool::new()));
        registry.register(Arc::new(CodebaseSearchTool::new()));
        registry.register(Arc::new(ListCodeDefinitionsTool::new()));
        registry.register(Arc::new(MultiApplyDiffTool::new()));
        registry.register(Arc::new(TerminalTool::new()));
        registry.register(Arc::new(GitTool::new()));
        registry.register(Arc::new(FileSystemTool::new(PathBuf::from("/tmp"))));
        registry.register(Arc::new(SimpleReadFileTool::new()));
        registry.register(Arc::new(FetchInstructionsTool::new()));
        registry.register(Arc::new(AccessMcpResourceTool::new()));
        registry.register(Arc::new(UseMcpToolTool::new()));
        registry.register(Arc::new(SearchFilesTool::new()));
        registry.register(Arc::new(NewRuleTool::new()));
        registry.register(Arc::new(BrowserActionTool::new()));
        registry.register(Arc::new(NewTaskTool::new()));
        // Add remaining tools...
        
        // Create test context
        let context = ToolContext {
            workspace: PathBuf::from("/tmp/test"),
            user: "test_user".to_string(),
            session: "test_session".to_string(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        };
        
        // Execute 100 operations
        for i in 0..100 {
            let tool_name = format!("readFile");
            let args = json!({
                "path": format!("test_{}.txt", i)
            });
            
            // Dispatch tool (will fail but allocates memory)
            let _ = registry.dispatch(&tool_name, args, context.clone()).await;
        }
        
        let final_mem = get_memory_usage();
        let memory_used = final_mem - initial_mem;
        let memory_mb = memory_used as f64 / (1024.0 * 1024.0);
        
        println!("Memory used: {:.2} MB", memory_mb);
        assert!(memory_mb < 3.0, "Memory usage {:.2} MB exceeds 3MB limit", memory_mb);
    }
    
    #[tokio::test]
    async fn test_tool_dispatch_under_10ms() {
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Arc::new(SimpleReadFileTool::new()));
        
        let context = ToolContext {
            workspace: PathBuf::from("/tmp/test"),
            user: "test_user".to_string(),
            session: "test_session".to_string(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        };
        
        // Warm up
        for _ in 0..10 {
            let _ = registry.dispatch(
                "simpleReadFileTool",
                json!({"path": "test.txt"}),
                context.clone()
            ).await;
        }
        
        // Measure dispatch time
        let mut total_duration = Duration::ZERO;
        let iterations = 100;
        
        for _ in 0..iterations {
            let start = Instant::now();
            
            let _ = registry.dispatch(
                "simpleReadFileTool",
                json!({"path": "test.txt"}),
                context.clone()
            ).await;
            
            total_duration += start.elapsed();
        }
        
        let avg_duration = total_duration / iterations as u32;
        let avg_ms = avg_duration.as_millis();
        
        println!("Average dispatch time: {}ms", avg_ms);
        assert!(avg_ms < 10, "Dispatch time {}ms exceeds 10ms limit", avg_ms);
    }
    
    #[tokio::test]
    async fn test_10k_concurrent_operations() {
        let registry = Arc::new(ToolRegistry::new());
        
        // Register multiple tools
        registry.register(Arc::new(SimpleReadFileTool::new()));
        registry.register(Arc::new(WriteFileTool::new()));
        registry.register(Arc::new(SearchFilesTool::new()));
        
        let context = ToolContext {
            workspace: PathBuf::from("/tmp/test"),
            user: "test_user".to_string(),
            session: "test_session".to_string(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        };
        
        let start = Instant::now();
        let mut handles = Vec::new();
        
        // Spawn 10,000 concurrent operations
        for i in 0..10_000 {
            let registry = registry.clone();
            let context = context.clone();
            let tool_name = match i % 3 {
                0 => "simpleReadFileTool",
                1 => "writeFile",
                _ => "searchFiles",
            };
            
            let handle = tokio::spawn(async move {
                let args = json!({
                    "path": format!("test_{}.txt", i),
                    "content": "test content"
                });
                
                registry.dispatch(tool_name, args, context).await
            });
            
            handles.push(handle);
        }
        
        // Wait for all to complete
        let mut successful = 0;
        let mut failed = 0;
        
        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => successful += 1,
                _ => failed += 1,
            }
        }
        
        let duration = start.elapsed();
        
        println!("10K operations completed in {:?}", duration);
        println!("Successful: {}, Failed: {}", successful, failed);
        
        assert!(duration < Duration::from_secs(30), 
            "10K operations took {:?}, exceeding 30s limit", duration);
    }
    
    #[tokio::test]
    async fn test_process_sandbox_isolation() {
        use crate::mcp_tools::sandbox_real::ProcessSandbox;
        
        let sandbox = ProcessSandbox::new(PathBuf::from("/tmp/sandbox"));
        
        // Try to access system file (should fail)
        let result = sandbox.execute_sandboxed(
            "cat /etc/passwd",
            crate::mcp_tools::sandbox::SandboxConfig {
                working_dir: PathBuf::from("/tmp"),
                env_vars: Default::default(),
                timeout: Duration::from_secs(5),
                memory_limit: 50 * 1024 * 1024,
                cpu_limit: Duration::from_secs(1),
                enable_network: false,
            }
        ).await;
        
        assert!(result.is_err() || result.unwrap().exit_code != 0,
            "Sandbox should prevent access to /etc/passwd");
    }
    
    #[tokio::test]
    async fn test_rate_limiting() {
        use crate::mcp_tools::rate_limiter::GovernorRateLimiter;
        
        let limiter = GovernorRateLimiter::new();
        
        // Should allow first 10 requests
        for i in 0..10 {
            let result = limiter.check_rate_limit("user1", "readFile").await;
            assert!(result.is_ok(), "Request {} should be allowed", i);
        }
        
        // Track time for rate limit to reset
        let start = Instant::now();
        
        // Keep trying until allowed again (should be within 1 minute)
        loop {
            if limiter.check_rate_limit("user1", "readFile").await.is_ok() {
                break;
            }
            
            if start.elapsed() > Duration::from_secs(65) {
                panic!("Rate limit did not reset within expected time");
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        use crate::mcp_tools::circuit_breaker::CircuitBreaker;
        
        let breaker = CircuitBreaker::new();
        let mut failure_count = 0;
        
        // Cause 5 failures to open circuit
        for _ in 0..5 {
            let result = breaker.call("failing_service", || {
                Err(anyhow::anyhow!("Service error"))
            }).await;
            
            if result.is_err() {
                failure_count += 1;
            }
        }
        
        assert_eq!(failure_count, 5, "Should have 5 failures");
        
        // Circuit should now be open
        let result = breaker.call("failing_service", || {
            Ok("success")
        }).await;
        
        assert!(result.is_err() && result.unwrap_err().to_string().contains("Circuit breaker"),
            "Circuit should be open after 5 failures");
        
        // Wait for timeout
        tokio::time::sleep(Duration::from_secs(61)).await;
        
        // Should now enter half-open and allow one attempt
        let result = breaker.call("failing_service", || {
            Ok("success")
        }).await;
        
        assert!(result.is_ok(), "Circuit should allow attempt after timeout");
    }
}
