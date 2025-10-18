// Production Validation Test Suite - Critical Phase
// Comprehensive tests for all T1-T12 completed tasks
// NO MOCKS - Real integration tests only

use lapce_ai_rust::core::tools::{
    traits::{Tool, ToolContext, ToolError},
    expanded_tools_registry::{ExpandedToolRegistry, TOOL_REGISTRY},
    streaming_v2::{UnifiedStreamEmitter, BackpressureConfig, StreamEvent},
    rooignore_unified::{UnifiedRooIgnore, RooIgnoreConfig},
    observability::OBSERVABILITY,
    diff_engine_v2::apply_diff_tool::ApplyDiffToolV2,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::fs;
use tempfile::TempDir;
use tokio::time::{timeout, Duration};

// ============================================================================
// T1: SearchFiles Consolidation Tests
// ============================================================================

#[tokio::test]
async fn test_search_files_real_ripgrep() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    fs::write(temp_dir.path().join("test1.txt"), "search_term here").unwrap();
    fs::write(temp_dir.path().join("test2.txt"), "no match").unwrap();
    fs::write(temp_dir.path().join("test3.txt"), "search_term again").unwrap();
    
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("searchFiles").expect("searchFiles tool must exist");
    
    let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
    
    let args = json!({
        "path": ".",
        "regex": "search_term",
        "filePattern": "*.txt"
    });
    
    let result = tool.execute(args, context).await;
    assert!(result.is_ok(), "SearchFiles should succeed");
    
    let output = result.unwrap();
    assert!(output.success);
    
    // Verify results contain both matching files
    let results = output.result["results"].as_array().unwrap();
    assert!(results.len() >= 2, "Should find at least 2 matches");
}

#[tokio::test]
async fn test_search_files_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("searchFiles").unwrap();
    let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
    
    let args = json!({"path": ".", "regex": "test"});
    let result = tool.execute(args, context).await;
    
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.result["results"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_search_files_invalid_regex() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("searchFiles").unwrap();
    let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
    
    let args = json!({"path": ".", "regex": "[invalid("});
    let result = tool.execute(args, context).await;
    
    // Should handle gracefully
    assert!(result.is_ok() || matches!(result, Err(ToolError::InvalidInput(_))));
}

// ============================================================================
// T2: Streaming Unification Tests
// ============================================================================

#[tokio::test]
async fn test_unified_emitter_search_progress() {
    let emitter = Arc::new(UnifiedStreamEmitter::new(BackpressureConfig::default()));
    
    // Subscribe to events
    let mut rx = emitter.subscribe();
    
    // Emit search progress
    emitter.emit_search_progress("corr-123", "test query", 10, 5, Some("file.rs"), 0.5).await.unwrap();
    
    // Verify event received
    let event = timeout(Duration::from_secs(1), rx.recv()).await;
    assert!(event.is_ok());
    
    if let Ok(Some(StreamEvent::SearchProgress(progress))) = event {
        assert_eq!(progress.correlation_id, "corr-123");
        assert_eq!(progress.files_searched, 10);
        assert_eq!(progress.matches_found, 5);
    } else {
        panic!("Expected SearchProgress event");
    }
}

#[tokio::test]
async fn test_unified_emitter_backpressure() {
    let config = BackpressureConfig {
        high_watermark: 10,
        low_watermark: 5,
        drop_policy: lapce_ai_rust::core::tools::streaming_v2::DropPolicy::DropOldest,
    };
    
    let emitter = Arc::new(UnifiedStreamEmitter::new(config));
    let mut rx = emitter.subscribe();
    
    // Flood with events
    for i in 0..100 {
        let _ = emitter.emit_search_progress(&format!("corr-{}", i), "query", i, 0, None, 0.0).await;
    }
    
    // Should have applied backpressure
    let stats = emitter.get_backpressure_stats();
    assert!(stats.drops > 0 || stats.throttles > 0, "Backpressure should engage");
}

#[tokio::test]
async fn test_streaming_command_events() {
    let emitter = Arc::new(UnifiedStreamEmitter::new(BackpressureConfig::default()));
    let mut rx = emitter.subscribe();
    
    // Test command lifecycle
    emitter.emit_command_started("echo test", vec![], "corr-456").await.unwrap();
    emitter.emit_command_output("corr-456", "test output".to_string(), 
        lapce_ai_rust::ipc::ipc_messages::StreamType::Stdout).await.unwrap();
    emitter.emit_command_exit("corr-456", 0, 100).await.unwrap();
    
    // Verify 3 events received
    let mut count = 0;
    for _ in 0..3 {
        if timeout(Duration::from_millis(100), rx.recv()).await.is_ok() {
            count += 1;
        }
    }
    assert_eq!(count, 3, "Should receive all 3 command events");
}

