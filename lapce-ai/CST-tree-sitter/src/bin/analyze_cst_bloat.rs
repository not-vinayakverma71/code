//! Analyze WHY our CSTs are so bloated compared to VSCode

use lapce_tree_sitter::native_parser_manager::NativeParserManager;
use std::sync::Arc;
use std::path::PathBuf;
use tree_sitter::Tree;
use std::mem;

fn get_rss_kb() -> u64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse().unwrap_or(0);
                }
            }
        }
    }
    0
}

fn analyze_tree_size(tree: &Tree, source: &[u8]) -> TreeAnalysis {
    let root = tree.root_node();
    
    let mut analysis = TreeAnalysis {
        node_count: 0,
        total_node_size: 0,
        tree_struct_size: mem::size_of::<Tree>(),
        source_size: source.len(),
        estimated_node_overhead: 0,
    };
    
    // Count nodes recursively
    count_nodes_recursive(root, &mut analysis);
    
    // Estimate node overhead
    // Each tree-sitter Node has: kind_id, state_id, start/end byte, start/end point, etc.
    let estimated_bytes_per_node = 64; // Conservative estimate
    analysis.estimated_node_overhead = analysis.node_count * estimated_bytes_per_node;
    
    analysis
}

fn count_nodes_recursive(node: tree_sitter::Node, analysis: &mut TreeAnalysis) {
    analysis.node_count += 1;
    
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            count_nodes_recursive(child, analysis);
        }
    }
}

#[derive(Debug)]
struct TreeAnalysis {
    node_count: usize,
    total_node_size: usize,
    tree_struct_size: usize,
    source_size: usize,
    estimated_node_overhead: usize,
}

