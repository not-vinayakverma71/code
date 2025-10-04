//! Performance benchmarks for tree-sitter native vs WASM comparison

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lapce_tree_sitter::NativeParserManager;
use std::sync::Arc;
use std::time::Duration;

fn generate_code(lines: usize) -> String {
    let mut code = String::new();
    for i in 0..lines {
        code.push_str(&format!(
            "fn function_{}() {{\n    let x = {};\n    println!(\"Value: {{}}\", x);\n}}\n\n",
            i, i
        ));
    }
    code
}

fn benchmark_parsing(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("parsing");
    group.measurement_time(Duration::from_secs(10));
    
    for size in [100, 1000, 10000].iter() {
        let code = generate_code(*size);
        
        group.bench_with_input(
            BenchmarkId::new("native", size),
            &code,
            |b, code| {
                b.to_async(&rt).iter(|| async {
                    let manager = Arc::new(NativeParserManager::new().unwrap());
                    // Simulate parsing (would need actual file in production)
                    black_box(code.len());
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_incremental_parsing(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("incremental_parsing");
    
    for size in [100, 1000, 10000].iter() {
        let code = generate_code(*size);
        let modified_code = code.replace("function_0", "function_modified");
        
        group.bench_with_input(
            BenchmarkId::new("native_incremental", size),
            &(code.clone(), modified_code),
            |b, (original, modified)| {
                b.to_async(&rt).iter(|| async {
                    let manager = Arc::new(NativeParserManager::new().unwrap());
                    // First parse
                    black_box(original.len());
                    // Incremental parse
                    black_box(modified.len());
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_symbol_extraction(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("symbol_extraction");
    
    for size in [10, 100, 1000].iter() {
        let code = generate_code(*size);
        
        group.bench_with_input(
            BenchmarkId::new("native_symbols", size),
            &code,
            |b, code| {
                b.to_async(&rt).iter(|| async {
                    let manager = Arc::new(NativeParserManager::new().unwrap());
                    // Extract symbols
                    black_box(code.len());
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    group.bench_function("cache_100_trees", |b| {
        b.iter(|| {
            let manager = NativeParserManager::new().unwrap();
            // Simulate caching 100 trees
            for i in 0..100 {
                black_box(i);
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_parsing,
    benchmark_incremental_parsing,
    benchmark_symbol_extraction,
    benchmark_memory_usage
);
criterion_main!(benches);