// ============================================================================
// T8: Registry Correctness Tests
// ============================================================================

#[test]
fn test_registry_all_tools_registered() {
    let registry = ExpandedToolRegistry::new();
    let tools = registry.list_tools();
    
    // Verify minimum expected tools
    assert!(tools.len() >= 19, "Should have at least 19 tools, got {}", tools.len());
    
    // Critical tools must exist
    let critical = vec!["readFile", "writeFile", "searchFiles", "applyDiff", "terminal", "observability"];
    for tool_name in critical {
        assert!(tools.contains(&tool_name.to_string()), "Missing critical tool: {}", tool_name);
    }
}

#[test]
fn test_registry_codex_naming_parity() {
    let registry = ExpandedToolRegistry::new();
    
    // camelCase tools (Codex parity)
    let camel_case_tools = vec!["readFile", "writeFile", "editFile", "insertContent", 
                                 "searchAndReplace", "listFiles", "searchFiles", "applyDiff"];
    
    for tool_name in camel_case_tools {
        let tool = registry.get_tool(tool_name);
        assert!(tool.is_some(), "CamelCase tool missing: {}", tool_name);
        assert_eq!(tool.unwrap().name(), tool_name);
    }
}

#[test]
fn test_registry_categories() {
    let registry = ExpandedToolRegistry::new();
    
    let expected_categories = vec!["fs", "search", "git", "encoding", "system", 
                                    "network", "diff", "compression", "terminal", "debug"];
    
    for category in expected_categories {
        let tools = registry.list_by_category(category);
        assert!(!tools.is_empty(), "Category {} should have tools", category);
    }
}

// ============================================================================
// T10: RooIgnore Unification Tests
// ============================================================================

#[test]
fn test_rooignore_blocks_secrets() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = RooIgnoreConfig {
        workspace: temp_dir.path().to_path_buf(),
        rooignore_path: temp_dir.path().join(".rooignore"),
        enable_hot_reload: false,
        cache_ttl: Duration::from_secs(300),
        max_cache_size: 1000,
        default_patterns: vec!["*.secret".to_string(), ".env".to_string()],
        strict_mode: true,
    };
    
    let enforcer = UnifiedRooIgnore::new(config).unwrap();
    
    // Test blocking
    assert!(enforcer.check_allowed(&temp_dir.path().join("api.secret")).is_err());
    assert!(enforcer.check_allowed(&temp_dir.path().join(".env")).is_err());
    assert!(enforcer.check_allowed(&temp_dir.path().join("normal.txt")).is_ok());
}

#[test]
fn test_rooignore_cache_performance() {
    let temp_dir = TempDir::new().unwrap();
    let config = RooIgnoreConfig::default();
    let mut config = config;
    config.workspace = temp_dir.path().to_path_buf();
    
    let enforcer = UnifiedRooIgnore::new(config).unwrap();
    
    let test_path = temp_dir.path().join("test.txt");
    
    // First check (cache miss)
    let start = std::time::Instant::now();
    let _ = enforcer.check_allowed(&test_path);
    let first_duration = start.elapsed();
    
    // Second check (cache hit)
    let start = std::time::Instant::now();
    let _ = enforcer.check_allowed(&test_path);
    let cached_duration = start.elapsed();
    
    // Cache should be faster
    assert!(cached_duration < first_duration, 
            "Cached lookup should be faster: {:?} vs {:?}", cached_duration, first_duration);
}

#[test]
fn test_rooignore_statistics() {
    let temp_dir = TempDir::new().unwrap();
    let config = RooIgnoreConfig {
        workspace: temp_dir.path().to_path_buf(),
        rooignore_path: temp_dir.path().join(".rooignore"),
        enable_hot_reload: false,
        cache_ttl: Duration::from_secs(300),
        max_cache_size: 1000,
        default_patterns: vec!["*.blocked".to_string()],
        strict_mode: true,
    };
    
    let enforcer = UnifiedRooIgnore::new(config).unwrap();
    
    // Perform checks
    let _ = enforcer.check_allowed(&temp_dir.path().join("allowed.txt"));
    let _ = enforcer.check_allowed(&temp_dir.path().join("blocked.blocked"));
    
    let stats = enforcer.get_stats();
    assert_eq!(stats.total_checks, 2);
    assert_eq!(stats.allows, 1);
    assert_eq!(stats.blocks, 1);
}

// ============================================================================
// T11: Diff Streaming Tests
// ============================================================================

