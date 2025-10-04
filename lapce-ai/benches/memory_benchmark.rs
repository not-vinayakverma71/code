use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lapce_ai_rust::memory::*;
use std::sync::Arc;

fn benchmark_arena_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("arena_allocation");
    
    // Compare arena allocation vs standard allocation
    group.bench_function("arena_small_alloc", |b| {
        b.iter(|| {
            let arena = ArenaScope::new();
            for i in 0..1000 {
                let value = arena.alloc(i);
                black_box(value);
            }
        });
    });
    
    group.bench_function("standard_small_alloc", |b| {
        b.iter(|| {
            let mut values = Vec::new();
            for i in 0..1000 {
                let value = Box::new(i);
                values.push(value);
            }
            black_box(values);
        });
    });
    
    // String allocation comparison
    group.bench_function("arena_string_alloc", |b| {
        b.iter(|| {
            let arena = ArenaScope::new();
            for i in 0..100 {
                let s = arena.alloc_str(&format!("test string {}", i));
                black_box(s);
            }
        });
    });
    
    group.bench_function("standard_string_alloc", |b| {
        b.iter(|| {
            let mut strings = Vec::new();
            for i in 0..100 {
                let s = format!("test string {}", i);
                strings.push(s);
            }
            black_box(strings);
        });
    });
    
    group.finish();
}

fn benchmark_object_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("object_pool");
    
    #[derive(Clone)]
    struct TestObject {
        data: Vec<u8>,
    }
    
    impl Default for TestObject {
        fn default() -> Self {
            Self {
                data: vec![0; 1024], // 1KB object
            }
        }
    }
    
    let pool = ObjectPool::new(256, TestObject::default);
    
    group.bench_function("pool_allocation", |b| {
        b.iter(|| {
            let mut objects = Vec::new();
            for _ in 0..100 {
                objects.push(pool.acquire());
            }
            // Objects return to pool when dropped
            drop(objects);
        });
    });
    
    group.bench_function("standard_allocation", |b| {
        b.iter(|| {
            let mut objects = Vec::new();
            for _ in 0..100 {
                objects.push(TestObject::default());
            }
            drop(objects);
        });
    });
    
    group.finish();
}

fn benchmark_string_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning");
    
    let interner = StringInterner::new();
    let test_strings: Vec<String> = (0..100)
        .map(|i| format!("string_{}", i % 20)) // 20 unique strings, repeated
        .collect();
    
    group.bench_function("with_interning", |b| {
        b.iter(|| {
            let mut interned = Vec::new();
            for s in &test_strings {
                interned.push(interner.intern(s));
            }
            black_box(interned);
        });
    });
    
    group.bench_function("without_interning", |b| {
        b.iter(|| {
            let mut arcs = Vec::new();
            for s in &test_strings {
                arcs.push(Arc::<str>::from(s.as_str()));
            }
            black_box(arcs);
        });
    });
    
    // Test deduplication benefit
    group.bench_function("interning_memory_benefit", |b| {
        b.iter(|| {
            let mut total_size = 0;
            for _ in 0..1000 {
                let s = interner.intern("frequently_used_string");
                total_size += s.len();
            }
            black_box(total_size);
        });
    });
    
    group.bench_function("no_interning_memory", |b| {
        b.iter(|| {
            let mut strings = Vec::new();
            let mut total_size = 0;
            for _ in 0..1000 {
                let s = "frequently_used_string".to_string();
                total_size += s.len();
                strings.push(s);
            }
            black_box((strings, total_size));
        });
    });
    
    group.finish();
}

fn benchmark_buffer_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_pool");
    
    let pool = BufferPool::new();
    
    for size in &[512, 4096, 65536] {
        group.bench_with_input(
            BenchmarkId::new("pool_buffer", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut buffers = Vec::new();
                    for _ in 0..10 {
                        let mut buf = pool.acquire(size);
                        buf.extend_from_slice(&vec![0u8; size]);
                        buffers.push(buf);
                    }
                    drop(buffers); // Return to pool
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("standard_buffer", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut buffers = Vec::new();
                    for _ in 0..10 {
                        let mut buf = Vec::with_capacity(size);
                        buf.extend_from_slice(&vec![0u8; size]);
                        buffers.push(buf);
                    }
                    drop(buffers);
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_generational_arena(c: &mut Criterion) {
    let mut group = c.benchmark_group("generational_arena");
    
    group.bench_function("insert_remove", |b| {
        let mut arena = GenerationalArena::<String>::new();
        let mut indices = Vec::new();
        
        // Pre-populate
        for i in 0..100 {
            indices.push(arena.insert(format!("item_{}", i)));
        }
        
        b.iter(|| {
            // Remove and re-insert
            for i in 0..10 {
                if let Some(idx) = indices.get(i) {
                    arena.remove(*idx);
                    let new_idx = arena.insert(format!("new_item_{}", i));
                    indices[i] = new_idx;
                }
            }
        });
    });
    
    group.bench_function("access_pattern", |b| {
        let mut arena = GenerationalArena::<i32>::new();
        let mut indices = Vec::new();
        
        for i in 0..1000 {
            indices.push(arena.insert(i));
        }
        
        b.iter(|| {
            let mut sum = 0;
            for idx in &indices {
                if let Some(value) = arena.get(*idx) {
                    sum += *value;
                }
            }
            black_box(sum);
        });
    });
    
    group.finish();
}

fn benchmark_memory_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_metrics");
    
    let metrics = MemoryMetrics::new();
    
    group.bench_function("record_allocation", |b| {
        b.iter(|| {
            metrics.record_allocation(1024);
            metrics.record_deallocation(1024);
        });
    });
    
    group.bench_function("get_stats", |b| {
        // Pre-populate some data
        for _ in 0..100 {
            metrics.record_allocation(1024);
        }
        
        b.iter(|| {
            let stats = metrics.stats();
            black_box(stats);
        });
    });
    
    group.finish();
}

fn benchmark_memory_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_manager");
    
    let manager = MemoryManager::new();
    
    group.bench_function("full_lifecycle", |b| {
        b.iter(|| {
            // Simulate request processing with various memory operations
            
            // Arena allocation
            {
                let arena = manager.get_arena();
                for i in 0..100 {
                    let value = arena.alloc(i);
                    black_box(value);
                }
            }
            
            // String interning
            for i in 0..50 {
                let s = manager.intern_string(&format!("string_{}", i % 10));
                black_box(s);
            }
            
            // Object pool usage
            {
                let pools = manager.pools();
                let mut requests = Vec::new();
                for _ in 0..10 {
                    requests.push(pools.request_pool.get());
                }
                drop(requests);
            }
            
            // Get metrics
            let allocated = manager.allocated_bytes();
            black_box(allocated);
        });
    });
    
    group.bench_function("concurrent_access", |b| {
        use std::thread;
        
        b.iter(|| {
            let manager = Arc::new(manager);
            let mut handles = Vec::new();
            
            for _ in 0..4 {
                let mgr = manager.clone();
                let handle = thread::spawn(move || {
                    for i in 0..25 {
                        let arena = mgr.get_arena();
                        let value = arena.alloc(i);
                        black_box(value);
                        
                        let s = mgr.intern_string(&format!("thread_string_{}", i));
                        black_box(s);
                    }
                });
                handles.push(handle);
            }
            
            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_arena_allocation,
    benchmark_object_pool,
    benchmark_string_interning,
    benchmark_buffer_pool,
    benchmark_generational_arena,
    benchmark_memory_metrics,
    benchmark_memory_manager
);

criterion_main!(benches);
