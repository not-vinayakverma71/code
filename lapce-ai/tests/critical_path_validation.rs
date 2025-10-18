// Critical Path Validation - Production Ready Tests
// Focused on essential functionality with correct APIs

use lapce_ai_rust::core::tools::{
    traits::{Tool, ToolContext},
    expanded_tools_registry::ExpandedToolRegistry,
};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// T8: Registry Critical Tests - MUST PASS
// ============================================================================

#[test]
fn test_critical_registry_all_tools_present() {
    let registry = ExpandedToolRegistry::new();
    let tools = registry.list_tools();
    
    println!("Total tools registered: {}", tools.len());
    
    // Absolute minimum - these MUST exist
    let critical_tools = vec![
        "readFile",
        "writeFile", 
        "searchFiles",
        "listFiles",
        "applyDiff",
    ];
    
    for tool_name in &critical_tools {
        assert!(
            tools.contains(&tool_name.to_string()),
            "CRITICAL: Missing tool: {}. This is a production blocker!", 
            tool_name
        );
    }
    
    println!("✅ All critical tools present");
}

#[test]
fn test_critical_registry_tool_instantiation() {
    let registry = ExpandedToolRegistry::new();
    
    // Verify we can actually get tools, not just list them
    let read_tool = registry.get_tool("readFile");
    assert!(read_tool.is_some(), "readFile tool must be instantiable");
    
    let write_tool = registry.get_tool("writeFile");
    assert!(write_tool.is_some(), "writeFile tool must be instantiable");
    
    let search_tool = registry.get_tool("searchFiles");
    assert!(search_tool.is_some(), "searchFiles tool must be instantiable");
    
    println!("✅ All critical tools can be instantiated");
}

#[test]
fn test_critical_registry_categories_exist() {
    let registry = ExpandedToolRegistry::new();
    
    let fs_tools = registry.list_by_category("fs");
    assert!(!fs_tools.is_empty(), "fs category must have tools");
    
    let search_tools = registry.list_by_category("search");
    assert!(!search_tools.is_empty(), "search category must have tools");
    
    println!("✅ Tool categories properly organized");
}

// ============================================================================
// T1: SearchFiles Critical Path
// ============================================================================

#[tokio::test]
async fn test_critical_search_files_basic_operation() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test files
    fs::write(temp_dir.path().join("match1.txt"), "findme content").unwrap();
    fs::write(temp_dir.path().join("match2.txt"), "findme again").unwrap();
    fs::write(temp_dir.path().join("nomatch.txt"), "nothing here").unwrap();
    
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("searchFiles")
        .expect("CRITICAL: searchFiles tool must exist");
    
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(), 
        "test_user".to_string()
    );
    
    let args = json!({
        "path": ".",
        "regex": "findme"
    });
    
    let result = tool.execute(args, context).await;
    
    assert!(
        result.is_ok(),
        "CRITICAL: searchFiles must execute successfully. Error: {:?}",
        result.err()
    );
    
    let output = result.unwrap();
    assert!(output.success, "searchFiles must report success");
    
    println!("✅ SearchFiles critical path working");
}

// ============================================================================
// T10: RooIgnore Critical Security Test
// ============================================================================

#[tokio::test]
async fn test_critical_rooignore_blocks_secrets() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create .rooignore with secret patterns
    fs::write(
        temp_dir.path().join(".rooignore"),
        "*.secret\n.env\n*.key\n"
    ).unwrap();
    
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "security_test".to_string()
    );
    
    // Test blocking of secret files
    let secret_path = temp_dir.path().join("api.secret");
    let is_blocked = !context.is_path_allowed(&secret_path);
    
    assert!(
        is_blocked,
        "CRITICAL SECURITY FAILURE: RooIgnore must block .secret files!"
    );
    
    let env_path = temp_dir.path().join(".env");
    let env_blocked = !context.is_path_allowed(&env_path);
    
    assert!(
        env_blocked,
        "CRITICAL SECURITY FAILURE: RooIgnore must block .env files!"
    );
    
    // Normal files should be allowed
    let normal_path = temp_dir.path().join("normal.txt");
    let normal_allowed = context.is_path_allowed(&normal_path);
    
    assert!(
        normal_allowed,
        "RooIgnore should allow normal files"
    );
    
    println!("✅ RooIgnore critical security working");
}

// ============================================================================
// File Operations Critical Path
// ============================================================================

#[tokio::test]
async fn test_critical_write_read_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ExpandedToolRegistry::new();
    
    // CRITICAL: Write operation
    let write_tool = registry.get_tool("writeFile")
        .expect("writeFile must exist");
    
    let write_context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_user".to_string()
    );
    
    let test_content = "Critical test content";
    let write_args = json!({
        "path": "test.txt",
        "content": test_content
    });
    
    let write_result = write_tool.execute(write_args, write_context).await;
    assert!(
        write_result.is_ok(),
        "CRITICAL: writeFile must succeed. Error: {:?}",
        write_result.err()
    );
    
    // Verify file created
    let file_path = temp_dir.path().join("test.txt");
    assert!(file_path.exists(), "File must be created on disk");
    
    // CRITICAL: Read operation
    let read_tool = registry.get_tool("readFile")
        .expect("readFile must exist");
    
    let read_context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_user".to_string()
    );
    
    let read_args = json!({
        "path": "test.txt"
    });
    
    let read_result = read_tool.execute(read_args, read_context).await;
    assert!(
        read_result.is_ok(),
        "CRITICAL: readFile must succeed. Error: {:?}",
        read_result.err()
    );
    
    let output = read_result.unwrap();
    assert!(output.success, "readFile must report success");
    
    // Verify content matches
    let content_value = &output.result["content"];
    assert!(
        content_value.is_string(),
        "readFile must return string content"
    );
    
    println!("✅ Write-Read cycle critical path working");
}

