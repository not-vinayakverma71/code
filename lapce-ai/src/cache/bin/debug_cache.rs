/// Debug why cache hit rate is 0%
use crate::cache::*;
use crate::cache::types::{CacheConfig, L1Config, L2Config, CompressionType, CacheKey, CacheValue};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== DEBUGGING CACHE HIT RATE ISSUE ===\n");

    let config = CacheConfig {
        l1_config: L1Config {
            max_entries: 100,
            ttl: Duration::from_secs(3600),
            idle_time: Duration::from_secs(600),
            bloom_size: 1000,
            bloom_fp_rate: 0.01,
        },
        l2_config: L2Config {
            cache_dir: std::path::PathBuf::from("/tmp/debug_cache_l2"),
            max_size: 10 * 1024 * 1024,
            compression: CompressionType::None,
        },
        l3_redis_url: None,
    };

    let cache = Arc::new(CacheSystem::new(config).await?);
    
    // Test 1: Simple put and get
    println!("Test 1: Simple put/get");
    let key1 = CacheKey("test_key_1".to_string());
    let value1 = CacheValue::new(vec![1, 2, 3, 4, 5]);
    
    println!("  Putting key: {:?}", key1);
    cache.put(key1.clone(), value1.clone()).await;
    
    println!("  Getting key: {:?}", key1);
    let retrieved = cache.get(&key1).await;
    println!("  Retrieved: {:?}\n", retrieved.is_some());
    
    // Test 2: Check L1 directly
    println!("Test 2: Check L1 cache directly");
    let key2 = CacheKey("test_key_2".to_string());
    let value2 = CacheValue::new(vec![6, 7, 8]);
    
    println!("  Putting to L1 directly");
    cache.l1_cache.put(key2.clone(), value2.clone()).await;
    
    println!("  Getting from L1 directly");
    let l1_result = cache.l1_cache.get(&key2).await;
    println!("  L1 result: {:?}\n", l1_result.is_some());
    
    // Test 3: Check bloom filter
    println!("Test 3: Bloom filter check");
    let key3 = CacheKey("test_key_3".to_string());
    
    println!("  Checking bloom filter before insert");
    let bf_before = cache.l1_cache.bloom_filter.read().await.contains(&key3);
    println!("  Bloom filter contains key3 (before): {}", bf_before);
    
    cache.l1_cache.bloom_filter.write().await.insert(&key3);
    
    println!("  Checking bloom filter after insert");
    let bf_after = cache.l1_cache.bloom_filter.read().await.contains(&key3);
    println!("  Bloom filter contains key3 (after): {}\n", bf_after);
    
    // Test 4: Check should_cache logic
    println!("Test 4: Check should_cache logic");
    let key4 = CacheKey("test_key_4".to_string());
    let value4 = CacheValue::new(vec![9, 10, 11]);
    
    // Add to bloom filter first
    cache.l1_cache.bloom_filter.write().await.insert(&key4);
    
    // Record some access to increase frequency
    cache.l1_cache.access_counter.record(&key4);
    cache.l1_cache.access_counter.record(&key4);
    
    let frequency = cache.l1_cache.access_counter.frequency(&key4);
    println!("  Frequency for key4: {}", frequency);
    
    // Now try to cache
    cache.l1_cache.put(key4.clone(), value4.clone()).await;
    let cached = cache.l1_cache.get(&key4).await;
    println!("  Cached after put: {:?}\n", cached.is_some());
    
    // Test 5: Check MokaCache directly
    println!("Test 5: MokaCache direct test");
    let key5 = CacheKey("test_key_5".to_string());
    let value5 = CacheValue::new(vec![12, 13, 14]);
    
    // Insert directly into MokaCache
    cache.l1_cache.cache.insert(key5.clone(), value5.clone()).await;
    
    // Get directly from MokaCache
    let moka_result = cache.l1_cache.cache.get(&key5).await;
    println!("  MokaCache contains key5: {}\n", moka_result.is_some());
    
    // Test 6: Check metrics
    println!("Test 6: Cache metrics");
    println!("  L1 hits: {}", cache.metrics.l1_hits.load(std::sync::atomic::Ordering::Relaxed));
    println!("  L1 misses: {}", cache.metrics.l1_misses.load(std::sync::atomic::Ordering::Relaxed));
    println!("  Hit rate: {:.2}%", cache.metrics.get_hit_rate() * 100.0);
    
    std::fs::remove_dir_all("/tmp/debug_cache_l2").ok();
    Ok(())
}
