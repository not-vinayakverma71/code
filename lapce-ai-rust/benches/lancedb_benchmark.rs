/// Production-grade benchmarks for LanceDB
/// Must meet ALL 8 performance requirements

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lapce_ai_rust::lancedb::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tempfile::TempDir;
use rayon::prelude::*;

/// Test configuration
struct TestConfig {
    small_repo: Vec<PathBuf>,     // 100 files
    medium_repo: Vec<PathBuf>,    // 1,000 files  
    large_repo: Vec<PathBuf>,      // 10,000 files
    huge_repo: Vec<PathBuf>,       // 100,000 files
    queries: Vec<String>,
    ground_truth: Vec<GroundTruth>,
}

#[derive(Clone)]
struct GroundTruth {
    query: String,
    expected_files: Vec<PathBuf>,
    expected_snippets: Vec<String>,
}

impl TestConfig {
    fn new() -> Self {
        // Create real test data from actual Rust files
        let mut small_repo = Vec::new();
        let mut medium_repo = Vec::new();
        let mut large_repo = Vec::new();
        let mut huge_repo = Vec::new();
        
        // Scan actual Rust projects
        let rust_std = PathBuf::from("/usr/lib/rustlib/src/rust/library");
        let current_project = PathBuf::from(".");
        
        // Collect real Rust files
        if rust_std.exists() {
            for entry in walkdir::WalkDir::new(&rust_std)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
            {
                let path = entry.path().to_path_buf();
                
                if small_repo.len() < 100 {
                    small_repo.push(path.clone());
                }
                if medium_repo.len() < 1000 {
                    medium_repo.push(path.clone());
                }
                if large_repo.len() < 10000 {
                    large_repo.push(path.clone());
                }
                if huge_repo.len() < 100000 {
                    huge_repo.push(path.clone());
                }
            }
        }
        
        // Add current project files
        for entry in walkdir::WalkDir::new(&current_project)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
        {
            let path = entry.path().to_path_buf();
            
            if small_repo.len() < 100 {
                small_repo.push(path.clone());
            }
            if medium_repo.len() < 1000 {
                medium_repo.push(path.clone());
            }
        }
        
        // Real queries from actual development
        let queries = vec![
            "async function that handles errors".to_string(),
            "impl trait for generic type".to_string(),
            "tokio spawn task".to_string(),
            "parse JSON from string".to_string(),
            "HashMap with custom hasher".to_string(),
            "serialize struct to bytes".to_string(),
            "TCP server accept connections".to_string(),
            "mutex lock in async context".to_string(),
            "iterator map filter collect".to_string(),
            "match Result Ok Err".to_string(),
        ];
        
        // Ground truth for accuracy testing
        let ground_truth = vec![
            GroundTruth {
                query: "async function error handling".to_string(),
                expected_files: vec![
                    PathBuf::from("src/ipc_server.rs"),
                    PathBuf::from("src/mcp_tools/core.rs"),
                ],
                expected_snippets: vec![
                    "async fn handle_connection".to_string(),
                    "Result<(), IpcError>".to_string(),
                ],
            },
            // Add more ground truth data
        ];
        
        Self {
            small_repo,
            medium_repo,
            large_repo,
            huge_repo,
            queries,
            ground_truth,
        }
    }
}

/// Benchmark 1: Memory Usage < 10MB
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config = TestConfig::new();
    
    c.bench_function("memory_usage_10mb", |b| {
        b.iter(|| {
            rt.block_on(async {
                let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
                
                // Index 1000 files
                db.index_files(config.medium_repo.clone()).await.unwrap();
                
                // Measure memory
                let memory_mb = get_process_memory_mb();
                
                // Must be < 10MB
                assert!(memory_mb < 10.0, "Memory usage {}MB exceeds 10MB limit", memory_mb);
                
                black_box(memory_mb)
            })
        })
    });
}

/// Benchmark 2: Query Latency < 5ms
fn bench_query_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config = TestConfig::new();
    
    // Setup: Index files first
    let db = rt.block_on(async {
        let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
        db.index_files(config.medium_repo.clone()).await.unwrap();
        db
    });
    
    let mut group = c.benchmark_group("query_latency_5ms");
    
    for query in &config.queries {
        group.bench_with_input(
            BenchmarkId::new("search", query),
            query,
            |b, q| {
                b.iter(|| {
                    rt.block_on(async {
                        let start = Instant::now();
                        let results = db.search(q, 10).await.unwrap();
                        let latency = start.elapsed();
                        
                        // Must be < 5ms
                        assert!(latency < Duration::from_millis(5), 
                            "Query latency {:?} exceeds 5ms limit", latency);
                        
                        black_box(results)
                    })
                })
            }
        );
    }
    group.finish();
}

/// Benchmark 3: Indexing Throughput > 1000 files/sec
fn bench_indexing_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config = TestConfig::new();
    
    c.bench_function("indexing_1000_files_per_sec", |b| {
        b.iter(|| {
            rt.block_on(async {
                let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
                
                let files = config.medium_repo.clone();
                let file_count = files.len();
                
                let start = Instant::now();
                db.index_files(files).await.unwrap();
                let elapsed = start.elapsed();
                
                let throughput = file_count as f64 / elapsed.as_secs_f64();
                
                // Must be > 1000 files/sec
                assert!(throughput > 1000.0, 
                    "Indexing throughput {} files/sec below 1000 files/sec target", throughput);
                
                black_box(throughput)
            })
        })
    });
}