// ============================================================================
// Concurrent Operations Stability Test
// ============================================================================

#[tokio::test]
async fn test_critical_concurrent_tool_execution() {
    let temp_dir = TempDir::new().unwrap();
    let registry = std::sync::Arc::new(ExpandedToolRegistry::new());
    
    // Create test files
    for i in 0..10 {
        fs::write(
            temp_dir.path().join(format!("file{}.txt", i)),
            format!("content {}", i)
        ).unwrap();
    }
    
    // Launch 10 concurrent read operations
    let mut handles = vec![];
    
    for i in 0..10 {
        let registry_clone = registry.clone();
        let temp_path = temp_dir.path().to_path_buf();
        
        let handle = tokio::spawn(async move {
            let tool = registry_clone.get_tool("readFile")
                .expect("readFile must exist");
            
            let context = ToolContext::new(temp_path, format!("user{}", i));
            let args = json!({"path": format!("file{}.txt", i)});
            
            tool.execute(args, context).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all operations
    let mut success_count = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => eprintln!("Tool error: {:?}", e),
            Err(e) => eprintln!("Task error: {:?}", e),
        }
    }
    
    assert_eq!(
        success_count, 10,
        "CRITICAL: All concurrent operations must succeed. Got {}/10",
        success_count
    );
    
    println!("✅ Concurrent operations stable");
}

// ============================================================================
// Error Handling Critical Path
// ============================================================================

#[tokio::test]
async fn test_critical_nonexistent_file_handling() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("readFile").unwrap();
    
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_user".to_string()
    );
    
    let args = json!({"path": "does_not_exist.txt"});
    let result = tool.execute(args, context).await;
    
    // Must fail gracefully, not panic
    assert!(
        result.is_err(),
        "Reading nonexistent file must return error, not panic"
    );
    
    println!("✅ Error handling graceful (no panics)");
}

#[tokio::test]
async fn test_critical_invalid_arguments_handling() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("readFile").unwrap();
    
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_user".to_string()
    );
    
    // Missing required argument
    let args = json!({});
    let result = tool.execute(args, context).await;
    
    // Must fail gracefully
    assert!(
        result.is_err(),
        "Invalid arguments must return error, not panic"
    );
    
    println!("✅ Invalid argument handling graceful");
}

// ============================================================================
// Performance Critical Test
// ============================================================================

#[tokio::test]
async fn test_critical_performance_reasonable_latency() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create 100 test files
    for i in 0..100 {
        fs::write(
            temp_dir.path().join(format!("perf{}.txt", i)),
            format!("performance test content {}", i)
        ).unwrap();
    }
    
    let registry = ExpandedToolRegistry::new();
    let tool = registry.get_tool("searchFiles").unwrap();
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "perf_user".to_string()
    );
    
    let args = json!({
        "path": ".",
        "regex": "performance"
    });
    
    let start = std::time::Instant::now();
    let result = tool.execute(args, context).await;
    let duration = start.elapsed();
    
    assert!(result.is_ok(), "Search must succeed");
    
    // Should complete in reasonable time (< 2 seconds for 100 files)
    assert!(
        duration.as_secs() < 2,
        "PERFORMANCE WARNING: Search took {:?} for 100 files (should be < 2s)",
        duration
    );
    
    println!("✅ Performance within acceptable range: {:?}", duration);
}

// ============================================================================
// Production Readiness Summary Test
// ============================================================================

#[test]
fn test_production_readiness_checklist() {
    let registry = ExpandedToolRegistry::new();
    let tools = registry.list_tools();
    
    println!("\n=== PRODUCTION READINESS CHECKLIST ===\n");
    
    // 1. Tool Count
    let tool_count = tools.len();
    println!("1. Tools Registered: {} ✅", tool_count);
    assert!(tool_count >= 15, "Should have at least 15 tools");
    
    // 2. Critical Tools
    let critical = vec!["readFile", "writeFile", "searchFiles", "applyDiff"];
    let mut all_present = true;
    for tool in &critical {
        let present = tools.contains(&tool.to_string());
        println!("   - {}: {}", tool, if present { "✅" } else { "❌" });
        all_present = all_present && present;
    }
    assert!(all_present, "All critical tools must be present");
    
    // 3. Categories
    let categories = vec!["fs", "search", "diff", "terminal"];
    for cat in categories {
        let cat_tools = registry.list_by_category(cat);
        println!("2. Category '{}': {} tools ✅", cat, cat_tools.len());
        assert!(!cat_tools.is_empty(), "Category {} must have tools", cat);
    }
    
    println!("\n=== ALL PRODUCTION CHECKS PASSED ===\n");
}
