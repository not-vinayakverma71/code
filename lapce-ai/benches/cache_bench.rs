use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lapce_ai_rust::cache::{
    final_cache::CacheV3,
    types::{CacheConfig, CacheKey, CacheValue, L1Config, L2Config},
};
use std::time::Duration;
use tokio::runtime::Runtime;

fn cache_hit_rate_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let config = CacheConfig {
        l1_config: L1Config {
            max_entries: 1000,
            ttl: Duration::from_secs(60),
            idle_time: Duration::from_secs(30),
            bloom_size: 100000,
            bloom_fp_rate: 0.01,
        },
        l2_config: L2Config {
            max_size: 100_000_000, // 100MB
            compression: true,
            cache_dir: std::path::PathBuf::from("/tmp/lapce_cache_bench"),
        },
        l3_redis_url: None,
    };
    
    let cache = rt.block_on(async {
        CacheV3::new(config).await.unwrap()
    });
    
    // Pre-populate cache
    rt.block_on(async {
        for i in 0..100 {
            let key = CacheKey(format!("key_{}", i));
            let value = CacheValue::new(vec![i as u8; 100]);
            cache.put(key, value).await;
        }
    });
    
    let mut group = c.benchmark_group("cache_hit_rate");
    
    // L1 cache hits (hot data)
    group.bench_function("l1_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let key = CacheKey(format!("key_{}", black_box(42)));
            let _ = cache.get(&key).await;
        });
    });
    
    // L1 cache miss (cold data)
    group.bench_function("l1_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let key = CacheKey(format!("missing_key_{}", black_box(999)));
            let _ = cache.get(&key).await;
        });
    });
    
    group.finish();
}

fn cache_throughput_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let config = CacheConfig {
        l1_config: L1Config {
            max_entries: 10000,
            ttl: Duration::from_secs(300),
            idle_time: Duration::from_secs(60),
            bloom_size: 1000000,
            bloom_fp_rate: 0.001,
        },
        l2_config: L2Config {
            max_size: 1_000_000_000, // 1GB
            compression: false,
            cache_dir: std::path::PathBuf::from("/tmp/lapce_cache_throughput"),
        },
        l3_redis_url: None,
    };
    
    let cache = rt.block_on(async {
        CacheV3::new(config).await.unwrap()
    });
    
    let mut group = c.benchmark_group("cache_throughput");
    
    // Write throughput
    group.bench_function("write_ops_per_sec", |b| {
        let mut counter = 0u64;
        b.to_async(&rt).iter(|| async {
            let key = CacheKey(format!("write_key_{}", counter));
            let value = CacheValue::new(vec![0u8; 1000]);
            cache.put(key, value).await;
            counter += 1;
        });
    });
    
    // Read throughput
    group.bench_function("read_ops_per_sec", |b| {
        // Pre-populate
        rt.block_on(async {
            for i in 0..1000 {
                let key = CacheKey(format!("read_key_{}", i));
                let value = CacheValue::new(vec![i as u8; 1000]);
                cache.put(key, value).await;
            }
        });
        
        let mut counter = 0u64;
        b.to_async(&rt).iter(|| async {
            let key = CacheKey(format!("read_key_{}", counter % 1000));
            let _ = cache.get(&key).await;
            counter += 1;
        });
    });
    
    // Mixed workload (80% read, 20% write)
    group.bench_function("mixed_workload", |b| {
        let mut counter = 0u64;
        b.to_async(&rt).iter(|| async {
            if counter % 5 == 0 {
                // Write
                let key = CacheKey(format!("mixed_key_{}", counter));
                let value = CacheValue::new(vec![0u8; 1000]);
                cache.put(key, value).await;
            } else {
                // Read
                let key = CacheKey(format!("mixed_key_{}", counter % 100));
                let _ = cache.get(&key).await;
            }
            counter += 1;
        });
    });
    
    group.finish();
}

fn cache_memory_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("cache_memory");
    
    // Memory overhead per entry
    group.bench_function(BenchmarkId::new("memory_per_entry", 1000), |b| {
        b.to_async(&rt).iter(|| async {
            let config = CacheConfig {
                l1_config: L1Config {
                    max_entries: black_box(1000),
                    ttl: Duration::from_secs(300),
                    idle_time: Duration::from_secs(60),
                    bloom_size: 100000,
                    bloom_fp_rate: 0.01,
                },
                l2_config: L2Config {
                    max_size: 100_000_000,
                    compression: true,
                    cache_dir: std::path::PathBuf::from("/tmp/lapce_cache_mem"),
                },
                l3_redis_url: None,
            };
            
            let cache = CacheV3::new(config).await.unwrap();
            
            // Add entries
            for i in 0..1000 {
                let key = CacheKey(format!("mem_key_{}", i));
                let value = CacheValue::new(vec![i as u8; 100]);
                cache.put(key, value).await;
            }
            
            // Force a reference to prevent optimization
            black_box(&cache);
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    cache_hit_rate_benchmark,
    cache_throughput_benchmark,
    cache_memory_benchmark
);
criterion_main!(benches);