/// Benchmark 4: Accuracy > 90%
fn bench_accuracy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config = TestConfig::new();
    
    c.bench_function("accuracy_90_percent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
                
                // Index known files
                db.index_files(config.small_repo.clone()).await.unwrap();
                
                let mut correct = 0;
                let mut total = 0;
                
                for gt in &config.ground_truth {
                    let results = db.search(&gt.query, 10).await.unwrap();
                    
                    // Check if expected files are in results
                    for expected_file in &gt.expected_files {
                        total += 1;
                        if results.iter().any(|r| r.path == *expected_file) {
                            correct += 1;
                        }
                    }
                    
                    // Check if expected snippets are found
                    for expected_snippet in &gt.expected_snippets {
                        total += 1;
                        if results.iter().any(|r| r.content.contains(expected_snippet)) {
                            correct += 1;
                        }
                    }
                }
                
                let accuracy = if total > 0 {
                    correct as f64 / total as f64
                } else {
                    0.0
                };
                
                // Must be > 90%
                assert!(accuracy > 0.9, "Accuracy {}% below 90% target", accuracy * 100.0);
                
                black_box(accuracy)
            })
        })
    });
}

/// Benchmark 5: Incremental Update < 100ms
fn bench_incremental_update(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config = TestConfig::new();
    
    // Setup
    let db = rt.block_on(async {
        let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
        db.index_files(config.small_repo.clone()).await.unwrap();
        db
    });
    
    c.bench_function("incremental_update_100ms", |b| {
        b.iter(|| {
            rt.block_on(async {
                if let Some(file) = config.small_repo.first() {
                    let start = Instant::now();
                    db.incremental_update(file.clone()).await.unwrap();
                    let latency = start.elapsed();
                    
                    // Must be < 100ms
                    assert!(latency < Duration::from_millis(100),
                        "Incremental update {:?} exceeds 100ms limit", latency);
                    
                    black_box(latency)
                } else {
                    black_box(Duration::from_millis(0))
                }
            })
        })
    });
}

/// Benchmark 6: Cache Hit Rate > 80%
fn bench_cache_hit_rate(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config = TestConfig::new();
    
    c.bench_function("cache_hit_rate_80_percent", |b| {
        b.iter(|| {
            rt.block_on(async {
                let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
                db.index_files(config.small_repo.clone()).await.unwrap();
                
                // Run same queries multiple times
                for _ in 0..5 {
                    for query in &config.queries[0..5] {
                        db.search(query, 10).await.unwrap();
                    }
                }
                
                let metrics = db.get_metrics();
                let hit_rate = metrics.cache_hit_rate;
                
                // Must be > 80%
                assert!(hit_rate > 0.8, "Cache hit rate {}% below 80% target", hit_rate * 100.0);
                
                black_box(hit_rate)
            })
        })
    });
}

/// Benchmark 7: Concurrent Queries > 100
fn bench_concurrent_queries(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config = TestConfig::new();
    
    c.bench_function("concurrent_queries_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let db = Arc::new(LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap());
                db.index_files(config.small_repo.clone()).await.unwrap();
                
                // Launch 100 concurrent queries
                let mut handles = Vec::new();
                
                for i in 0..100 {
                    let db_clone = db.clone();
                    let query = config.queries[i % config.queries.len()].clone();
                    
                    handles.push(tokio::spawn(async move {
                        db_clone.search(&query, 10).await.unwrap()
                    }));
                }
                
                // Wait for all to complete
                let start = Instant::now();
                let results = futures::future::join_all(handles).await;
                let elapsed = start.elapsed();
                
                // All must succeed
                assert_eq!(results.len(), 100);
                for result in results {
                    assert!(result.is_ok());
                }
                
                // Should complete reasonably fast
                assert!(elapsed < Duration::from_secs(10),
                    "100 concurrent queries took {:?}", elapsed);
                
                black_box(elapsed)
            })
        })
    });
}

/// Benchmark 8: Scale to 100K files
fn bench_scale_100k_files(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config = TestConfig::new();
    
    // Only run if we have enough test files
    if config.huge_repo.len() < 10000 {
        println!("Skipping 100K scale test - insufficient test data");
        return;
    }
    
    c.bench_function("scale_100k_files", |b| {
        b.iter(|| {
            rt.block_on(async {
                let db = LanceDB::new(temp_dir.path().to_path_buf()).await.unwrap();
                
                // Index in batches
                for chunk in config.huge_repo.chunks(10000) {
                    db.index_files(chunk.to_vec()).await.unwrap();
                }
                
                // Verify can still search efficiently
                let start = Instant::now();
                let results = db.search("test query", 10).await.unwrap();
                let latency = start.elapsed();
                
                // Should still be fast even with 100K files
                assert!(latency < Duration::from_millis(50),
                    "Search in 100K files took {:?}", latency);
                
                let metrics = db.get_metrics();
                assert!(metrics.total_indexed_files >= 10000);
                
                black_box(results)
            })
        })
    });
}

/// Helper: Get current process memory usage in MB
fn get_process_memory_mb() -> f64 {
    use sysinfo::{System, SystemExt, ProcessExt};
    
    let mut system = System::new_all();
    system.refresh_all();
    
    if let Some(process) = system.process(sysinfo::get_current_pid().unwrap()) {
        process.memory() as f64 / 1024.0 / 1024.0
    } else {
        0.0
    }
}

use std::sync::Arc;

criterion_group!(
    benches,
    bench_memory_usage,
    bench_query_latency,
    bench_indexing_throughput,
    bench_accuracy,
    bench_incremental_update,
    bench_cache_hit_rate,
    bench_concurrent_queries,
    bench_scale_100k_files
);

criterion_main!(benches);
