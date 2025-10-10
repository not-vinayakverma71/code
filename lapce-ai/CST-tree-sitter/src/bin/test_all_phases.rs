//! Simple test to verify all phases are working

use lapce_tree_sitter::complete_pipeline::{
    CompletePipeline,
    CompletePipelineConfig,
};
use tree_sitter::Parser;
use tree_sitter_rust;
use std::path::PathBuf;
use tempfile::tempdir;

fn main() {
    println!("=== TESTING ALL OPTIMIZATION PHASES ===\n");
    
    // Test source code
    let source = r#"
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn main() {
    for i in 0..10 {
        println!("fib({}) = {}", i, fibonacci(i));
    }
}
"#;
    
    // Parse tree
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    println!("Original source: {} bytes", source.len());
    println!("Nodes in tree: {}\n", count_nodes(tree.root_node()));
    
    // Test configurations
    let configs = vec![
        ("No optimization", create_config_none()),
        ("Phase 1 (Varint+Pack+Intern)", create_config_phase1()),
        ("Phase 1+2 (+ Delta)", create_config_phase12()),
        ("Phase 1+2+3 (+ Bytecode)", create_config_phase123()),
        ("Phase 1-4a (+ Frozen)", create_config_phase14a()),
        ("Phase 1-4b (+ Mmap)", create_config_phase14b()),
        ("ALL PHASES", create_config_all()),
    ];
    
    println!("{:<30} {:>15} {:>15} {:>15}", "Configuration", "Final Size", "Compression", "Memory Used");
    println!("{:-<75}", "");
    
    for (name, config) in configs {
        let pipeline = CompletePipeline::new(config).unwrap();
        
        let result = pipeline.process_tree(
            PathBuf::from("test.rs"),
            tree.clone(),
            source.as_bytes(),
        ).unwrap();
        
        let stats = pipeline.stats();
        
        println!("{:<30} {:>15} {:>14.1}x {:>15}",
            name,
            format!("{} B", result.final_size),
            result.compression_ratio,
            format!("{} B", stats.total_memory_bytes)
        );
        
        // Show phase-specific stats
        if stats.phase1_varint_bytes > 0 {
            println!("  → Phase 1: varint={} B, interned={} symbols",
                stats.phase1_varint_bytes, stats.phase1_interned_symbols);
        }
        if stats.phase2_delta_bytes > 0 {
            println!("  → Phase 2: delta={} B, chunks={}",
                stats.phase2_delta_bytes, stats.phase2_chunks_created);
        }
        if stats.phase3_bytecode_bytes > 0 {
            println!("  → Phase 3: bytecode={} B, nodes={}",
                stats.phase3_bytecode_bytes, stats.phase3_nodes_encoded);
        }
        if stats.phase4a_frozen_entries > 0 {
            println!("  → Phase 4a: frozen={} entries, {} B on disk",
                stats.phase4a_frozen_entries, stats.phase4a_frozen_bytes);
        }
        if stats.phase4b_mmap_files > 0 {
            println!("  → Phase 4b: mmap={} files, {} B",
                stats.phase4b_mmap_files, stats.phase4b_mmap_bytes);
        }
        if stats.phase4c_segments > 0 {
            println!("  → Phase 4c: segments={}, {} B compressed",
                stats.phase4c_segments, stats.phase4c_segment_bytes);
        }
    }
    
    println!("\n✅ All phases tested successfully!");
}

fn count_nodes(node: tree_sitter::Node) -> usize {
    let mut count = 1;
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            count += count_nodes(child);
        }
    }
    count
}

fn create_config_none() -> CompletePipelineConfig {
    CompletePipelineConfig {
        memory_budget_mb: 500,
        phase1_varint: false,
        phase1_packing: false,
        phase1_interning: false,
        phase2_delta: false,
        phase2_chunking: false,
        phase3_bytecode: false,
        phase4a_frozen: false,
        phase4b_mmap: false,
        phase4c_segments: false,
        storage_dir: tempdir().unwrap().path().to_path_buf(),
    }
}

fn create_config_phase1() -> CompletePipelineConfig {
    let mut config = create_config_none();
    config.phase1_varint = true;
    config.phase1_packing = true;
    config.phase1_interning = true;
    config
}

fn create_config_phase12() -> CompletePipelineConfig {
    let mut config = create_config_phase1();
    config.phase2_delta = true;
    config.phase2_chunking = true;
    config
}

fn create_config_phase123() -> CompletePipelineConfig {
    let mut config = create_config_phase12();
    config.phase3_bytecode = true;
    config
}

fn create_config_phase14a() -> CompletePipelineConfig {
    let mut config = create_config_phase123();
    config.phase4a_frozen = true;
    config
}

fn create_config_phase14b() -> CompletePipelineConfig {
    let mut config = create_config_phase14a();
    config.phase4b_mmap = true;
    config
}

fn create_config_all() -> CompletePipelineConfig {
    CompletePipelineConfig::default()
}
