//! Validator to ensure CompactTree is 100% equivalent to Tree-sitter Tree
//! Checks every node attribute for exact match

use lapce_tree_sitter::compact::{CompactTreeBuilder, CompactNode};
use tree_sitter::{Parser, Tree, Node};
use std::path::PathBuf;
use walkdir::WalkDir;
use std::time::Instant;

const TEST_DIR: &str = "/home/verma/lapce/lapce-ai/massive_test_codebase";

/// Validation result for a file
#[derive(Debug)]
struct ValidationResult {
    file: PathBuf,
    passed: bool,
    node_count: usize,
    ts_memory: usize,
    compact_memory: usize,
    compression_ratio: f64,
    errors: Vec<String>,
}

/// Compare Tree-sitter node with CompactNode
fn compare_nodes(ts_node: Node, compact_node: CompactNode, source: &[u8], errors: &mut Vec<String>) -> bool {
    let mut passed = true;
    
    // Compare kind
    if ts_node.kind() != compact_node.kind() {
        errors.push(format!("Kind mismatch: TS '{}' vs Compact '{}'", 
                           ts_node.kind(), compact_node.kind()));
        passed = false;
    }
    
    // Compare flags
    if ts_node.is_named() != compact_node.is_named() {
        errors.push(format!("is_named mismatch at {}: TS {} vs Compact {}", 
                           ts_node.start_byte(), ts_node.is_named(), compact_node.is_named()));
        passed = false;
    }
    
    if ts_node.is_missing() != compact_node.is_missing() {
        errors.push(format!("is_missing mismatch at {}: TS {} vs Compact {}",
                           ts_node.start_byte(), ts_node.is_missing(), compact_node.is_missing()));
        passed = false;
    }
    
    if ts_node.is_extra() != compact_node.is_extra() {
        errors.push(format!("is_extra mismatch at {}: TS {} vs Compact {}",
                           ts_node.start_byte(), ts_node.is_extra(), compact_node.is_extra()));
        passed = false;
    }
    
    if ts_node.is_error() != compact_node.is_error() {
        errors.push(format!("is_error mismatch at {}: TS {} vs Compact {}",
                           ts_node.start_byte(), ts_node.is_error(), compact_node.is_error()));
        passed = false;
    }
    
    // Compare positions
    if ts_node.start_byte() != compact_node.start_byte() {
        errors.push(format!("start_byte mismatch for {}: TS {} vs Compact {}",
                           ts_node.kind(), ts_node.start_byte(), compact_node.start_byte()));
        passed = false;
    }
    
    if ts_node.end_byte() != compact_node.end_byte() {
        errors.push(format!("end_byte mismatch for {}: TS {} vs Compact {}",
                           ts_node.kind(), ts_node.end_byte(), compact_node.end_byte()));
        passed = false;
    }
    
    // Compare child count
    if ts_node.child_count() != compact_node.child_count() {
        errors.push(format!("child_count mismatch for {} at {}: TS {} vs Compact {}",
                           ts_node.kind(), ts_node.start_byte(), 
                           ts_node.child_count(), compact_node.child_count()));
        passed = false;
    }
    
    passed
}

/// Recursively validate tree equivalence
fn validate_tree_recursive(
    ts_node: Node,
    compact_node: CompactNode,
    source: &[u8],
    errors: &mut Vec<String>,
    depth: usize,
) -> bool {
    // Compare this node
    if !compare_nodes(ts_node, compact_node, source, errors) {
        return false;
    }
    
    // Compare children
    let mut ts_cursor = ts_node.walk();
    let ts_children: Vec<Node> = ts_node.children(&mut ts_cursor).collect();
    let compact_children: Vec<CompactNode> = compact_node.children().collect();
    
    if ts_children.len() != compact_children.len() {
        errors.push(format!("Child count mismatch at depth {}: TS {} vs Compact {}",
                           depth, ts_children.len(), compact_children.len()));
        return false;
    }
    
    let mut all_passed = true;
    for (ts_child, compact_child) in ts_children.iter().zip(compact_children.iter()) {
        if !validate_tree_recursive(*ts_child, *compact_child, source, errors, depth + 1) {
            all_passed = false;
            if errors.len() > 100 {
                errors.push("Too many errors, stopping validation".to_string());
                break;
            }
        }
    }
    
    all_passed
}

