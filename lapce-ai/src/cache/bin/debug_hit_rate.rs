/// Debug why hit rate is 0%
use crate::cache::*;
use crate::cache::types::{CacheConfig, L1Config, L2Config, CompressionType, CacheKey, CacheValue};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== DEBUGGING HIT RATE ===\n");

    let config = CacheConfig {
        l1_config: L1Config {
            max_entries: 1_000,
            ttl: Duration::from_secs(300),
            idle_time: Duration::from_secs(60),
            bloom_size: 10_000,
            bloom_fp_rate: 0.01,
        },
        l2_config: L2Config {
            cache_dir: std::path::PathBuf::from("/tmp/debug_hit_l2"),
            max_size: 100 * 1024 * 1024,
            compression: CompressionType::Lz4,
        },
        l3_redis_url: None,
    };

    let cache = Arc::new(CacheSystem::new(config).await?);
    
    // Put a test item
    let key = CacheKey("test_key".to_string());
    let value = CacheValue::new(vec![1, 2, 3, 4, 5]);
    
    println!("1. Putting key via cache.put()");
    cache.put(key.clone(), value.clone()).await;
    
    // Wait a moment for async operations
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    println!("2. Getting key via cache.get()");
    let result = cache.get(&key).await;
    println!("   Result: {:?}", result.is_some());
    
    println!("3. Getting key via coordinator.get()");
    let coord_result = cache.coordinator.get(&key).await;
    println!("   Result: {:?}", coord_result.is_some());
    
    println!("4. Getting key via L1 directly");
    let l1_result = cache.l1_cache.get(&key).await;
    println!("   Result: {:?}", l1_result.is_some());
    
    println!("5. Getting key via L2 directly");
    let l2_result = cache.l2_cache.get(&key).await?;
    println!("   Result: {:?}", l2_result.is_some());
    
    // Test with 100-byte item (common in test)
    let key2 = CacheKey("test_100".to_string());
    let value2 = CacheValue::new(vec![1; 100]);
    
    println!("\n6. Testing 100-byte item");
    cache.put(key2.clone(), value2.clone()).await;
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    let result2 = cache.get(&key2).await;
    println!("   cache.get(): {:?}", result2.is_some());
    
    let l2_result2 = cache.l2_cache.get(&key2).await?;
    println!("   L2 direct: {:?}", l2_result2.is_some());
    
    std::fs::remove_dir_all("/tmp/debug_hit_l2").ok();
    Ok(())
}
