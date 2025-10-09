//! Test tier transitions - verify warm/cold migrations work

use lapce_tree_sitter::phase4_cache_fixed::{Phase4Cache, Phase4Config};
use std::path::PathBuf;
use std::time::Duration;
use std::thread;
use tree_sitter::Parser;

fn main() {
    println!("=== TESTING TIER TRANSITIONS ===\n");
    
    // Configure with very short timeouts for testing
    let mut config = Phase4Config::default();
    config.memory_budget_mb = 10; // Small budget to force transitions
    config.test_mode = true; // Enable test mode for short timeouts
    
    let cache = Phase4Cache::new(config).expect("Failed to create cache");
    
    // Store multiple files
    println!("1. STORING 10 FILES:");
    println!("{}", "-".repeat(40));
    
    for _i in 0..10 {
        let filename = format!("test{}.rs", i);
        let _source = format!("fn test{}() {{ println!(\"Test {}\"); }}", i, i);
        let path = PathBuf::from(&filename);
        let hash = (i as u64 + 1) * 1000;
        
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let _tree = parser.parse(&source, None).unwrap();
        
        cache.store(path, hash, tree, source.as_bytes())
            .expect("Failed to store");
        
        println!("✅ Stored: {}", filename);
    }
    
    // Check initial distribution
    let stats = cache.stats();
    println!("\n2. INITIAL TIER DISTRIBUTION:");
    println!("{}", "-".repeat(40));
    println!("Hot: {} entries", stats.hot_entries);
    println!("Warm: {} entries", stats.warm_entries);
    println!("Cold: {} entries", stats.cold_entries);
    println!("Frozen: {} entries", stats.frozen_entries);
    
    // Access pattern to trigger promotions
    println!("\n3. ACCESSING FILES TO TRIGGER PROMOTIONS:");
    println!("{}", "-".repeat(40));
    
    // Access test0.rs frequently (should stay hot)
    for _ in 0..5 {
        let path = PathBuf::from("test0.rs");
        cache.get(&path, 1000).unwrap();
        println!("  Accessed test0.rs");
    }
    
    // Access test1.rs moderately (should be warm)
    for _ in 0..2 {
        let path = PathBuf::from("test1.rs");
        cache.get(&path, 2000).unwrap();
        println!("  Accessed test1.rs");
    }
    
    // Don't access others (should demote to cold/frozen)
    
    // Force tier management
    println!("\n4. TRIGGERING TIER MANAGEMENT:");
    println!("{}", "-".repeat(40));
    cache.manage_tiers().expect("Failed to manage tiers");
    println!("✅ Tier management executed");
    
    // Sleep to allow time-based demotions
    println!("\n5. WAITING FOR TIME-BASED DEMOTIONS:");
    println!("{}", "-".repeat(40));
    println!("Waiting 20 seconds for demotions to trigger...");
    for _i in 0..20 {
        thread::sleep(Duration::from_secs(1));
        print!(".");
        if i % 10 == 9 {
            println!();
        }
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }
    println!("\n✅ Wait complete");
    
    // Force tier management again
    cache.manage_tiers().expect("Failed to manage tiers");
    
    // Check final distribution
    let stats = cache.stats();
    println!("\n6. FINAL TIER DISTRIBUTION:");
    println!("{}", "-".repeat(40));
    println!("Hot: {} entries", stats.hot_entries);
    println!("Warm: {} entries", stats.warm_entries);
    println!("Cold: {} entries", stats.cold_entries);
    println!("Frozen: {} entries", stats.frozen_entries);
    
    // Verify cache still works
    println!("\n7. VERIFYING RETRIEVAL STILL WORKS:");
    println!("{}", "-".repeat(40));
    
    let test_path = PathBuf::from("test0.rs");
    match cache.get(&test_path, 1000) {
        Ok(Some((_tree, source))) => {
            println!("✅ Retrieved test0.rs - {} bytes, tree kind: {}", 
                source.len(), tree.root_node().kind());
        },
        Ok(None) => {
            println!("❌ Failed: test0.rs not found in cache");
        },
        Err(e) => {
            println!("❌ Error retrieving test0.rs: {}", e);
        }
    }
    
    // Print cache performance
    let final_stats = cache.stats();
    println!("\n8. CACHE PERFORMANCE:");
    println!("{}", "-".repeat(40));
    println!("Total hits: {}", final_stats.cache_hits);
    println!("Total misses: {}", final_stats.cache_misses);
    println!("Hit rate: {:.1}%", 
        if final_stats.cache_hits + final_stats.cache_misses > 0 {
            (final_stats.cache_hits as f64 / (final_stats.cache_hits + final_stats.cache_misses) as f64) * 100.0
        } else {
            0.0
        }
    );
    
    // Check if tiers are working
    let tier_changes = (stats.warm_entries > 0) || (stats.cold_entries > 0) || (stats.frozen_entries > 0);
    
    if tier_changes {
        println!("\n✅ TIER TRANSITIONS ARE WORKING!");
    } else {
        println!("\n⚠️ WARNING: No tier transitions detected!");
        println!("   All entries still in hot tier");
        println!("   This indicates tier management is not working properly");
    }
}