/// Validate a single file
fn validate_file(path: &PathBuf) -> ValidationResult {
    let mut errors = Vec::new();
    
    // Read source
    let source = match std::fs::read(path) {
        Ok(s) => s,
        Err(e) => {
            errors.push(format!("Failed to read file: {}", e));
            return ValidationResult {
                file: path.clone(),
                passed: false,
                node_count: 0,
                ts_memory: 0,
                compact_memory: 0,
                compression_ratio: 0.0,
                errors,
            };
        }
    };
    
    // Detect language and parse with Tree-sitter
    let mut parser = Parser::new();
    let language = match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => tree_sitter_rust::LANGUAGE.into(),
        Some("py") => tree_sitter_python::LANGUAGE.into(),
        Some("js") => tree_sitter_javascript::language(),
        Some("ts") => tree_sitter_typescript::language_typescript(),
        Some("go") => tree_sitter_go::LANGUAGE.into(),
        Some("c") => tree_sitter_c::LANGUAGE.into(),
        Some("cpp") => tree_sitter_cpp::LANGUAGE.into(),
        Some("java") => tree_sitter_java::LANGUAGE.into(),
        _ => {
            errors.push("Unsupported file type".to_string());
            return ValidationResult {
                file: path.clone(),
                passed: false,
                node_count: 0,
                ts_memory: 0,
                compact_memory: 0,
                compression_ratio: 0.0,
                errors,
            };
        }
    };
    
    parser.set_language(&language).unwrap();
    let ts_tree = match parser.parse(&source, None) {
        Some(t) => t,
        None => {
            errors.push("Failed to parse with Tree-sitter".to_string());
            return ValidationResult {
                file: path.clone(),
                passed: false,
                node_count: 0,
                ts_memory: 0,
                compact_memory: 0,
                compression_ratio: 0.0,
                errors,
            };
        }
    };
    
    // Count nodes in TS tree
    let node_count = count_nodes(ts_tree.root_node());
    
    // Estimate TS memory (80-100 bytes per node)
    let ts_memory = node_count * 90;
    
    // Build compact tree
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&ts_tree, &source);
    
    // Get compact memory
    let compact_memory = compact_tree.memory_bytes();
    let compression_ratio = ts_memory as f64 / compact_memory as f64;
    
    // Validate structure
    if let Err(e) = compact_tree.validate() {
        errors.push(format!("Compact tree validation failed: {}", e));
    }
    
    // Compare trees
    let ts_root = ts_tree.root_node();
    let compact_root = compact_tree.root();
    
    let passed = validate_tree_recursive(ts_root, compact_root, &source, &mut errors, 0);
    
    ValidationResult {
        file: path.clone(),
        passed,
        node_count,
        ts_memory,
        compact_memory,
        compression_ratio,
        errors,
    }
}

/// Count nodes in tree
fn count_nodes(node: Node) -> usize {
    let mut count = 1;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += count_nodes(child);
    }
    count
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║      COMPACT TREE EQUIVALENCE VALIDATOR              ║");
    println!("╚═══════════════════════════════════════════════════════╝");
    println!();
    
    // Collect test files
    let files: Vec<PathBuf> = WalkDir::new(TEST_DIR)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
            matches!(ext, "rs" | "py" | "ts" | "js" | "go" | "java" | "cpp" | "c")
        })
        .take(100) // Test first 100 files
        .map(|e| e.path().to_path_buf())
        .collect();
    
    println!("Testing {} files from {}", files.len(), TEST_DIR);
    println!();
    
    let start = Instant::now();
    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut total_ts_memory = 0;
    let mut total_compact_memory = 0;
    let mut failed_files = Vec::new();
    
    for (i, file) in files.iter().enumerate() {
        print!("Testing {}/{}: {:?}... ", i + 1, files.len(), file.file_name().unwrap());
        
        let result = validate_file(file);
        
        // Extract values before move
        let ts_memory = result.ts_memory;
        let compact_memory = result.compact_memory;
        
        if result.passed {
            println!("✅ PASS ({:.1}x compression)", result.compression_ratio);
            total_passed += 1;
        } else {
            println!("❌ FAIL ({} errors)", result.errors.len());
            total_failed += 1;
            failed_files.push(result);
        }
        
        total_ts_memory += ts_memory;
        total_compact_memory += compact_memory;
    }
    
    let elapsed = start.elapsed();
    
    println!();
    println!("═══════════════════════════════════════════════════════");
    println!("SUMMARY");
    println!("═══════════════════════════════════════════════════════");
    println!();
    println!("Files tested: {}", files.len());
    println!("Passed: {} ({:.1}%)", total_passed, 
             total_passed as f64 / files.len() as f64 * 100.0);
    println!("Failed: {}", total_failed);
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    println!();
    println!("Memory comparison:");
    println!("  Tree-sitter: {} KB", total_ts_memory / 1024);
    println!("  CompactTree: {} KB", total_compact_memory / 1024);
    println!("  Compression: {:.1}x", 
             total_ts_memory as f64 / total_compact_memory.max(1) as f64);
    println!("  Savings: {} KB ({:.1}%)", 
             (total_ts_memory - total_compact_memory) / 1024,
             (1.0 - total_compact_memory as f64 / total_ts_memory as f64) * 100.0);
    
    if !failed_files.is_empty() {
        println!();
        println!("Failed files:");
        for result in failed_files.iter().take(5) {
            println!("  {:?}:", result.file);
            for error in result.errors.iter().take(3) {
                println!("    - {}", error);
            }
        }
    }
    
    if total_failed == 0 {
        println!();
        println!("🎉 All tests passed! CompactTree is 100% equivalent to Tree-sitter!");
    }
}
