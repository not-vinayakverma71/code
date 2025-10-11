//! Tests for stable ID propagation through CST → AST pipeline (Phase B)

#![cfg(feature = "cst_ts")]

use super::*;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use std::io::Write;

#[tokio::test]
async fn test_stable_ids_present_in_cst() {
    let pipeline = CstToAstPipeline::new();
    
    // Create temporary Rust file
    let mut temp_file = NamedTempFile::new().unwrap();
    let source = "fn test() { let x = 42; }";
    temp_file.write_all(source.as_bytes()).unwrap();
    let path = temp_file.path().with_extension("rs");
    std::fs::copy(temp_file.path(), &path).unwrap();
    
    // Process with CstApi
    let output = pipeline.process_file(&path).await.unwrap();
    
    // Verify stable IDs are present in CST
    assert!(has_stable_ids(&output.cst), "CST should have stable IDs");
    
    // Cleanup
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_stable_ids_propagate_to_ast() {
    let pipeline = CstToAstPipeline::new();
    
    let mut temp_file = NamedTempFile::new().unwrap();
    let source = "fn test() { let x = 42; }";
    temp_file.write_all(source.as_bytes()).unwrap();
    let path = temp_file.path().with_extension("rs");
    std::fs::copy(temp_file.path(), &path).unwrap();
    
    let output = pipeline.process_file(&path).await.unwrap();
    
    // Verify stable IDs propagate to AST metadata
    assert!(has_ast_stable_ids(&output.ast), "AST should have stable IDs in metadata");
    
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_stable_ids_unique() {
    let pipeline = CstToAstPipeline::new();
    
    let mut temp_file = NamedTempFile::new().unwrap();
    let source = "fn main() {\n    let x = 1;\n    let y = 2;\n}";
    temp_file.write_all(source.as_bytes()).unwrap();
    let path = temp_file.path().with_extension("rs");
    std::fs::copy(temp_file.path(), &path).unwrap();
    
    let output = pipeline.process_file(&path).await.unwrap();
    
    // Collect all stable IDs
    let mut ids = collect_stable_ids(&output.cst);
    ids.sort();
    ids.dedup();
    
    let total_nodes = count_nodes(&output.cst);
    
    // All stable IDs should be unique
    assert_eq!(ids.len(), total_nodes, "All nodes should have unique stable IDs");
    
    std::fs::remove_file(&path).ok();
}

#[tokio::test]
async fn test_fallback_to_regular_parsing() {
    let pipeline = CstToAstPipeline::new();
    
    // Use a file that works with regular parsing
    let mut temp_file = NamedTempFile::new().unwrap();
    let source = "fn test() {}";
    temp_file.write_all(source.as_bytes()).unwrap();
    let path = temp_file.path().with_extension("rs");
    std::fs::copy(temp_file.path(), &path).unwrap();
    
    // Even if CstApi fails, regular parsing should work
    let result = pipeline.process_file(&path).await;
    
    assert!(result.is_ok(), "Should fallback to regular parsing if CstApi fails");
    
    std::fs::remove_file(&path).ok();
}

// Helper functions

fn has_stable_ids(cst: &CstNode) -> bool {
    if cst.stable_id.is_none() {
        return false;
    }
    
    for child in &cst.children {
        if !has_stable_ids(child) {
            return false;
        }
    }
    
    true
}

fn has_ast_stable_ids(ast: &AstNode) -> bool {
    if ast.metadata.stable_id.is_none() {
        return false;
    }
    
    for child in &ast.children {
        if !has_ast_stable_ids(child) {
            return false;
        }
    }
    
    true
}

fn collect_stable_ids(cst: &CstNode) -> Vec<u64> {
    let mut ids = Vec::new();
    
    if let Some(id) = cst.stable_id {
        ids.push(id);
    }
    
    for child in &cst.children {
        ids.extend(collect_stable_ids(child));
    }
    
    ids
}

fn count_nodes(cst: &CstNode) -> usize {
    let mut count = 1;
    
    for child in &cst.children {
        count += count_nodes(child);
    }
    
    count
}
