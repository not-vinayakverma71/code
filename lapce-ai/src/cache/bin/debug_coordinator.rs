/// Debug why coordinator isn't caching
use crate::cache::*;
use crate::cache::types::{CacheConfig, L1Config, L2Config, CompressionType, CacheKey, CacheValue};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== DEBUGGING COORDINATOR ISSUE ===\n");

    let config = CacheConfig {
        l1_config: L1Config {
            max_entries: 100,
            ttl: Duration::from_secs(3600),
            idle_time: Duration::from_secs(600),
            bloom_size: 1000,
            bloom_fp_rate: 0.01,
        },
        l2_config: L2Config {
            cache_dir: std::path::PathBuf::from("/tmp/debug_coord_l2"),
            max_size: 10 * 1024 * 1024,
            compression: CompressionType::None,
        },
        l3_redis_url: None,
    };

    let cache_system = Arc::new(CacheSystem::new(config).await?);
    
    println!("Test 1: Put via CacheSystem");
    let key1 = CacheKey("sys_key_1".to_string());
    let value1 = CacheValue::new(vec![1, 2, 3]);
    
    println!("  Putting key via system: {:?}", key1);
    cache_system.put(key1.clone(), value1.clone()).await;
    
    println!("  Getting key via system: {:?}", key1);
    let result = cache_system.get(&key1).await;
    println!("  Result: {}\n", result.is_some());
    
    println!("Test 2: Put via Coordinator directly");
    let key2 = CacheKey("coord_key_2".to_string());
    let value2 = CacheValue::new(vec![4, 5, 6]);
    
    println!("  Putting key via coordinator");
    cache_system.coordinator.put(key2.clone(), value2.clone()).await;
    
    println!("  Getting key via coordinator");
    let result2 = cache_system.coordinator.get(&key2).await;
    println!("  Result: {}\n", result2.is_some());
    
    println!("Test 3: Put to L1 directly then get via coordinator");
    let key3 = CacheKey("l1_key_3".to_string());
    let value3 = CacheValue::new(vec![7, 8, 9]);
    
    println!("  Putting key to L1 directly");
    cache_system.l1_cache.put(key3.clone(), value3.clone()).await;
    
    println!("  Getting from L1 directly");
    let l1_result = cache_system.l1_cache.get(&key3).await;
    println!("  L1 direct result: {}", l1_result.is_some());
    
    println!("  Getting via coordinator");
    let coord_result = cache_system.coordinator.get(&key3).await;
    println!("  Coordinator result: {}\n", coord_result.is_some());
    
    println!("Test 4: Check promotion policy");
    let key4 = CacheKey("policy_key_4".to_string());
    let value4 = CacheValue::new(vec![10, 11, 12]);
    
    let levels = cache_system.coordinator.promotion_policy.determine_levels(&key4, &value4);
    println!("  Levels for new item: {:?}", levels);
    
    println!("\nTest 5: Check L2 directly");
    let key5 = CacheKey("l2_key_5".to_string());
    let value5 = CacheValue::new(vec![13, 14, 15]);
    
    println!("  Putting to L2 directly");
    let l2_put = cache_system.l2_cache.put(key5.clone(), value5.clone()).await;
    println!("  L2 put result: {:?}", l2_put);
    
    println!("  Getting from L2 directly");
    let l2_get = cache_system.l2_cache.get(&key5).await;
    println!("  L2 get result: {}", l2_get.is_ok() && l2_get.unwrap().is_some());
    
    std::fs::remove_dir_all("/tmp/debug_coord_l2").ok();
    Ok(())
}