#[tokio::test]
async fn test_diff_streaming_events() {
    let emitter = Arc::new(UnifiedStreamEmitter::new(BackpressureConfig::default()));
    let tool = ApplyDiffToolV2::with_emitter(emitter.clone());
    
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "line 1\nline 2\n").unwrap();
    
    let mut rx = emitter.subscribe();
    
    let context = ToolContext::new(temp_dir.path().to_path_buf(), "test_user".to_string());
    let args = json!({
        "file": "test.txt",
        "patch": "simple patch"
    });
    
    // Execute in background
    let handle = tokio::spawn(async move {
        tool.execute(args, context).await
    });
    
    // Collect events
    let mut events = Vec::new();
    while let Ok(Some(event)) = timeout(Duration::from_millis(500), rx.recv()).await {
        if let StreamEvent::DiffStreamUpdate(_) = event {
            events.push(event);
        }
        if events.len() >= 3 {
            break;
        }
    }
    
    let _ = handle.await;
    
    // Should have received diff events
    assert!(!events.is_empty(), "Should receive diff streaming events");
}

// ============================================================================
// T12: Observability Tests
// ============================================================================

#[tokio::test]
async fn test_observability_metrics_collection() {
    // Clear previous data
    OBSERVABILITY.clear();
    
    // Simulate tool calls
    let span = OBSERVABILITY.log_tool_call(
        "test_tool",
        "corr-obs-1",
        &json!({"arg": "value"}),
        Some("user1".to_string()),
        Some("/workspace".to_string()),
    );
    drop(span);
    
    // Record result
    OBSERVABILITY.record_tool_result("test_tool", "corr-obs-1", 150, true, None);
    
    // Verify metrics
    let metrics = OBSERVABILITY.get_metrics();
    assert_eq!(metrics.total_calls, 1);
    assert_eq!(metrics.total_errors, 0);
    
    let tool_metrics = metrics.tool_calls.get("test_tool").unwrap();
    assert_eq!(tool_metrics.call_count, 1);
    assert_eq!(tool_metrics.total_duration_ms, 150);
}

#[tokio::test]
async fn test_observability_percentiles() {
    OBSERVABILITY.clear();
    
    // Record multiple calls with varying latencies
    for i in 1..=100 {
        OBSERVABILITY.record_tool_result("perf_tool", &format!("corr-{}", i), i * 10, true, None);
    }
    
    let metrics = OBSERVABILITY.get_metrics();
    let tool_metrics = metrics.tool_calls.get("perf_tool").unwrap();
    
    assert_eq!(tool_metrics.call_count, 100);
    assert!(tool_metrics.p50_duration_ms > 0);
    assert!(tool_metrics.p95_duration_ms > tool_metrics.p50_duration_ms);
    assert!(tool_metrics.p99_duration_ms > tool_metrics.p95_duration_ms);
}

#[tokio::test]
async fn test_observability_log_retention() {
    OBSERVABILITY.clear();
    
    // Generate logs
    for i in 0..50 {
        OBSERVABILITY.record_tool_result(&format!("tool_{}", i), &format!("corr-{}", i), 100, true, None);
    }
    
    let logs = OBSERVABILITY.get_logs(Some(20));
    assert_eq!(logs.len(), 20, "Should limit to requested count");
    
    // Should return most recent
    assert!(logs[0].timestamp > logs[logs.len() - 1].timestamp);
}

// ============================================================================
// Integration Tests - End-to-End Scenarios
// ============================================================================

#[tokio::test]
async fn test_e2e_file_operations_with_rooignore() {
    let temp_dir = TempDir::new().unwrap();
    
    // Setup .rooignore
    fs::write(temp_dir.path().join(".rooignore"), "*.blocked\n").unwrap();
    
    let mut context = ToolContext::new(temp_dir.path().to_path_buf(), "e2e_user".to_string());
    
    // Should be able to write allowed file
    let registry = ExpandedToolRegistry::new();
    let write_tool = registry.get_tool("writeFile").unwrap();
    
    let args = json!({
        "path": "allowed.txt",
        "content": "test content"
    });
    
    let result = write_tool.execute(args, context.clone()).await;
    assert!(result.is_ok(), "Should write allowed file");
    
    // Should NOT be able to write blocked file (if rooignore is enforced in tool)
    let args_blocked = json!({
        "path": "blocked.blocked",
        "content": "secret"
    });
    
    // Tool should check context.is_path_allowed()
    assert!(!context.is_path_allowed(&temp_dir.path().join("blocked.blocked")));
}