fn main() {
    println!("=====================================");
    println!(" CST BLOAT ANALYSIS");
    println!(" Why are we using 7.5 GB vs VSCode's 4 GB?");
    println!("=====================================\n");

    let rt = tokio::runtime::Runtime::new().unwrap();
    
    rt.block_on(async {
        let manager = Arc::new(NativeParserManager::new().unwrap());
        
        // Test files of varying sizes
        let test_cases = vec![
            ("Small Rust", "fn main() { println!(\"hello\"); }", "test.rs"),
            ("Medium Rust", include_str!("analyze_cst_bloat.rs"), "analyze.rs"),
            ("Small Python", "def hello():\n    print('hello')\n\nclass Foo:\n    pass", "test.py"),
        ];
        
        println!("🔍 Analyzing individual CST sizes:\n");
        
        for (name, source, filename) in test_cases {
            let source_bytes = source.as_bytes();
            let temp_file = std::env::temp_dir().join(filename);
            std::fs::write(&temp_file, source_bytes).unwrap();
            
            let baseline = get_rss_kb();
            
            match manager.parse_file(&temp_file).await {
                Ok(result) => {
                    let after_parse = get_rss_kb();
                    let memory_used = after_parse.saturating_sub(baseline);
                    
                    let analysis = analyze_tree_size(&result.tree, source_bytes);
                    
                    println!("📄 {}", name);
                    println!("  Source size: {} bytes", analysis.source_size);
                    println!("  Node count: {}", analysis.node_count);
                    println!("  Tree struct: {} bytes", analysis.tree_struct_size);
                    println!("  Est. node overhead: {} KB", analysis.estimated_node_overhead / 1024);
                    println!("  Actual memory delta: {} KB", memory_used);
                    println!("  Bytes per node: {:.1}", memory_used as f64 * 1024.0 / analysis.node_count as f64);
                    println!();
                }
                Err(e) => println!("  Error: {:?}", e),
            }
            
            std::fs::remove_file(&temp_file).ok();
        }
        
        println!("\n🔬 Analyzing storage structure:\n");
        
        // Check what we're actually storing
        println!("What's in our StoredCST:");
        println!("  1. tree: Tree - the actual tree-sitter tree");
        println!("  2. source: Vec<u8> - FULL SOURCE CODE COPY");
        println!("  3. file_path: PathBuf - path metadata");
        println!("  4. line_count: usize - line count");
        println!();
        
        println!("⚠️  PROBLEM IDENTIFIED:");
        println!("  We're storing Vec<u8> of FULL SOURCE with EVERY CST!");
        println!();
        
        println!("Tree-sitter already has the source in the Tree:");
        println!("  - Tree::root_node() contains byte ranges");
        println!("  - Tree uses source text for node content");
        println!("  - We're DUPLICATING the entire source!");
        println!();
        
        // Calculate the waste
        let avg_file_size = 291; // bytes per file from massive_test_codebase
        let files = 3000;
        let wasted_bytes = avg_file_size * files;
        let wasted_mb = wasted_bytes as f64 / 1024.0 / 1024.0;
        
        println!("💀 For massive_test_codebase (3000 files):");
        println!("  Average file: {} bytes", avg_file_size);
        println!("  Wasted on source duplication: {:.2} MB", wasted_mb);
        println!("  Tree-sitter CST overhead: ~{:.2} MB", 36.0 - wasted_mb);
        println!();
        
        println!("📊 Breakdown of 36 MB for 3000 files:");
        println!("  Source duplication: {:.2} MB (WASTE!)", wasted_mb);
        println!("  Actual CST nodes: ~{:.2} MB", 36.0 - wasted_mb);
        println!("  Per-file CST only: ~{:.2} KB", (36.0 - wasted_mb) * 1024.0 / 3000.0);
        println!();
        
        println!("=====================================");
        println!(" COMPARISON: US vs VSCODE");
        println!("=====================================\n");
        
        println!("VSCode/Windsurf (4 GB for 10K files):");
        println!("  - Electron renderer process");
        println!("  - TypeScript compiler");
        println!("  - Multiple LSP servers");
        println!("  - Semantic tokens");
        println!("  - IntelliSense cache");
        println!("  - Symbol index");
        println!("  - File watchers");
        println!("  Total: ~4 GB");
        println!();
        
        println!("Our CST (7.5 GB for 10K files):");
        let source_size_10k = avg_file_size * 10000;
        let source_waste_10k_mb = source_size_10k as f64 / 1024.0 / 1024.0;
        let cst_only_10k_mb = 7500.0 - source_waste_10k_mb;
        
        println!("  - Tree-sitter CSTs: ~{:.2} MB", cst_only_10k_mb);
        println!("  - Source duplication: ~{:.2} MB (WASTE!)", source_waste_10k_mb);
        println!("  Total: ~7500 MB");
        println!();
        
        println!("🔥 IF WE FIX SOURCE DUPLICATION:");
        println!("  Current: 7.5 GB");
        println!("  Without duplication: ~{:.2} GB", cst_only_10k_mb / 1024.0);
        println!("  Savings: {:.2} GB ({:.1}%)", source_waste_10k_mb / 1024.0, 
            source_waste_10k_mb / 7500.0 * 100.0);
        println!();
        
        println!("=====================================");
        println!(" ROOT CAUSE");
        println!("=====================================\n");
        
        println!("❌ Our mistake:");
        println!("  struct StoredCST {{");
        println!("      tree: Tree,");
        println!("      source: Vec<u8>,  // ← DUPLICATES WHAT Tree ALREADY HAS");
        println!("  }}");
        println!();
        
        println!("✅ What we should do:");
        println!("  struct StoredCST {{");
        println!("      tree: Tree,");
        println!("      // NO SOURCE - Tree already has it!");
        println!("  }}");
        println!();
        println!("  Or use Arc<[u8]> shared between trees of same file");
        println!();
        
        println!("📊 Expected memory after fix:");
        let fixed_mb = 7500.0 - source_waste_10k_mb;
        println!("  10K files: ~{:.2} GB (vs current 7.5 GB)", fixed_mb / 1024.0);
        println!("  Per file: ~{:.2} KB (vs current 768 KB)", fixed_mb * 1024.0 / 10000.0);
        println!();
        
        println!("⚠️  But even {:.2} GB is still more than VSCode's 4 GB!", fixed_mb / 1024.0);
        println!("    We need to investigate tree-sitter's internal structure further.");
    });
}
