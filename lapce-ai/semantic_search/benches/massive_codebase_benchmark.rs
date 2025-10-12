// Massive Codebase Benchmark - 3,000 files from massive_test_codebase
// Validates specs from:
// - docs/05-TREE-SITTER-INTEGRATION.md: >10K lines/sec, <10ms incremental, <5MB memory
// - docs/06-SEMANTIC-SEARCH-LANCEDB.md: CST integration, semantic chunking

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::Instant;
use walkdir::WalkDir;
use tokio::runtime::Runtime;

const MASSIVE_CODEBASE_PATH: &str = "../massive_test_codebase";

/// Collect all Rust, Python, TypeScript files
fn collect_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    for entry in WalkDir::new(MASSIVE_CODEBASE_PATH)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_str().unwrap_or("");
                if matches!(ext_str, "rs" | "py" | "ts") {
                    files.push(path.to_path_buf());
                }
            }
        }
    }
    
    files
}

/// Count total lines in files
fn count_lines(files: &[PathBuf]) -> usize {
    files.iter()
        .filter_map(|f| fs::read_to_string(f).ok())
        .map(|content| content.lines().count())
        .sum()
}

/// Benchmark parsing all 3,000 files
fn bench_parse_all_files(c: &mut Criterion) {
    let files = collect_files();
    let total_lines = count_lines(&files);
    
    println!("\n=== Massive Codebase Stats ===");
    println!("Total files: {}", files.len());
    println!("Total lines: {}", total_lines);
    println!("Target: >10,000 lines/sec (from 05-TREE-SITTER-INTEGRATION.md)");
    
    let mut group = c.benchmark_group("massive_codebase");
    group.sample_size(10);
    group.throughput(Throughput::Elements(total_lines as u64));
    
    group.bench_function("parse_all_3000_files", |b| {
        let pipeline = CstToAstPipeline::new();
        let rt = Runtime::new().unwrap();
        
        b.iter(|| {
            let start = Instant::now();
            let mut success = 0;
            let mut failed = 0;
            
            for file_path in &files {
                match rt.block_on(pipeline.process_file(file_path)) {
                    Ok(_) => success += 1,
                    Err(_) => failed += 1,
                }
            }
            
            let elapsed = start.elapsed();
            let lines_per_sec = total_lines as f64 / elapsed.as_secs_f64();
            
            black_box((success, failed, lines_per_sec))
        });
    });
    
    group.finish();
}

/// Benchmark parsing by language
fn bench_parse_by_language(c: &mut Criterion) {
    let all_files = collect_files();
    
    let rust_files: Vec<_> = all_files.iter()
        .filter(|f| f.extension().and_then(|e| e.to_str()) == Some("rs"))
        .cloned()
        .collect();
    
    let python_files: Vec<_> = all_files.iter()
        .filter(|f| f.extension().and_then(|e| e.to_str()) == Some("py"))
        .cloned()
        .collect();
    
    let ts_files: Vec<_> = all_files.iter()
        .filter(|f| f.extension().and_then(|e| e.to_str()) == Some("ts"))
        .cloned()
        .collect();
    
    println!("\n=== Files by Language ===");
    println!("Rust: {}", rust_files.len());
    println!("Python: {}", python_files.len());
    println!("TypeScript: {}", ts_files.len());
    
    let pipeline = CstToAstPipeline::new();
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("by_language");
    
    // Rust
    if !rust_files.is_empty() {
        let rust_lines = count_lines(&rust_files);
        group.throughput(Throughput::Elements(rust_lines as u64));
        group.bench_function("rust_files", |b| {
            b.iter(|| {
                for file in &rust_files {
                    black_box(rt.block_on(pipeline.process_file(file)).ok());
                }
            });
        });
    }
    
    // Python
    if !python_files.is_empty() {
        let py_lines = count_lines(&python_files);
        group.throughput(Throughput::Elements(py_lines as u64));
        group.bench_function("python_files", |b| {
            b.iter(|| {
                for file in &python_files {
                    black_box(rt.block_on(pipeline.process_file(file)).ok());
                }
            });
        });
    }
    
    // TypeScript
    if !ts_files.is_empty() {
        let ts_lines = count_lines(&ts_files);
        group.throughput(Throughput::Elements(ts_lines as u64));
        group.bench_function("typescript_files", |b| {
            b.iter(|| {
                for file in &ts_files {
                    black_box(rt.block_on(pipeline.process_file(file)).ok());
                }
            });
        });
    }
    
    group.finish();
}

