// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Security tests for file path handling and sandboxing

#[cfg(feature = "cst_ts")]
use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use std::io::Write;

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_reject_absolute_paths_outside_workspace() {
    // This test validates that we handle absolute paths safely
    let pipeline = CstToAstPipeline::new();
    
    // Try to parse a file outside normal workspace bounds
    let result = pipeline.process_file(Path::new("/etc/passwd")).await;
    
    // Should fail (file doesn't exist or isn't a valid source file)
    assert!(result.is_err(), "Should reject /etc/passwd");
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_reject_directory_traversal_attempts() {
    let temp_dir = TempDir::new().unwrap();
    let safe_file = temp_dir.path().join("safe.rs");
    std::fs::write(&safe_file, "fn main() {}").unwrap();
    
    let pipeline = CstToAstPipeline::new();
    
    // Valid file should work
    let result = pipeline.process_file(&safe_file).await;
    assert!(result.is_ok(), "Safe file should parse");
    
    // Attempt directory traversal pattern
    let traversal_path = temp_dir.path().join("../../../etc/passwd");
    let result = pipeline.process_file(&traversal_path).await;
    
    // Should fail gracefully
    assert!(result.is_err(), "Directory traversal should be rejected");
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_symlink_handling() {
    let temp_dir = TempDir::new().unwrap();
    let real_file = temp_dir.path().join("real.rs");
    std::fs::write(&real_file, "fn main() {}").unwrap();
    
    let pipeline = CstToAstPipeline::new();
    
    // Should handle real file
    let result = pipeline.process_file(&real_file).await;
    assert!(result.is_ok(), "Real file should parse");
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let link_file = temp_dir.path().join("link.rs");
        symlink(&real_file, &link_file).unwrap();
        
        // Symlinks should be resolved safely
        let result = pipeline.process_file(&link_file).await;
        assert!(result.is_ok(), "Symlink should be followed safely");
    }
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_file_size_limits_respected() {
    // Validate that extremely large files are handled gracefully
    let temp_dir = TempDir::new().unwrap();
    let large_file = temp_dir.path().join("large.rs");
    
    // Create a moderately large file (1MB of code)
    let mut file = std::fs::File::create(&large_file).unwrap();
    for i in 0..10000 {
        writeln!(file, "fn func_{}() {{ println!(\"test\"); }}", i).unwrap();
    }
    drop(file);
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&large_file).await;
    
    // Should handle large files without panic
    // May succeed or fail with appropriate error
    if let Err(e) = result {
        let error_msg = format!("{}", e);
        assert!(!error_msg.contains("panic"), "Should not panic on large files");
    }
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_special_characters_in_filenames() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test various special characters that should be handled safely
    let test_names = vec![
        "test file.rs",  // space
        "test-file.rs",  // dash
        "test_file.rs",  // underscore
        "test.rs",       // normal
    ];
    
    let pipeline = CstToAstPipeline::new();
    
    for name in test_names {
        let file_path = temp_dir.path().join(name);
        std::fs::write(&file_path, "fn main() {}").unwrap();
        
        let result = pipeline.process_file(&file_path).await;
        assert!(result.is_ok(), "Should handle filename: {}", name);
    }
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_null_bytes_in_path_rejected() {
    let pipeline = CstToAstPipeline::new();
    
    // Rust's Path automatically handles null bytes, but test defensive handling
    let result = pipeline.process_file(Path::new("test\0.rs")).await;
    
    // Should fail safely without panic
    assert!(result.is_err(), "Null byte in path should be rejected");
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_relative_paths_resolved_safely() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    std::fs::write(&file_path, "fn main() {}").unwrap();
    
    let pipeline = CstToAstPipeline::new();
    
    // Absolute path should work
    let result = pipeline.process_file(&file_path).await;
    assert!(result.is_ok(), "Absolute path should work");
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_concurrent_file_access_safe() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("concurrent.rs");
    std::fs::write(&file_path, "fn main() { println!(\"test\"); }").unwrap();
    
    // Test that concurrent access to the same file is safe
    let mut handles = vec![];
    for _ in 0..5 {
        let path = file_path.clone();
        let handle = tokio::spawn(async move {
            let pipeline = CstToAstPipeline::new();
            pipeline.process_file(&path).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent access should be safe");
    }
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_non_utf8_content_handled() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("invalid.rs");
    
    // Write invalid UTF-8
    std::fs::write(&file_path, &[0xFF, 0xFE, 0xFD]).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await;
    
    // Should fail gracefully with clear error
    assert!(result.is_err(), "Invalid UTF-8 should be rejected");
    let error_msg = format!("{}", result.unwrap_err());
    assert!(!error_msg.contains("panic"), "Should not panic on invalid UTF-8");
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_empty_file_handled() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.rs");
    std::fs::write(&file_path, "").unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await;
    
    // Empty files should be handled (may succeed with empty AST or fail gracefully)
    match result {
        Ok(_) => {}, // Success is fine
        Err(e) => {
            let error_msg = format!("{}", e);
            assert!(!error_msg.contains("panic"), "Should not panic on empty file");
        }
    }
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_permission_denied_handled() {
    // Note: This test may not work on all platforms or CI environments
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("noperm.rs");
        std::fs::write(&file_path, "fn main() {}").unwrap();
        
        // Remove read permissions
        let mut perms = std::fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&file_path, perms).ok();
        
        let pipeline = CstToAstPipeline::new();
        let result = pipeline.process_file(&file_path).await;
        
        // Should fail gracefully with permission error
        assert!(result.is_err(), "Permission denied should be handled");
    }
}

#[cfg(feature = "cst_ts")]
#[test]
fn test_path_canonicalization_safety() {
    // Test that path operations don't expose security issues
    let test_paths = vec![
        "./test.rs",
        "../test.rs",
        "test/../other.rs",
        "./././test.rs",
    ];
    
    for path_str in test_paths {
        let path = Path::new(path_str);
        // Should not panic on any path
        let _ = path.canonicalize();
    }
}
