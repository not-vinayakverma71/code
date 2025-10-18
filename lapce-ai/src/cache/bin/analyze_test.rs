/// Analyze why cache hit rate is only 10%
use crate::cache::*;
use crate::cache::types::{CacheConfig, L1Config, L2Config, CompressionType, CacheKey, CacheValue};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== ANALYZING CACHE HIT RATE ISSUE ===\n");

    let config = CacheConfig {
        l1_config: L1Config {
            max_entries: 1_000,
            ttl: Duration::from_secs(300),
            idle_time: Duration::from_secs(60),
            bloom_size: 10_000,
            bloom_fp_rate: 0.01,
        },
        l2_config: L2Config {
            cache_dir: std::path::PathBuf::from("/tmp/analyze_l2"),
            max_size: 100 * 1024 * 1024,
            compression: CompressionType::Lz4,
        },
        l3_redis_url: None,
    };

    let cache = Arc::new(CacheSystem::new(config).await?);
    
    println!("Phase 1: Populate cache with 10,000 items");
    for i in 0..10_000 {
        let key = CacheKey(format!("key_{}", i));
        let value = CacheValue::new(vec![i as u8; 100]);
        cache.put(key, value).await;
    }
    
    println!("\nPhase 2: Check what's actually cached");
    let mut l1_count = 0;
    let mut l2_count = 0;
    
    // Check first 100 keys (the ones we'll query in hit rate test)
    for i in 0..100 {
        let key = CacheKey(format!("key_{}", i));
        
        if cache.l1_cache.get(&key).await.is_some() {
            l1_count += 1;
        }
        if cache.l2_cache.get(&key).await.unwrap_or(None).is_some() {
            l2_count += 1;
        }
    }
    
    println!("Keys 0-99 in L1: {}/100", l1_count);
    println!("Keys 0-99 in L2: {}/100", l2_count);
    
    println!("\nPhase 3: Check last 100 keys");
    let mut l1_last = 0;
    let mut l2_last = 0;
    
    for i in 9900..10_000 {
        let key = CacheKey(format!("key_{}", i));
        
        if cache.l1_cache.get(&key).await.is_some() {
            l1_last += 1;
        }
        if cache.l2_cache.get(&key).await.unwrap_or(None).is_some() {
            l2_last += 1;
        }
    }
    
    println!("Keys 9900-9999 in L1: {}/100", l1_last);
    println!("Keys 9900-9999 in L2: {}/100", l2_last);
    
    println!("\nPhase 4: Test hit rate like the real test");
    cache.metrics.reset();
    let mut actual_hits = 0;
    
    for i in 0..1000 {
        let key = if i % 10 == 0 {
            CacheKey(format!("missing_{}", i))  // 10% missing
        } else {
            CacheKey(format!("key_{}", i % 100))  // 90% should hit keys 0-99
        };
        
        if cache.get(&key).await.is_some() {
            actual_hits += 1;
        }
    }
    
    println!("\nHit rate: {}/1000 = {:.1}%", actual_hits, (actual_hits as f64 / 10.0));
    
    std::fs::remove_dir_all("/tmp/analyze_l2").ok();
    Ok(())
}
