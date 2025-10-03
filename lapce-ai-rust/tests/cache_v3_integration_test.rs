#[cfg(feature = "cache_v3")]
mod cache_v3_integration_tests {
    use lapce_ai_rust::cache_v3::*;
    use lapce_ai_rust::cache_v3::types::{CacheConfig, L1Config, L2Config, CompressionType, CacheKey, CacheValue};
    use std::sync::Arc;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_cache_v3_basic_operations() {
        let config = CacheConfig {
            l1_config: L1Config {
                max_entries: 100,
                ttl: Duration::from_secs(60),
                idle_time: Duration::from_secs(10),
                bloom_size: 10_000,
                bloom_fp_rate: 0.01,
            },
            l2_config: L2Config {
                path: "/tmp/test_cache_v3_integration".to_string(),
                max_size: 10 * 1024 * 1024, // 10MB
                compression: CompressionType::Lz4,
            },
            l3_redis_url: None,
        };
        
        let cache = Arc::new(CacheSystem::new(config).await.unwrap());
        
        // Test put and get
        let key = CacheKey("test_key".to_string());
        let value = CacheValue::new(vec![1, 2, 3, 4, 5]);
        
        cache.put(key.clone(), value.clone()).await;
        
        let retrieved = cache.get(&key).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, vec![1, 2, 3, 4, 5]);
        
        // Clean up
        std::fs::remove_dir_all("/tmp/test_cache_v3_integration").ok();
    }
    
    #[tokio::test]
    async fn test_cache_v3_performance() {
        let config = CacheConfig {
            l1_config: L1Config {
                max_entries: 500,
                ttl: Duration::from_secs(300),
                idle_time: Duration::from_secs(60),
                bloom_size: 50_000,
                bloom_fp_rate: 0.01,
            },
            l2_config: L2Config {
                path: "/tmp/test_cache_v3_perf".to_string(),
                max_size: 100 * 1024 * 1024, // 100MB
                compression: CompressionType::Lz4,
            },
            l3_redis_url: None,
        };
        
        let cache = Arc::new(CacheSystem::new(config).await.unwrap());
        
        // Insert 1000 items
        for i in 0..1000 {
            let key = CacheKey(format!("key_{}", i));
            let value = CacheValue::new(vec![i as u8; 100]);
            cache.put(key, value).await;
        }
        
        // Measure query performance
        let start = std::time::Instant::now();
        for i in 0..1000 {
            let key = CacheKey(format!("key_{}", i % 100));
            let _ = cache.get(&key).await;
        }
        let elapsed = start.elapsed();
        
        // Should complete 1000 queries in under 100ms
        assert!(elapsed.as_millis() < 100);
        
        // Clean up
        std::fs::remove_dir_all("/tmp/test_cache_v3_perf").ok();
    }
    
    #[tokio::test]
    async fn test_cache_v3_metrics() {
        let config = CacheConfig {
            l1_config: L1Config {
                max_entries: 50,
                ttl: Duration::from_secs(60),
                idle_time: Duration::from_secs(10),
                bloom_size: 5_000,
                bloom_fp_rate: 0.01,
            },
            l2_config: L2Config {
                path: "/tmp/test_cache_v3_metrics".to_string(),
                max_size: 10 * 1024 * 1024,
                compression: CompressionType::None,
            },
            l3_redis_url: None,
        };
        
        let cache = Arc::new(CacheSystem::new(config).await.unwrap());
        
        // Populate cache
        for i in 0..10 {
            let key = CacheKey(format!("metric_key_{}", i));
            let value = CacheValue::new(vec![i as u8; 50]);
            cache.put(key, value).await;
        }
        
        // Test hits
        cache.metrics.reset();
        for i in 0..10 {
            let key = CacheKey(format!("metric_key_{}", i));
            let _ = cache.get(&key).await;
        }
        
        // Test misses
        for i in 10..15 {
            let key = CacheKey(format!("missing_key_{}", i));
            let _ = cache.get(&key).await;
        }
        
        let hit_rate = cache.metrics.get_hit_rate();
        // Should have 10 hits and 5 misses = 66.7% hit rate
        assert!(hit_rate > 0.6);
        
        // Clean up
        std::fs::remove_dir_all("/tmp/test_cache_v3_metrics").ok();
    }
}
