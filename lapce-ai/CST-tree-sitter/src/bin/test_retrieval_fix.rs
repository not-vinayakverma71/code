//! Test that retrieval actually works now

use lapce_tree_sitter::phase4_cache_fixed::{Phase4Cache, Phase4Config};
use std::path::PathBuf;
use tree_sitter::Parser;

fn main() {
    println!("=== TESTING RETRIEVAL FIX ===\n");
    
    // Create cache with fixed implementation
    let mut config = Phase4Config::default();
    config.test_mode = true; // Enable test mode for short timeouts
    let cache = Phase4Cache::new(config).expect("Failed to create cache");
    
    // Test data
    let test_files = vec![
        ("test.rs", "fn main() { println!(\"Hello, world!\"); }"),
        ("test.py", "def hello():\n    print('Hello, world!')"),
        ("test.js", "function hello() { console.log('Hello, world!'); }"),
    ];
    
    println!("1. STORING FILES:");
    println!("{}", "-".repeat(40));
    
    // Store files
    for (i, (filename, source)) in test_files.iter().enumerate() {
        let path = PathBuf::from(filename);
        let hash = (i + 1) as u64 * 1000;
        
        // Parse based on extension
        let mut parser = Parser::new();
        let _tree = match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => {
                parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
                parser.parse(source, None).unwrap()
            },
            Some("py") => {
                parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
                parser.parse(source, None).unwrap()
            },
            Some("js") => {
                #[cfg(feature = "lang-javascript")]
                {
                    parser.set_language(&tree_sitter_javascript::language()).unwrap();
                    parser.parse(source, None).unwrap()
                }
                #[cfg(not(feature = "lang-javascript"))]
                panic!("JavaScript support not compiled in")
            },
            _ => panic!("Unknown extension"),
        };
        
        cache.store(path.clone(), hash, tree, source.as_bytes())
            .expect("Failed to store");
        
        println!("✅ Stored: {} ({} bytes)", filename, source.len());
    }
    
    println!("\n2. RETRIEVING FILES:");
    println!("{}", "-".repeat(40));
    
    // Retrieve and verify
    let mut success_count = 0;
    let mut failure_count = 0;
    
    for (i, (filename, original_source)) in test_files.iter().enumerate() {
        let path = PathBuf::from(filename);
        let hash = (i + 1) as u64 * 1000;
        
        match cache.get(&path, hash) {
            Ok(Some((_tree, source))) => {
                // Verify we got data back
                if source.len() == original_source.len() {
                    println!("✅ Retrieved: {} - Tree kind: {}", 
                        filename, tree.root_node().kind());
                    success_count += 1;
                } else {
                    println!("❌ Retrieved: {} - Source size mismatch!", filename);
                    failure_count += 1;
                }
            },
            Ok(None) => {
                println!("❌ Failed: {} - Returned None (NOT FIXED!)", filename);
                failure_count += 1;
            },
            Err(e) => {
                println!("❌ Error: {} - {}", filename, e);
                failure_count += 1;
            }
        }
    }
    
    // Print stats
    let stats = cache.stats();
    println!("\n3. CACHE STATISTICS:");
    println!("{}", "-".repeat(40));
    println!("Hot entries: {}", stats.hot_entries);
    println!("Warm entries: {}", stats.warm_entries);
    println!("Cold entries: {}", stats.cold_entries);
    println!("Frozen entries: {}", stats.frozen_entries);
    println!("Cache hits: {}", stats.cache_hits);
    println!("Cache misses: {}", stats.cache_misses);
    
    println!("\n4. FINAL RESULTS:");
    println!("{}", "-".repeat(40));
    println!("✅ Success: {}/{}", success_count, test_files.len());
    println!("❌ Failures: {}/{}", failure_count, test_files.len());
    
    if failure_count == 0 {
        println!("\n🎉 RETRIEVAL IS FIXED! ALL TESTS PASSED!");
    } else {
        println!("\n❌ RETRIEVAL STILL BROKEN! {} TESTS FAILED!", failure_count);
        std::process::exit(1);
    }
}
