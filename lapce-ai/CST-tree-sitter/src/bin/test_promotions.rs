//! Test that promotions work correctly

use lapce_tree_sitter::{MultiTierCache, MultiTierConfig};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::thread;
use tree_sitter::Parser;
use bytes::Bytes;
use lapce_tree_sitter::compact::bytecode::{
    TreeSitterBytecodeEncoder,
    SegmentedBytecodeStream,
};
use std::sync::Arc;

fn main() {
    println!("=== TESTING PROMOTIONS ===\n");
    
    // Configure with low promotion thresholds
    let config = MultiTierConfig {
        hot_tier_mb: 2,
        warm_tier_mb: 2,
        cold_tier_mb: 2,
        promote_to_hot_threshold: 3,    // After 3 accesses, warm→hot
        promote_to_warm_threshold: 2,   // After 2 accesses, cold→warm
        demote_to_warm_timeout: Duration::from_secs(2),
        demote_to_cold_timeout: Duration::from_secs(4),
        demote_to_frozen_timeout: Duration::from_secs(6),
        storage_dir: std::env::temp_dir().join("promotion_test"),
        enable_compression: false,
        tier_management_interval: Duration::from_secs(1),
    };
    
    let cache = MultiTierCache::new(config).expect("Failed to create cache");
    
    // Store a file
    let path = PathBuf::from("test.rs");
    let source = "fn test() { println!(\"test\"); }";
    
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    // Create bytecode
    let mut encoder = TreeSitterBytecodeEncoder::new();
    let bytecode = encoder.encode_tree(&tree, source.as_bytes());
    let segmented = SegmentedBytecodeStream::from_bytecode_stream(
        bytecode,
        std::env::temp_dir().join("promotion_test_segments")
    ).unwrap();
    
    // Store in cache
    cache.store(path.clone(), 12345, tree, source.as_bytes()).unwrap();
    
    let initial_stats = cache.stats();
    println!("Initial state:");
    println!("  Hot: {}, Warm: {}, Cold: {}", 
        initial_stats.hot_entries,
        initial_stats.warm_entries,
        initial_stats.cold_entries);
    println!("  Promotions: {}", initial_stats.promotions);
    
    // Wait for demotion to warm
    println!("\nWaiting 3 seconds for demotion to warm...");
    thread::sleep(Duration::from_secs(3));
    cache.manage_tiers().unwrap();
    
    let after_warm_demotion = cache.stats();
    println!("After demotion to warm:");
    println!("  Hot: {}, Warm: {}, Cold: {}", 
        after_warm_demotion.hot_entries,
        after_warm_demotion.warm_entries,
        after_warm_demotion.cold_entries);
    
    // Access the file 3 times to trigger promotion back to hot
    println!("\nAccessing file 3 times to trigger promotion...");
    for i in 1..=3 {
        cache.get(&path).unwrap();
        println!("  Access #{}", i);
    }
    
    let after_accesses = cache.stats();
    println!("\nAfter 3 accesses:");
    println!("  Hot: {}, Warm: {}, Cold: {}", 
        after_accesses.hot_entries,
        after_accesses.warm_entries,
        after_accesses.cold_entries);
    println!("  Promotions: {}", after_accesses.promotions);
    
    if after_accesses.promotions > initial_stats.promotions {
        println!("\n✅ PROMOTION SUCCESSFUL! {} promotions occurred", 
            after_accesses.promotions - initial_stats.promotions);
    } else {
        println!("\n❌ NO PROMOTIONS OCCURRED!");
    }
    
    // Test cold→warm promotion
    println!("\n--- Testing Cold→Warm Promotion ---");
    
    // Wait for demotion to cold
    println!("Waiting 5 more seconds for demotion to cold...");
    thread::sleep(Duration::from_secs(5));
    cache.manage_tiers().unwrap();
    
    let after_cold_demotion = cache.stats();
    println!("After demotion to cold:");
    println!("  Hot: {}, Warm: {}, Cold: {}", 
        after_cold_demotion.hot_entries,
        after_cold_demotion.warm_entries,
        after_cold_demotion.cold_entries);
    
    // Access twice for cold→warm promotion
    println!("\nAccessing file 2 times for cold→warm promotion...");
    for i in 1..=2 {
        cache.get(&path).unwrap();
        println!("  Access #{}", i);
    }
    
    let final_stats = cache.stats();
    println!("\nFinal state:");
    println!("  Hot: {}, Warm: {}, Cold: {}", 
        final_stats.hot_entries,
        final_stats.warm_entries,
        final_stats.cold_entries);
    println!("  Total promotions: {}", final_stats.promotions);
    
    if final_stats.promotions > after_accesses.promotions {
        println!("\n✅ COLD→WARM PROMOTION SUCCESSFUL!");
    } else {
        println!("\n⚠️ Cold→Warm promotion may not have triggered");
    }
    
    // Summary
    println!("\n=== SUMMARY ===");
    println!("Total promotions during test: {}", final_stats.promotions);
    println!("Total demotions during test: {}", final_stats.demotions);
    
    if final_stats.promotions > 0 {
        println!("\n✅ PROMOTION LOGIC IS WORKING!");
    } else {
        println!("\n❌ PROMOTION LOGIC NEEDS INVESTIGATION");
    }
}