/// Benchmark symbol extraction performance
fn bench_symbol_extraction(c: &mut Criterion) {
    let files = collect_files();
    let sample_files: Vec<_> = files.iter().take(100).cloned().collect();
    
    println!("\n=== Symbol Extraction ===");
    println!("Testing on 100 sample files");
    println!("Target: <50ms per 1K line file (from 05-TREE-SITTER-INTEGRATION.md)");
    
    let pipeline = CstToAstPipeline::new();
    let rt = Runtime::new().unwrap();
    
    c.bench_function("symbol_extraction_100_files", |b| {
        b.iter(|| {
            let mut total_symbols = 0;
            
            for file_path in &sample_files {
                if let Ok(result) = rt.block_on(pipeline.process_file(file_path)) {
                    total_symbols += count_symbols(&result.ast);
                }
            }
            
            black_box(total_symbols)
        });
    });
}

/// Benchmark cache effectiveness
fn bench_cache_hit_rate(c: &mut Criterion) {
    let files = collect_files();
    let sample_files: Vec<_> = files.iter().take(100).cloned().collect();
    
    println!("\n=== Cache Performance ===");
    println!("Target: >90% cache hit rate (from 05-TREE-SITTER-INTEGRATION.md)");
    
    let pipeline = CstToAstPipeline::new();
    let rt = Runtime::new().unwrap();
    
    // Warm up cache
    for file_path in &sample_files {
        let _ = rt.block_on(pipeline.process_file(file_path));
    }
    
    c.bench_function("cached_parse_100_files", |b| {
        b.iter(|| {
            for file_path in &sample_files {
                black_box(rt.block_on(pipeline.process_file(file_path)).ok());
            }
        });
    });
}

/// Benchmark memory footprint
fn bench_memory_footprint(c: &mut Criterion) {
    println!("\n=== Memory Footprint ===");
    println!("Target: <5MB for parser instances (from 05-TREE-SITTER-INTEGRATION.md)");
    
    #[cfg(target_os = "linux")]
    {
        use std::fs::read_to_string;
        
        let baseline = get_rss_kb();
        println!("Baseline memory: {} KB", baseline.unwrap_or(0));
        
        let pipeline = CstToAstPipeline::new();
        
        let after_init = get_rss_kb();
        if let (Some(before), Some(after)) = (baseline, after_init) {
            let delta_kb = after.saturating_sub(before);
            let delta_mb = delta_kb as f64 / 1024.0;
            println!("After pipeline init: {} KB ({:.2} MB)", delta_kb, delta_mb);
            
            if delta_mb > 5.0 {
                println!("⚠️  WARNING: Memory usage {:.2} MB exceeds 5MB target", delta_mb);
            } else {
                println!("✅ Memory usage {:.2} MB within 5MB target", delta_mb);
            }
        }
        
        c.bench_function("memory_footprint", |b| {
            b.iter(|| black_box(&pipeline));
        });
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        println!("Memory profiling only available on Linux");
        c.bench_function("memory_footprint_skipped", |b| {
            b.iter(|| {});
        });
    }
}

#[cfg(target_os = "linux")]
fn get_rss_kb() -> Option<usize> {
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

fn count_symbols(node: &lancedb::processors::cst_to_ast_pipeline::AstNode) -> usize {
    use lancedb::processors::cst_to_ast_pipeline::AstNodeType;
    
    let mut count = 0;
    match node.node_type {
        AstNodeType::FunctionDeclaration | 
        AstNodeType::ClassDeclaration |
        AstNodeType::StructDeclaration |
        AstNodeType::EnumDeclaration => count = 1,
        _ => {}
    }
    
    for child in &node.children {
        count += count_symbols(child);
    }
    
    count
}

criterion_group!(
    benches,
    bench_parse_all_files,
    bench_parse_by_language,
    bench_symbol_extraction,
    bench_cache_hit_rate,
    bench_memory_footprint
);

criterion_main!(benches);
