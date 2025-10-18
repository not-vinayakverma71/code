// CST Memory Profiling Benchmark
// Target: ≤10MB aggregate idle footprint

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use std::fs;
use tempfile::TempDir;

#[cfg(target_os = "linux")]
fn get_memory_usage() -> Option<usize> {
    use std::fs::read_to_string;
    let status = read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse().ok();
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn get_memory_usage() -> Option<usize> {
    None
}

fn generate_sample(lines: usize) -> String {
    let mut code = String::new();
    for i in 0..lines {
        code.push_str(&format!("fn func_{}() {{\n", i));
        code.push_str("    let x = 42;\n");
        code.push_str("}\n\n");
    }
    code
}

/// Benchmark memory usage for idle parser
fn bench_idle_memory(c: &mut Criterion) {
    let baseline = get_memory_usage();
    
    let _pipeline = CstToAstPipeline::new();
    
    let after_init = get_memory_usage();
    
    if let (Some(before), Some(after)) = (baseline, after_init) {
        let delta = after.saturating_sub(before);
        println!("\n=== Idle Memory Usage ===");
        println!("Baseline: {} KB", before);
        println!("After init: {} KB", after);
        println!("Delta: {} KB ({:.2} MB)", delta, delta as f64 / 1024.0);
        println!("Target: ≤10 MB");
        
        if delta as f64 / 1024.0 > 10.0 {
            println!("⚠️  WARNING: Memory usage exceeds 10MB target");
        } else {
            println!("✅ Memory usage within target");
        }
    }
    
    c.bench_function("idle_memory", |b| {
        b.iter(|| {
            black_box(&_pipeline);
        });
    });
}

/// Benchmark memory usage during parsing
fn bench_parse_memory(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    let sample = generate_sample(1000);
    fs::write(&file_path, &sample).unwrap();
    
    let baseline = get_memory_usage();
    let pipeline = CstToAstPipeline::new();
    
    c.bench_function("parse_memory", |b| {
        b.iter(|| {
            black_box(pipeline.process_file(&file_path).unwrap());
        });
    });
    
    let after_parse = get_memory_usage();
    
    if let (Some(before), Some(after)) = (baseline, after_parse) {
        let delta = after.saturating_sub(before);
        println!("\n=== Parse Memory Usage ===");
        println!("Baseline: {} KB", before);
        println!("After parse (1000 lines): {} KB", after);
        println!("Delta: {} KB ({:.2} MB)", delta, delta as f64 / 1024.0);
    }
}

/// Benchmark memory usage for cached parsers
fn bench_cached_memory(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let pipeline = CstToAstPipeline::new();
    
    // Create multiple files and parse them
    let files: Vec<_> = (0..10).map(|i| {
        let path = temp_dir.path().join(format!("test{}.rs", i));
        let sample = generate_sample(100);
        fs::write(&path, &sample).unwrap();
        path
    }).collect();
    
    let baseline = get_memory_usage();
    
    // Parse all files (will populate cache)
    for path in &files {
        let _ = pipeline.process_file(path);
    }
    
    let after_cache = get_memory_usage();
    
    if let (Some(before), Some(after)) = (baseline, after_cache) {
        let delta = after.saturating_sub(before);
        println!("\n=== Cached Memory Usage ===");
        println!("Baseline: {} KB", before);
        println!("After caching 10 files: {} KB", after);
        println!("Delta: {} KB ({:.2} MB)", delta, delta as f64 / 1024.0);
        println!("Per file: {:.2} KB", delta as f64 / 10.0);
    }
    
    c.bench_function("cached_memory", |b| {
        b.iter(|| {
            for path in &files {
                black_box(pipeline.process_file(path).unwrap());
            }
        });
    });
}

/// Benchmark memory growth over many parses
fn bench_memory_growth(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let pipeline = CstToAstPipeline::new();
    
    let baseline = get_memory_usage();
    
    c.bench_function("memory_growth_100_files", |b| {
        b.iter(|| {
            for i in 0..100 {
                let path = temp_dir.path().join(format!("growth{}.rs", i));
                let sample = generate_sample(50);
                fs::write(&path, &sample).unwrap();
                black_box(pipeline.process_file(&path).unwrap());
            }
        });
    });
    
    let after_growth = get_memory_usage();
    
    if let (Some(before), Some(after)) = (baseline, after_growth) {
        let delta = after.saturating_sub(before);
        println!("\n=== Memory Growth (100 files) ===");
        println!("Baseline: {} KB", before);
        println!("After 100 parses: {} KB", after);
        println!("Delta: {} KB ({:.2} MB)", delta, delta as f64 / 1024.0);
        println!("Growth per file: {:.2} KB", delta as f64 / 100.0);
    }
}

criterion_group!(
    benches,
    bench_idle_memory,
    bench_parse_memory,
    bench_cached_memory,
    bench_memory_growth
);

criterion_main!(benches);
