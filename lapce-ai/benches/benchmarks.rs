// PHASE 4.3: COMPREHENSIVE BENCHMARK SUITE
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;
use std::collections::HashMap;

// BENCHMARK 1: Embedding Generation Performance
fn bench_embedding_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("embedding_generation");
    
    for size in [100, 500, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                // Simulate embedding generation
                let text = "a".repeat(size);
                generate_embedding(black_box(&text))
            });
        });
    }
    group.finish();
}

// BENCHMARK 2: Search Performance
fn bench_search_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_performance");
    
    // Setup test index
    let index = create_test_index(10000);
    
    for query_complexity in ["simple", "medium", "complex"].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(query_complexity), 
            query_complexity,
            |b, &complexity| {
                b.iter(|| {
                    let query = generate_query(complexity);
                    search_index(&index, black_box(&query))
                });
            }
        );
    }
    group.finish();
}

// BENCHMARK 3: Indexing Speed
fn bench_indexing_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing_speed");
    group.measurement_time(Duration::from_secs(10));
    
    for num_files in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_files as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_files),
            num_files,
            |b, &num| {
                let files = generate_test_files(num);
                b.iter(|| {
                    index_files(black_box(&files))
                });
            }
        );
    }
    group.finish();
}

// BENCHMARK 4: Cache Performance
fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");
    
    let mut cache = HashMap::new();
    // Populate cache
    for i in 0..1000 {
        cache.insert(format!("key_{}", i), format!("value_{}", i));
    }
    
    // Benchmark cache hits
    group.bench_function("cache_hits", |b| {
        b.iter(|| {
            for i in 0..100 {
                let key = format!("key_{}", i % 1000);
                cache.get(black_box(&key));
            }
        });
    });
    
    // Benchmark cache misses
    group.bench_function("cache_misses", |b| {
        b.iter(|| {
            for i in 1000..1100 {
                let key = format!("key_{}", i);
                cache.get(black_box(&key));
            }
        });
    });
    
    group.finish();
}

// BENCHMARK 5: Memory Usage
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    for data_size in [1_000, 10_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(data_size),
            data_size,
            |b, &size| {
                b.iter(|| {
                    let data = allocate_data(size);
                    process_data(black_box(&data))
                });
            }
        );
    }
    group.finish();
}

// BENCHMARK 6: Incremental Update Performance
fn bench_incremental_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_updates");
    
    group.bench_function("single_file_update", |b| {
        b.iter(|| {
            update_single_file(black_box("test.rs"), black_box("updated content"))
        });
    });
    
    group.bench_function("batch_update_10", |b| {
        let files: Vec<_> = (0..10).map(|i| format!("file_{}.rs", i)).collect();
        b.iter(|| {
            batch_update_files(black_box(&files))
        });
    });
    
    group.bench_function("batch_update_100", |b| {
        let files: Vec<_> = (0..100).map(|i| format!("file_{}.rs", i)).collect();
        b.iter(|| {
            batch_update_files(black_box(&files))
        });
    });
    
    group.finish();
}

// BENCHMARK 7: Parallel Processing
fn bench_parallel_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_processing");
    
    for num_threads in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("threads", num_threads),
            num_threads,
            |b, &threads| {
                b.iter(|| {
                    parallel_process(black_box(1000), black_box(threads))
                });
            }
        );
    }
    group.finish();
}

// BENCHMARK 8: Query Complexity
fn bench_query_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_complexity");
    
    // Simple keyword search
    group.bench_function("keyword_search", |b| {
        b.iter(|| {
            keyword_search(black_box("function"))
        });
    });
    
    // Semantic search
    group.bench_function("semantic_search", |b| {
        b.iter(|| {
            semantic_search(black_box("find all error handling functions"))
        });
    });
    
    // Complex multi-filter search
    group.bench_function("complex_search", |b| {
        b.iter(|| {
            complex_search(
                black_box("async functions"),
                black_box(&["src/", "tests/"]),
                black_box(&["rs", "ts"])
            )
        });
    });
    
    group.finish();
}

// BENCHMARK 9: Compression Performance
fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
    
    let data = "a".repeat(10000);
    
    group.bench_function("compress", |b| {
        b.iter(|| {
            compress_data(black_box(&data))
        });
    });
    
    let compressed = compress_data(&data);
    group.bench_function("decompress", |b| {
        b.iter(|| {
            decompress_data(black_box(&compressed))
        });
    });
    
    group.finish();
}

// BENCHMARK 10: End-to-End Pipeline
fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");
    group.measurement_time(Duration::from_secs(20));
    
    group.bench_function("small_codebase_100_files", |b| {
        let files = generate_test_files(100);
        b.iter(|| {
            full_pipeline(black_box(&files))
        });
    });
    
    group.bench_function("medium_codebase_1000_files", |b| {
        let files = generate_test_files(1000);
        b.iter(|| {
            full_pipeline(black_box(&files))
        });
    });
    
    group.finish();
}

// Helper functions for benchmarks
fn generate_embedding(text: &str) -> Vec<f32> {
    vec![0.1; 1536] // Simulate 1536-dim embedding
}

fn create_test_index(size: usize) -> HashMap<String, Vec<f32>> {
    let mut index = HashMap::new();
    for i in 0..size {
        index.insert(format!("doc_{}", i), vec![0.1; 100]);
    }
    index
}

fn generate_query(complexity: &str) -> String {
    match complexity {
        "simple" => "test".to_string(),
        "medium" => "find function implementation".to_string(),
        "complex" => "async error handling with retry logic in database connections".to_string(),
        _ => "default query".to_string(),
    }
}

fn search_index(index: &HashMap<String, Vec<f32>>, query: &str) -> Vec<(String, f32)> {
    // Simulate search
    index.iter()
        .take(10)
        .map(|(k, _)| (k.clone(), 0.5))
        .collect()
}

fn generate_test_files(num: usize) -> Vec<String> {
    (0..num).map(|i| format!("file_{}.rs", i)).collect()
}

fn index_files(files: &[String]) -> usize {
    files.len() // Simulate indexing
}

fn allocate_data(size: usize) -> Vec<u8> {
    vec![0; size]
}

fn process_data(data: &[u8]) -> usize {
    data.len()
}

fn update_single_file(filename: &str, content: &str) -> bool {
    true // Simulate update
}

fn batch_update_files(files: &[String]) -> usize {
    files.len()
}

fn parallel_process(items: usize, threads: usize) -> usize {
    items / threads // Simulate parallel processing
}

fn keyword_search(keyword: &str) -> Vec<String> {
    vec![keyword.to_string()]
}

fn semantic_search(query: &str) -> Vec<String> {
    vec![query.to_string()]
}

fn complex_search(query: &str, paths: &[&str], extensions: &[&str]) -> Vec<String> {
    vec![query.to_string()]
}

fn compress_data(data: &str) -> Vec<u8> {
    data.bytes().collect() // Simulate compression
}

fn decompress_data(data: &[u8]) -> String {
    String::from_utf8_lossy(data).to_string()
}

fn full_pipeline(files: &[String]) -> usize {
    // Simulate full indexing + search pipeline
    files.len() * 10 // chunks
}

criterion_group!(
    benches,
    bench_embedding_generation,
    bench_search_performance,
    bench_indexing_speed,
    bench_cache_performance,
    bench_memory_usage,
    bench_incremental_updates,
    bench_parallel_processing,
    bench_query_complexity,
    bench_compression,
    bench_end_to_end
);

criterion_main!(benches);
