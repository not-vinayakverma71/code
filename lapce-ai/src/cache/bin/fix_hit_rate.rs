/// Fix the hit rate issue - test shows 0% but cache works
use crate::cache::*;
use crate::cache::types::{CacheConfig, L1Config, L2Config, CompressionType, CacheKey, CacheValue};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== FIXING HIT RATE ISSUE ===\n");

    let config = CacheConfig {
        l1_config: L1Config {
            max_entries: 1_000,
            ttl: Duration::from_secs(300),
            idle_time: Duration::from_secs(60),
            bloom_size: 10_000,
            bloom_fp_rate: 0.01,
        },
        l2_config: L2Config {
            cache_dir: std::path::PathBuf::from("/tmp/fix_hit_l2"),
            max_size: 100 * 1024 * 1024,
            compression: CompressionType::Lz4,
        },
        l3_redis_url: None,
    };

    let cache = Arc::new(CacheSystem::new(config).await?);
    
    // Exactly replicate what the test does
    println!("Step 1: Populate with 10,000 items like the test");
    for i in 0..10_000 {
        let key = CacheKey(format!("key_{}", i));
        let value = CacheValue::new(vec![i as u8; 100]);
        cache.put(key, value).await;
        
        // Show progress
        if i % 1000 == 0 {
            println!("  Added {} items...", i);
        }
    }
    
    // Wait for any async operations
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    println!("\nStep 2: Check what's in each layer");
    let mut l1_count = 0;
    let mut l2_count = 0;
    
    for i in 0..100 {
        let key = CacheKey(format!("key_{}", i));
        if cache.l1_cache.get(&key).await.is_some() {
            l1_count += 1;
        }
        if let Ok(Some(_)) = cache.l2_cache.get(&key).await {
            l2_count += 1;
        }
    }
    
    println!("Keys 0-99 in L1: {}/100", l1_count);
    println!("Keys 0-99 in L2: {}/100", l2_count);
    
    println!("\nStep 3: Reset metrics and test hit rate");
    cache.metrics.reset();
    
    let mut hits_via_system = 0;
    let mut hits_via_coordinator = 0;
    let mut hits_expected = 0;
    
    for i in 0..1000 {
        let key = if i % 10 == 0 {
            // 100 missing keys (10%)
            CacheKey(format!("missing_{}", i))
        } else {
            // 900 keys that should hit (90%)
            hits_expected += 1;
            CacheKey(format!("key_{}", i % 100))
        };
        
        // Test via cache system
        if cache.get(&key).await.is_some() {
            hits_via_system += 1;
        }
        
        // Also test via coordinator directly
        if cache.coordinator.get(&key).await.is_some() {
            hits_via_coordinator += 1;
        }
    }
    
    println!("\nResults:");
    println!("  Expected hits: {}/1000 (90%)", hits_expected);
    println!("  Hits via cache.get(): {}/1000", hits_via_system);
    println!("  Hits via coordinator.get(): {}/1000", hits_via_coordinator);
    println!("  Metrics hit rate: {:.1}%", cache.metrics.get_hit_rate() * 100.0);
    
    println!("\nStep 4: Test with simpler scenario");
    // Put 100 items
    for i in 0..100 {
        let key = CacheKey(format!("simple_{}", i));
        let value = CacheValue::new(vec![i as u8; 50]); // Small items
        cache.put(key, value).await;
    }
    
    // Wait for writes
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Get them back
    let mut simple_hits = 0;
    for i in 0..100 {
        let key = CacheKey(format!("simple_{}", i));
        if cache.get(&key).await.is_some() {
            simple_hits += 1;
        }
    }
    
    println!("Simple test: {}/100 hits", simple_hits);
    
    std::fs::remove_dir_all("/tmp/fix_hit_l2").ok();
    Ok(())
}