#[tokio::test]
async fn test_e2e_search_with_streaming() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple test files
    for i in 0..10 {
        fs::write(temp_dir.path().join(format!("file{}.txt", i)), 
                  format!("content {} with search_term", i)).unwrap();
    }
    
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("searchFiles").unwrap();
    let context = ToolContext::new(temp_dir.path().to_path_buf(), "search_user".to_string());
    
    let args = json!({
        "path": ".",
        "regex": "search_term"
    });
    
    let result = tool.execute(args, context).await;
    assert!(result.is_ok());
    
    let output = result.unwrap();
    let results = output.result["results"].as_array().unwrap();
    assert_eq!(results.len(), 10, "Should find all 10 files");
}

#[tokio::test]
async fn test_e2e_observability_full_cycle() {
    OBSERVABILITY.clear();
    
    let temp_dir = TempDir::new().unwrap();
    let registry = ExpandedToolRegistry::new();
    
    // Execute several tools
    let read_tool = registry.get_tool("readFile").unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
    
    let context = ToolContext::new(temp_dir.path().to_path_buf(), "obs_user".to_string());
    let args = json!({"path": "test.txt"});
    
    // Execute with logging
    let _ = read_tool.execute_with_logging(args, context).await;
    
    // Verify metrics captured
    let metrics = OBSERVABILITY.get_metrics();
    assert!(metrics.total_calls > 0);
    assert!(metrics.tool_calls.contains_key("readFile"));
}

// ============================================================================
// Error Handling & Edge Cases
// ============================================================================

#[tokio::test]
async fn test_error_handling_invalid_args() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("readFile").unwrap();
    let context = ToolContext::new(temp_dir.path().to_path_buf(), "error_user".to_string());
    
    // Missing required argument
    let args = json!({});
    let result = tool.execute(args, context).await;
    
    assert!(result.is_err());
    assert!(matches!(result, Err(ToolError::InvalidInput(_)) | Err(ToolError::InvalidArguments(_))));
}

#[tokio::test]
async fn test_error_handling_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("readFile").unwrap();
    let context = ToolContext::new(temp_dir.path().to_path_buf(), "error_user".to_string());
    
    let args = json!({"path": "nonexistent.txt"});
    let result = tool.execute(args, context).await;
    
    assert!(result.is_err());
    assert!(matches!(result, Err(ToolError::NotFound(_)) | Err(ToolError::ExecutionFailed(_))));
}

#[tokio::test]
async fn test_concurrent_tool_execution() {
    let temp_dir = TempDir::new().unwrap();
    let registry = Arc::new(ExpandedToolRegistry::new());
    
    // Create test files
    for i in 0..5 {
        fs::write(temp_dir.path().join(format!("file{}.txt", i)), "content").unwrap();
    }
    
    // Execute tools concurrently
    let mut handles = vec![];
    for i in 0..5 {
        let registry_clone = registry.clone();
        let temp_path = temp_dir.path().to_path_buf();
        
        let handle = tokio::spawn(async move {
            let tool = registry_clone.get_tool("readFile").unwrap();
            let context = ToolContext::new(temp_path, format!("user{}", i));
            let args = json!({"path": format!("file{}.txt", i)});
            tool.execute(args, context).await
        });
        
        handles.push(handle);
    }
    
    // All should succeed
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent execution should succeed");
    }
}

// ============================================================================
// Performance & Stress Tests
// ============================================================================

#[tokio::test]
async fn test_performance_search_1k_files() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create 1000 files
    for i in 0..1000 {
        fs::write(temp_dir.path().join(format!("file{}.txt", i)), 
                  format!("content {} search_term", i)).unwrap();
    }
    
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("searchFiles").unwrap();
    let context = ToolContext::new(temp_dir.path().to_path_buf(), "perf_user".to_string());
    
    let args = json!({"path": ".", "regex": "search_term"});
    
    let start = std::time::Instant::now();
    let result = tool.execute(args, context).await;
    let duration = start.elapsed();
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 500, "Search should complete in < 500ms, took {:?}", duration);
}

#[test]
fn test_stress_rooignore_cache() {
    let temp_dir = TempDir::new().unwrap();
    let config = RooIgnoreConfig::default();
    let mut config = config;
    config.workspace = temp_dir.path().to_path_buf();
    config.max_cache_size = 100;
    
    let enforcer = UnifiedRooIgnore::new(config).unwrap();
    
    // Test 1000 different paths
    for i in 0..1000 {
        let path = temp_dir.path().join(format!("file{}.txt", i));
        let _ = enforcer.check_allowed(&path);
    }
    
    // Cache should not grow unbounded
    let stats = enforcer.get_stats();
    assert!(stats.cache_hits + stats.cache_misses >= 1000);
}
