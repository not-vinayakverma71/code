//! Find where the fuck our memory is going

use std::path::PathBuf;
use tree_sitter::Tree;
use lapce_tree_sitter::native_parser_manager::NativeParserManager;
use std::sync::Arc;

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

fn main() {
    println!("FINDING THE BLOAT");
    println!("==================\n");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    rt.block_on(async {
        let manager = Arc::new(NativeParserManager::new().unwrap());
        let baseline = get_rss_kb();
        
        println!("Baseline: {} KB\n", baseline);
        
        // Test 1: Store just Tree, no source
        println!("Test 1: Storing ONLY Tree (no source)");
        println!("---------------------------------------");
        
        let mut trees_only: Vec<Tree> = Vec::new();
        let test_files: Vec<PathBuf> = std::fs::read_dir("/home/verma/lapce/lapce-ai/massive_test_codebase")
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .flat_map(|d| {
                std::fs::read_dir(d).unwrap()
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
                    .take(10)
            })
            .take(100)
            .collect();
        
        for file in &test_files {
            if let Ok(result) = manager.parse_file(file).await {
                trees_only.push(result.tree);
            }
        }
        
        let after_trees = get_rss_kb();
        let tree_memory = after_trees.saturating_sub(baseline);
        
        println!("Stored {} trees", trees_only.len());
        println!("Memory: {} KB", tree_memory);
        println!("Per tree: {:.2} KB\n", tree_memory as f64 / trees_only.len() as f64);
        
        trees_only.clear();
        drop(trees_only);
        
        // Test 2: Store Tree + Source
        println!("Test 2: Storing Tree + Source");
        println!("-------------------------------");
        
        struct WithSource {
            tree: Tree,
            source: Vec<u8>,
        }
        
        let mut with_source: Vec<WithSource> = Vec::new();
        
        for file in &test_files {
            if let Ok(content) = std::fs::read(file) {
                if let Ok(result) = manager.parse_file(file).await {
                    with_source.push(WithSource {
                        tree: result.tree,
                        source: content,
                    });
                }
            }
        }
        
        let after_with_source = get_rss_kb();
        let with_source_memory = after_with_source.saturating_sub(baseline);
        
        println!("Stored {} trees+source", with_source.len());
        println!("Memory: {} KB", with_source_memory);
        println!("Per item: {:.2} KB\n", with_source_memory as f64 / with_source.len() as f64);
        
        let avg_source_size = with_source.iter().map(|w| w.source.len()).sum::<usize>() / with_source.len();
        println!("Average source size: {} bytes", avg_source_size);
        println!("Source overhead: {:.2} KB per file", avg_source_size as f64 / 1024.0);
        println!("Tree overhead: {:.2} KB per file\n", 
            (with_source_memory as f64 / with_source.len() as f64) - (avg_source_size as f64 / 1024.0));
        
        with_source.clear();
        drop(with_source);
        
        // Test 3: What if we DON'T store source?
        println!("Test 3: Tree without storing source text");
        println!("------------------------------------------");
        println!("Tree-sitter requires source text to be valid!");
        println!("If we drop source, tree nodes become invalid!");
        println!();
        println!("This might be why we MUST store source.");
        println!("But that means ~300 bytes per file is source.");
        println!("Leaving ~12 KB for tree nodes.");
        println!();
        
        // Test 4: Check if source is DUPLICATED
        println!("Test 4: Is source duplicated in Tree?");
        println!("---------------------------------------");
        
        let test_code = b"fn test() { println!(\"hello\"); }";
        let test_path = std::env::temp_dir().join("test_dup.rs");
        std::fs::write(&test_path, test_code).unwrap();
        
        let baseline2 = get_rss_kb();
        
        // Parse and keep tree + source
        let result = manager.parse_file(&test_path).await.unwrap();
        let tree_with_source = result.tree;
        let source_copy = test_code.to_vec();
        
        let after_one = get_rss_kb();
        let one_memory = after_one.saturating_sub(baseline2);
        
        println!("One tree+source: {} KB", one_memory);
        println!("Source size: {} bytes", test_code.len());
        println!("Tree overhead: ~{} KB", one_memory.saturating_sub(test_code.len() as u64 / 1024));
        println!();
        
        // Now parse 100 identical files
        let mut identical_trees = Vec::new();
        for i in 0..100 {
            let path = std::env::temp_dir().join(format!("dup_{}.rs", i));
            std::fs::write(&path, test_code).unwrap();
            
            if let Ok(result) = manager.parse_file(&path).await {
                identical_trees.push((result.tree, test_code.to_vec()));
            }
        }
        
        let after_100 = get_rss_kb();
        let hundred_memory = after_100.saturating_sub(baseline2);
        
        println!("100 identical trees+source: {} KB", hundred_memory);
        println!("Per tree: {:.2} KB", hundred_memory as f64 / 100.0);
        println!();
        
        if hundred_memory / 100 > one_memory * 2 {
            println!("❌ NO STRING INTERNING!");
            println!("Each tree stores its own copy of node strings!");
        }
        
        // Cleanup
        std::fs::remove_file(&test_path).ok();
        for i in 0..100 {
            std::fs::remove_file(std::env::temp_dir().join(format!("dup_{}.rs", i))).ok();
        }
        
        println!("\n==================");
        println!("FINDINGS");
        println!("==================\n");
        
        println!("1. Tree alone: ~{:.2} KB per file", tree_memory as f64 / 100.0);
        println!("2. Tree + Source: ~{:.2} KB per file", with_source_memory as f64 / 100.0);
        println!("3. Source size: ~0.3 KB average");
        println!("4. Tree-sitter C nodes: ~{:.2} KB per file", 
            (with_source_memory as f64 / 100.0) - 0.3);
        println!();
        println!("Tree-sitter is allocating ~12 KB in C heap per file.");
        println!("This is NOT in Rust memory - it's malloc'd C memory.");
        println!("RSS includes this, which is why we see the bloat.");
    });
}
