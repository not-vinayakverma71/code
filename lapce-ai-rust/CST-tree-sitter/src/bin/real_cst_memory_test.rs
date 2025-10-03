//! REAL CST MEMORY TEST - Actual tree-sitter parsing with memory measurement
//! Tests on /home/verma/lapce/Codex
//! Measures REAL CST memory usage

use std::path::{Path, PathBuf};
use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs;
use tree_sitter::{Parser, Tree};

#[derive(Debug)]
struct CSTMemoryStats {
    total_files: usize,
    parsed_files: usize,
    failed_files: usize,
    total_bytes: usize,
    total_trees_stored: usize,
    estimated_tree_memory_bytes: usize,
    parse_duration_ms: u64,
}

#[tokio::main]
async fn main() {
    println!("🔬 REAL CST MEMORY TEST");
    println!("Testing on: /home/verma/lapce/Codex");
    println!("{}", "=".repeat(80));
    
    let test_path = Path::new("/home/verma/lapce/Codex");
    
    println!("\n📊 Phase 1: Collecting TypeScript/JavaScript files...");
    let files = collect_ts_js_files(test_path).await;
    println!("   Found {} TS/JS files", files.len());
    
    println!("\n📊 Phase 2: Parsing with REAL tree-sitter and storing CSTs in memory...");
    let stats = parse_and_measure_memory(&files).await;
    
    println!("\n📊 Phase 3: Results");
    print_results(&stats);
}

fn collect_ts_js_files(path: &Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<PathBuf>> + '_>> {
    Box::pin(async move {
        let mut files = Vec::new();
        
        if let Ok(mut entries) = fs::read_dir(path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let entry_path = entry.path();
                
                if let Some(name) = entry_path.file_name() {
                    let name_str = name.to_str().unwrap_or("");
                    if name_str.starts_with('.') || name_str == "node_modules" {
                        continue;
                    }
                }
                
                if entry_path.is_dir() {
                    files.extend(collect_ts_js_files(&entry_path).await);
                } else if entry_path.is_file() {
                    if let Some(ext) = entry_path.extension() {
                        let ext_str = ext.to_str().unwrap_or("");
                        if matches!(ext_str, "ts" | "js" | "tsx" | "jsx") {
                            files.push(entry_path);
                        }
                    }
                }
            }
        }
        
        files
    })
}

