//! Test tier transitions with debug output

use lapce_tree_sitter::phase4_cache_fixed::{Phase4Cache, Phase4Config};
use lapce_tree_sitter::{MultiTierCache, MultiTierConfig};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::thread;
use tree_sitter::Parser;

fn main() {
    println!("=== TESTING TIER TRANSITIONS WITH DEBUG ===\n");
    
    // Create multi-tier cache directly with very short timeouts
    let config = MultiTierConfig {
        hot_tier_mb: 2,  // Very small to force transitions
        warm_tier_mb: 2,
        cold_tier_mb: 2,
        promote_to_hot_threshold: 5,
        promote_to_warm_threshold: 2,
        demote_to_warm_timeout: Duration::from_secs(2),   // 2 seconds
        demote_to_cold_timeout: Duration::from_secs(4),   // 4 seconds  
        demote_to_frozen_timeout: Duration::from_secs(6), // 6 seconds
        storage_dir: std::env::temp_dir().join("tier_test"),
        enable_compression: true,
        tier_management_interval: Duration::from_secs(1),
    };
    
    let cache = MultiTierCache::new(config).expect("Failed to create cache");
    
    // Store files
    println!("1. STORING FILES:");
    println!("{}", "-".repeat(40));
    
    for i in 0..5 {
        let source = format!("fn test{}() {{ }}", i);
        let path = PathBuf::from(format!("test{}.rs", i));
        
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(&source, None).unwrap();
        
        cache.store(path.clone(), i as u64, tree, source.as_bytes())
            .expect("Failed to store");
        
        println!("✅ Stored: test{}.rs", i);
    }
    
    let initial_stats = cache.stats();
    println!("\n2. INITIAL STATE:");
    println!("{}", "-".repeat(40));
    println!("Hot: {}, Warm: {}, Cold: {}, Frozen: {}", 
        initial_stats.hot_entries,
        initial_stats.warm_entries, 
        initial_stats.cold_entries,
        initial_stats.frozen_entries);
    
    // Wait and check periodically
    println!("\n3. MONITORING TIER TRANSITIONS:");
    println!("{}", "-".repeat(40));
    
    for cycle in 0..10 {
        thread::sleep(Duration::from_secs(2));
        
        // Force tier management
        cache.manage_tiers().expect("Tier management failed");
        
        let stats = cache.stats();
        println!("After {}s: Hot: {}, Warm: {}, Cold: {}, Frozen: {}", 
            (cycle + 1) * 2,
            stats.hot_entries,
            stats.warm_entries, 
            stats.cold_entries,
            stats.frozen_entries);
        
        // Check if we have any transitions
        if stats.warm_entries > 0 || stats.cold_entries > 0 || stats.frozen_entries > 0 {
            println!("✅ TIER TRANSITIONS DETECTED!");
        }
        
        // Access one file to keep it hot
        if cycle % 3 == 0 {
            let path = PathBuf::from("test0.rs");
            cache.get(&path).ok();
            println!("  (Accessed test0.rs to keep it hot)");
        }
    }
    
    // Final check
    let final_stats = cache.stats();
    println!("\n4. FINAL STATE:");
    println!("{}", "-".repeat(40));
    println!("Hot: {}, Warm: {}, Cold: {}, Frozen: {}", 
        final_stats.hot_entries,
        final_stats.warm_entries, 
        final_stats.cold_entries,
        final_stats.frozen_entries);
    println!("Promotions: {}, Demotions: {}", 
        final_stats.promotions,
        final_stats.demotions);
    
    // Print tier info
    println!("\n5. DETAILED TIER INFO:");
    println!("{}", "-".repeat(40));
    println!("{}", cache.tier_info());
    
    if final_stats.demotions > 0 {
        println!("\n✅ SUCCESS: {} tier demotions occurred!", final_stats.demotions);
    } else {
        println!("\n❌ FAILURE: No tier transitions detected!");
    }
}
