// CST Parser Performance Benchmarks
// Target: ≥10K lines/second throughput

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use std::path::Path;
use tempfile::TempDir;
use std::fs;

/// Generate synthetic code samples of varying sizes
fn generate_rust_sample(lines: usize) -> String {
    let mut code = String::new();
    code.push_str("// Generated Rust code\n");
    for i in 0..lines {
        code.push_str(&format!("fn function_{}() {{\n", i));
        code.push_str("    let x = 42;\n");
        code.push_str("    let y = x * 2;\n");
        code.push_str("    println!(\"{{}}\" , y);\n");
        code.push_str("}\n\n");
    }
    code
}

fn generate_javascript_sample(lines: usize) -> String {
    let mut code = String::new();
    code.push_str("// Generated JavaScript code\n");
    for i in 0..lines {
        code.push_str(&format!("function func_{}() {{\n", i));
        code.push_str("    const x = 42;\n");
        code.push_str("    const y = x * 2;\n");
        code.push_str("    console.log(y);\n");
        code.push_str("}\n\n");
    }
    code
}

fn generate_python_sample(lines: usize) -> String {
    let mut code = String::new();
    code.push_str("# Generated Python code\n");
    for i in 0..lines {
        code.push_str(&format!("def func_{}():\n", i));
        code.push_str("    x = 42\n");
        code.push_str("    y = x * 2\n");
        code.push_str("    print(y)\n\n");
    }
    code
}

/// Benchmark parser throughput for single language
fn bench_parse_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_throughput");
    
    let sizes = vec![100, 500, 1000, 5000, 10000];
    
    for size in sizes {
        let sample = generate_rust_sample(size);
        let line_count = sample.lines().count();
        
        group.throughput(Throughput::Elements(line_count as u64));
        group.bench_with_input(
            BenchmarkId::new("rust", line_count),
            &sample,
            |b, sample| {
                let temp_dir = TempDir::new().unwrap();
                let file_path = temp_dir.path().join("test.rs");
                fs::write(&file_path, sample).unwrap();
                let pipeline = CstToAstPipeline::new();
                
                b.iter(|| {
                    black_box(pipeline.process_file(&file_path).unwrap());
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark multi-language parsing throughput
fn bench_multilang_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("multilang_throughput");
    
    let languages = vec![
        ("rust", "rs", generate_rust_sample(1000)),
        ("javascript", "js", generate_javascript_sample(1000)),
        ("python", "py", generate_python_sample(1000)),
    ];
    
    for (lang_name, ext, sample) in languages {
        let line_count = sample.lines().count();
        
        group.throughput(Throughput::Elements(line_count as u64));
        group.bench_with_input(
            BenchmarkId::new("parse", lang_name),
            &(ext, sample),
            |b, (ext, sample)| {
                let temp_dir = TempDir::new().unwrap();
                let file_path = temp_dir.path().join(format!("test.{}", ext));
                fs::write(&file_path, sample).unwrap();
                let pipeline = CstToAstPipeline::new();
                
                b.iter(|| {
                    black_box(pipeline.process_file(&file_path).unwrap());
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark parser throughput for mixed codebase
fn bench_mixed_codebase(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let pipeline = CstToAstPipeline::new();
    
    // Create mixed language files
    let files = vec![
        ("test1.rs", generate_rust_sample(500)),
        ("test2.js", generate_javascript_sample(500)),
        ("test3.py", generate_python_sample(500)),
        ("test4.rs", generate_rust_sample(500)),
        ("test5.js", generate_javascript_sample(500)),
    ];
    
    let mut total_lines = 0;
    let file_paths: Vec<_> = files.iter().map(|(name, content)| {
        let path = temp_dir.path().join(name);
        fs::write(&path, content).unwrap();
        total_lines += content.lines().count();
        path
    }).collect();
    
    c.bench_function("mixed_codebase", |b| {
        b.iter(|| {
            for path in &file_paths {
                black_box(pipeline.process_file(path).unwrap());
            }
        });
    });
    
    println!("\n=== Mixed Codebase Benchmark ===");
    println!("Total files: {}", files.len());
    println!("Total lines: {}", total_lines);
    println!("Target: ≥10,000 lines/sec");
}

/// Benchmark cache effectiveness
fn bench_cache_effectiveness(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    let sample = generate_rust_sample(1000);
    fs::write(&file_path, &sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    
    // First parse to warm cache
    let _ = pipeline.process_file(&file_path);
    
    c.bench_function("cached_parse", |b| {
        b.iter(|| {
            black_box(pipeline.process_file(&file_path).unwrap());
        });
    });
}

criterion_group!(
    benches,
    bench_parse_throughput,
    bench_multilang_throughput,
    bench_mixed_codebase,
    bench_cache_effectiveness
);

criterion_main!(benches);
