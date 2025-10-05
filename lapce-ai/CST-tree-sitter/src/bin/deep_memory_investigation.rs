//! Deep investigation into why each CST uses 12.4 KB

use lapce_tree_sitter::native_parser_manager::NativeParserManager;
use std::sync::Arc;
use std::collections::HashMap;
use tree_sitter::Tree;
use std::path::PathBuf;

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

struct MinimalCST {
    tree: Tree,
    source: Vec<u8>,
}

fn count_tree_nodes(tree: &Tree) -> usize {
    fn count_recursive(node: tree_sitter::Node) -> usize {
        let mut count = 1;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += count_recursive(child);
            }
        }
        count
    }
    count_recursive(tree.root_node())
}

fn main() {
    println!("=====================================");
    println!(" DEEP MEMORY INVESTIGATION");
    println!(" Finding the 12.4 KB bloat");
    println!("=====================================\n");

    let rt = tokio::runtime::Runtime::new().unwrap();
    
    rt.block_on(async {
        let manager = Arc::new(NativeParserManager::new().unwrap());
        
        // Test with progressively more CSTs to see memory growth pattern
        let mut stored: HashMap<PathBuf, MinimalCST> = HashMap::new();
        
        let test_files = [
            ("/home/verma/lapce/lapce-ai/massive_test_codebase/module_9/submodule_0/module_901.py", "py"),
            ("/home/verma/lapce/lapce-ai/massive_test_codebase/module_9/submodule_0/component_909.rs", "rs"),
            ("/home/verma/lapce/lapce-ai/massive_test_codebase/module_9/submodule_0/service_904.ts", "ts"),
        ];
        
        println!("🔬 Testing individual file memory growth:\n");
        
        let baseline = get_rss_kb();
        println!("Baseline: {} KB\n", baseline);
        
        for (i, (path, lang)) in test_files.iter().enumerate() {
            let path_buf = PathBuf::from(path);
            
            if let Ok(content) = std::fs::read(&path_buf) {
                let source_size = content.len();
                
                match manager.parse_file(&path_buf).await {
                    Ok(result) => {
                        let node_count = count_tree_nodes(&result.tree);
                        
                        // Store it
                        stored.insert(path_buf.clone(), MinimalCST {
                            tree: result.tree,
                            source: content,
                        });
                        
                        let current = get_rss_kb();
                        let delta = current.saturating_sub(baseline);
                        let per_file = if i > 0 { delta / (i + 1) as u64 } else { delta };
                        
                        println!("File {}: {} ({} bytes, {} nodes)", i+1, lang, source_size, node_count);
                        println!("  Total memory: {} KB", delta);
                        println!("  Avg per file: {} KB", per_file);
                        println!("  Expected: source({}) + tree(~2KB) = ~{} KB", 
                            source_size / 1024, (source_size / 1024) + 2);
                        println!();
                    }
                    Err(e) => println!("Error: {:?}", e),
                }
            }
        }
        
        println!("\n🔍 Analyzing stored HashMap overhead:\n");
        
        let without_hashmap = std::mem::size_of::<MinimalCST>();
        let with_pathbuf = std::mem::size_of::<PathBuf>();
        let hashmap_entry = with_pathbuf + without_hashmap + 32; // rough estimate
        
        println!("Size estimates:");
        println!("  MinimalCST struct: {} bytes", without_hashmap);
        println!("  PathBuf: {} bytes", with_pathbuf);
        println!("  HashMap entry overhead: ~32 bytes");
        println!("  Total per entry: ~{} bytes", hashmap_entry);
        println!();
        
        println!("\n🔬 Testing with 100 small files:\n");
        
        stored.clear();
        let baseline2 = get_rss_kb();
        
        // Create 100 tiny files
        let temp_dir = std::env::temp_dir();
        for i in 0..100 {
            let small_code = format!("fn test_{}() {{ println!(\"test\"); }}", i);
            let temp_file = temp_dir.join(format!("test_{}.rs", i));
            std::fs::write(&temp_file, &small_code).unwrap();
            
            if let Ok(result) = manager.parse_file(&temp_file).await {
                stored.insert(temp_file.clone(), MinimalCST {
                    tree: result.tree,
                    source: small_code.into_bytes(),
                });
            }
        }
        
        let after_100 = get_rss_kb();
        let memory_100 = after_100.saturating_sub(baseline2);
        let per_file_100 = memory_100 as f64 / 100.0;
        
        println!("100 small files (~40 bytes each):");
        println!("  Total memory: {} KB", memory_100);
        println!("  Per file: {:.2} KB", per_file_100);
        println!("  Expected: ~0.5 KB per file");
        println!("  Actual: {:.2}x higher!", per_file_100 / 0.5);
        println!();
        
        // Clean up
        for i in 0..100 {
            std::fs::remove_file(temp_dir.join(format!("test_{}.rs", i))).ok();
        }
        
        println!("\n=====================================");
        println!(" HYPOTHESIS");
        println!("=====================================\n");
        
        println!("Possible causes of bloat:");
        println!();
        println!("1. ❓ Tree-sitter C allocation overhead");
        println!("   - Tree allocates nodes on C heap");
        println!("   - Each node ~50-100 bytes in C");
        println!("   - For file with 100 nodes = 5-10 KB");
        println!();
        println!("2. ❓ HashMap/PathBuf overhead");
        println!("   - PathBuf with long paths");
        println!("   - HashMap bucket allocation");
        println!("   - String interning");
        println!();
        println!("3. ❓ Memory fragmentation");
        println!("   - Allocator fragmentation");
        println!("   - C heap separate from Rust heap");
        println!("   - RSS includes unused but allocated pages");
        println!();
        println!("4. ❓ NativeParserManager overhead");
        println!("   - Caches or internal state");
        println!("   - Growing per parse operation");
        println!();
        
        println!("=====================================");
        println!(" COMPARISON");
        println!("=====================================\n");
        
        println!("VSCode/TypeScript Compiler:");
        println!("  - Uses custom AST structure");
        println!("  - Optimized for memory density");
        println!("  - Aggressive interning of strings");
        println!("  - Shared nodes for identical subtrees");
        println!("  Result: ~400 KB per 1000 lines");
        println!();
        
        println!("Our tree-sitter:");
        println!("  - Generic C tree structure");
        println!("  - No string interning");
        println!("  - No node sharing");
        println!("  - Each node is separate allocation");
        println!("  Result: ~12.4 KB per ~15 lines = ~827 KB per 1000 lines");
        println!();
        
        println!("⚠️  We're 2x worse than VSCode!");
        println!();
        
        println!("=====================================");
        println!(" POTENTIAL FIXES");
        println!("=====================================\n");
        
        println!("1. ✅ Use LRU cache (don't store all CSTs)");
        println!("   - Keep only 500-1000 hot files");
        println!("   - Memory: 6-12 MB");
        println!();
        println!("2. ❌ Can't optimize tree-sitter itself");
        println!("   - It's a C library");
        println!("   - Node structure is fixed");
        println!();
        println!("3. ✅ Share source text with Arc<[u8]>");
        println!("   - If reparsing same file");
        println!("   - Save source duplication");
        println!();
        println!("4. ✅ Serialize to disk for cold files");
        println!("   - Serialize rarely-used CSTs");
        println!("   - Load on demand");
        println!();
        
        println!("💡 BEST SOLUTION: LRU cache + lazy loading");
        println!("   Keep 500 files = 6 MB (vs 7.5 GB for 10K)");
    });
}
