//! Test Phase 2 integration - Dual representation system

use lapce_tree_sitter::native_parser_manager_v2::{NativeParserManagerV2};
use lapce_tree_sitter::dual_representation::DualRepresentationConfig;
use std::path::Path;
use std::fs;
use std::time::Instant;

fn main() {
    println!("Testing Phase 2: Dual Representation Integration\n");
    println!("═══════════════════════════════════════════════");
    
    // Create parser manager with dual representation
    let config = DualRepresentationConfig {
        compact_threshold: 100,  // Use compact for files > 100 bytes
        compact_in_hot: true,
        auto_compact: true,
    };
    
    let manager = NativeParserManagerV2::with_config(1000, config);
    
    // Test files
    let test_files = vec![
        "/home/verma/lapce/lapce-ai/massive_test_codebase/module_9/submodule_0/module_901.py",
        "/home/verma/lapce/lapce-ai/massive_test_codebase/module_9/submodule_0/component_901.rs",
        "/home/verma/lapce/lapce-ai/massive_test_codebase/module_9/submodule_0/service_901.ts",
    ];
    
    let mut total_ts_memory = 0;
    let mut total_compact_memory = 0;
    let mut total_time_ms = 0;
    
    println!("\nParsing files:");
    for path_str in &test_files {
        let path = Path::new(path_str);
        
        if !path.exists() {
            println!("  File not found: {}", path_str);
            continue;
        }
        
        let file_size = fs::metadata(path).unwrap().len();
        
        let start = Instant::now();
        match manager.parse_file(path) {
            Ok(result) => {
                let elapsed = start.elapsed();
                let memory = result.tree.memory_bytes();
                let nodes = result.tree.node_count();
                let bytes_per_node = memory as f64 / nodes as f64;
                
                println!("\n  File: {}", path.file_name().unwrap().to_str().unwrap());
                println!("    Size: {} bytes", file_size);
                println!("    Nodes: {}", nodes);
                println!("    Representation: {}", if result.is_compact { "Compact" } else { "Standard" });
                println!("    Memory: {} bytes ({:.2} bytes/node)", memory, bytes_per_node);
                println!("    Parse time: {:.2} ms", elapsed.as_secs_f64() * 1000.0);
                
                if result.is_compact {
                    total_compact_memory += memory;
                } else {
                    total_ts_memory += memory;
                }
                total_time_ms += elapsed.as_millis() as usize;
                
                // Test node navigation
                let root = result.tree.root();
                println!("    Root: {} with {} children", root.kind(), root.child_count());
                
                // Navigate first child
                if let Some(first_child) = root.first_child() {
                    println!("    First child: {} at {}..{}", 
                             first_child.kind(), 
                             first_child.start_byte(), 
                             first_child.end_byte());
                }
            }
            Err(e) => {
                println!("  Error parsing {}: {}", path_str, e);
            }
        }
    }
    
    // Memory statistics
    let stats = manager.memory_stats();
    println!("\n═══════════════════════════════════════════════");
    println!("Memory Statistics:");
    println!("  Compact trees: {}", stats.compact_trees);
    println!("  Standard trees: {}", stats.standard_trees);
    println!("  Total memory: {} KB", stats.total_memory_bytes / 1024);
    println!("  Average per tree: {} bytes", stats.average_bytes_per_tree);
    
    println!("\nPerformance:");
    println!("  Total parse time: {} ms", total_time_ms);
    println!("  Average per file: {:.2} ms", total_time_ms as f64 / test_files.len() as f64);
    
    // Test compaction
    println!("\n═══════════════════════════════════════════════");
    println!("Testing compaction...");
    
    let before_memory = stats.total_memory_bytes;
    manager.compact_all();
    let after_stats = manager.memory_stats();
    let after_memory = after_stats.total_memory_bytes;
    
    println!("  Memory before: {} KB", before_memory / 1024);
    println!("  Memory after: {} KB", after_memory / 1024);
    println!("  Savings: {} KB ({:.1}%)", 
             (before_memory - after_memory) / 1024,
             (1.0 - after_memory as f64 / before_memory as f64) * 100.0);
    
    println!("\n✅ Phase 2 Integration Test Complete!");
}