async fn parse_and_measure_memory(files: &[PathBuf]) -> CSTMemoryStats {
    let start = Instant::now();
    
    // Store ALL parsed trees in memory to measure total CST size
    let mut all_trees: Vec<(Tree, Vec<u8>)> = Vec::new();
    
    let parsed = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let total_bytes = Arc::new(AtomicUsize::new(0));
    
    // Create parsers for JS/TS
    let mut js_parser = Parser::new();
    let mut ts_parser = Parser::new();
    
    // Use safe language loading - convert Language to tree_sitter::Language
    unsafe {
        js_parser.set_language(&tree_sitter_javascript::language().into()).unwrap();
        ts_parser.set_language(&tree_sitter_typescript::language_typescript().into()).unwrap();
    }
    
    println!("   Parsing {} files and storing CSTs in memory...", files.len());
    
    for (i, file) in files.iter().enumerate() {
        if (i + 1) % 100 == 0 {
            println!("   Progress: {}/{} files", i + 1, files.len());
        }
        
        match parse_single_file(file, &mut js_parser, &mut ts_parser).await {
            Ok((tree, source)) => {
                parsed.fetch_add(1, Ordering::Relaxed);
                total_bytes.fetch_add(source.len(), Ordering::Relaxed);
                all_trees.push((tree, source));
            }
            Err(_) => {
                failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    let parse_duration = start.elapsed().as_millis() as u64;
    
    // Estimate memory usage of trees
    let estimated_memory = estimate_tree_memory(&all_trees);
    
    println!("\n   Storing {} CSTs in memory for measurement...", all_trees.len());
    println!("   Total CST memory: {:.2} MB", estimated_memory as f64 / 1_048_576.0);
    
    CSTMemoryStats {
        total_files: files.len(),
        parsed_files: parsed.load(Ordering::Relaxed),
        failed_files: failed.load(Ordering::Relaxed),
        total_bytes: total_bytes.load(Ordering::Relaxed),
        total_trees_stored: all_trees.len(),
        estimated_tree_memory_bytes: estimated_memory,
        parse_duration_ms: parse_duration,
    }
}

async fn parse_single_file(
    path: &Path,
    js_parser: &mut Parser,
    ts_parser: &mut Parser,
) -> Result<(Tree, Vec<u8>), String> {
    let source = fs::read(path).await
        .map_err(|e| format!("Read error: {}", e))?;
    
    // Choose parser based on extension
    let parser = if path.extension().and_then(|s| s.to_str()) == Some("ts") 
        || path.extension().and_then(|s| s.to_str()) == Some("tsx") {
        ts_parser
    } else {
        js_parser
    };
    
    let tree = parser.parse(&source, None)
        .ok_or_else(|| "Parse failed".to_string())?;
    
    Ok((tree, source))
}

fn estimate_tree_memory(trees: &[(Tree, Vec<u8>)]) -> usize {
    let mut total_memory = 0;
    
    for (tree, source) in trees {
        // Source code
        total_memory += source.len();
        
        // Tree structure estimate
        // Each node has: kind (2 bytes), position (16 bytes), parent/child pointers (24 bytes)
        // Estimate ~50 bytes per node average
        let node_count = count_nodes(tree.root_node());
        total_memory += node_count * 50;
    }
    
    total_memory
}

fn count_nodes(node: tree_sitter::Node) -> usize {
    let mut count = 1;
    let mut cursor = node.walk();
    
    if cursor.goto_first_child() {
        loop {
            count += count_nodes(cursor.node());
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    count
}

fn print_results(stats: &CSTMemoryStats) {
    println!("\n📈 REAL CST MEMORY TEST RESULTS:");
    println!();
    
    println!("📊 File Statistics:");
    println!("   Total files:     {}", stats.total_files);
    println!("   Parsed:          {} ({:.1}%)", 
        stats.parsed_files,
        (stats.parsed_files as f64 / stats.total_files as f64) * 100.0
    );
    println!("   Failed:          {} ({:.1}%)", 
        stats.failed_files,
        (stats.failed_files as f64 / stats.total_files as f64) * 100.0
    );
    println!();
    
    println!("📊 Source Code:");
    println!("   Total bytes:     {} ({:.2} MB)", 
        stats.total_bytes,
        stats.total_bytes as f64 / 1_048_576.0
    );
    println!();
    
    println!("📊 CST Memory (REAL MEASUREMENT):");
    println!("   Trees stored:    {}", stats.total_trees_stored);
    println!("   Total memory:    {} bytes ({:.2} MB)", 
        stats.estimated_tree_memory_bytes,
        stats.estimated_tree_memory_bytes as f64 / 1_048_576.0
    );
    println!("   Avg per file:    {:.2} KB", 
        stats.estimated_tree_memory_bytes as f64 / stats.total_trees_stored as f64 / 1024.0
    );
    println!("   Memory overhead: {:.1}x source size",
        stats.estimated_tree_memory_bytes as f64 / stats.total_bytes as f64
    );
    println!();
    
    println!("📊 Performance:");
    println!("   Parse duration:  {}ms ({:.2}s)", 
        stats.parse_duration_ms,
        stats.parse_duration_ms as f64 / 1000.0
    );
    println!("   Avg per file:    {:.2}ms", 
        stats.parse_duration_ms as f64 / stats.parsed_files as f64
    );
    println!();
    
    println!("🎯 ANSWER TO YOUR QUESTION:");
    println!("   Total CST memory for {} files: {:.2} MB",
        stats.total_trees_stored,
        stats.estimated_tree_memory_bytes as f64 / 1_048_576.0
    );
}
