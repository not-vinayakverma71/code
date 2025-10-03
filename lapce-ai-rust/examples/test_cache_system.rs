use lapce_ai_rust::working_cache_system::{WorkingCacheSystem, CacheStats};
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing WorkingCacheSystem (L1/L2/L3)\n");
    
    let cache = WorkingCacheSystem::new().await?;
    println!("✓ Cache system initialized");
    
    // Check initial stats
    let stats = cache.stats().await;
    println!("\nInitial Stats:");
    println!("  L1 (Moka): {} entries", stats.l1_entries);
    println!("  L2 (Sled): {} entries", stats.l2_entries);
    println!("  L3 (Redis): {}", if stats.l3_connected { "Connected" } else { "Not available" });
    
    // Test write performance
    println!("\n=== Write Performance ===");
    let test_sizes = vec![
        (64, 10000, "64B × 10K"),
        (256, 5000, "256B × 5K"),
        (1024, 2000, "1KB × 2K"),
    ];
    
    for (size, count, label) in test_sizes {
        let value = vec![0xAB; size];
        let start = Instant::now();
        
        for i in 0..count {
            let key = format!("key_{}_{}", size, i);
            cache.set(&key, value.clone()).await?;
        }
        
        let elapsed = start.elapsed();
        let ops_per_sec = count as f64 / elapsed.as_secs_f64();
        
        println!("{}: {:.0} ops/sec", label, ops_per_sec);
    }
    
    // Check stats after writes
    let stats = cache.stats().await;
    println!("\nAfter Writes:");
    println!("  L1: {} entries", stats.l1_entries);
    println!("  L2: {} entries", stats.l2_entries);
    
    // Test read performance (cache hits)
    println!("\n=== Read Performance (Hits) ===");
    
    // L1 hits
    let start = Instant::now();
    let mut hits = 0;
    for i in 0..1000 {
        let key = format!("key_64_{}", i);
        if cache.get(&key).await.is_some() {
            hits += 1;
        }
    }
    let l1_time = start.elapsed();
    println!("L1 hits: {}/1000 in {:.2}ms ({:.0} ops/sec)", 
             hits, l1_time.as_millis(), 1000.0 / l1_time.as_secs_f64());
    
    // Clear L1 to test L2 hits
    cache.clear().await?;
    println!("\n✓ Cleared cache");
    
    // Re-populate L2 only
    for i in 0..1000 {
        let key = format!("test_{}", i);
        cache.set(&key, vec![i as u8; 64]).await?;
    }
    
    // Test cache miss
    let start = Instant::now();
    let mut misses = 0;
    for i in 10000..10100 {
        let key = format!("nonexistent_{}", i);
        if cache.get(&key).await.is_none() {
            misses += 1;
        }
    }
    let miss_time = start.elapsed();
    println!("\nCache misses: {}/100 in {:.2}ms", misses, miss_time.as_millis());
    
    // Final stats
    let final_stats = cache.stats().await;
    println!("\nFinal Stats:");
    println!("  L1: {} entries", final_stats.l1_entries);
    println!("  L2: {} entries", final_stats.l2_entries);
    println!("  L3: {}", if final_stats.l3_connected { "Connected" } else { "Not available" });
    
    println!("\n✓ All cache tests completed!");
    Ok(())
}
