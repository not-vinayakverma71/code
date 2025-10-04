use lapce_ai_rust::cache_v2::proper_cache_system::CacheSystem;
use std::time::Instant;
use tempfile::tempdir;

#[tokio::main]
async fn main() {
    println!("\n=== ACTUAL PERFORMANCE TEST ===\n");
    
    // Create cache with temp directories
    let temp_l2 = tempdir().unwrap();
    let temp_l3 = tempdir().unwrap();
    
    let cache = CacheSystem::new(
        1000,     // L1 max entries
        temp_l2.path().to_str().unwrap(),
        Some(temp_l3.path().to_str().unwrap()), // L3
    ).await.unwrap();
    
    // Test L1 throughput
    println!("Testing L1 throughput...");
    let key = "test_key".to_string();
    cache.put(key.clone(), vec![1; 100]).await.unwrap();
    
    // Warm up
    for _ in 0..1000 {
        cache.get(&key).await;
    }
    
    // Actual test
    let start = Instant::now();
    let iterations = 100_000;
    for _ in 0..iterations {
        cache.get(&key).await;
    }
    let elapsed = start.elapsed();
    
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
    println!("L1 Throughput: {:.0} ops/s", ops_per_sec);
    
    if ops_per_sec > 100_000.0 {
        println!("✅ PASSES: {} ops/s > 100,000 ops/s", ops_per_sec as u64);
    } else {
        println!("❌ FAILS: {} ops/s < 100,000 ops/s", ops_per_sec as u64);
    }
}
