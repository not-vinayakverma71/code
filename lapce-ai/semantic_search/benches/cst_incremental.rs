// CST Incremental Edit Latency Benchmark
// Target: <10ms for small edits with stable IDs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use std::fs;
use tempfile::TempDir;

/// Generate base code sample
fn generate_base_sample() -> String {
    r#"
fn main() {
    let x = 42;
    let y = x * 2;
    println!("Result: {}", y);
}

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
    
    fn distance(&self) -> f64 {
        ((self.x * self.x + self.y * self.y) as f64).sqrt()
    }
}
"#.to_string()
}

/// Apply small edit: single line change
fn apply_small_edit(code: &str) -> String {
    code.replace("let x = 42;", "let x = 43;")
}

/// Apply medium edit: function body change
fn apply_medium_edit(code: &str) -> String {
    code.replace(
        "fn distance(&self) -> f64 {\n        ((self.x * self.x + self.y * self.y) as f64).sqrt()\n    }",
        "fn distance(&self) -> f64 {\n        let dx = self.x as f64;\n        let dy = self.y as f64;\n        (dx * dx + dy * dy).sqrt()\n    }"
    )
}

/// Apply large edit: add new struct
fn apply_large_edit(code: &str) -> String {
    format!("{}\n\nstruct Circle {{\n    radius: f64,\n}}\n\nimpl Circle {{\n    fn area(&self) -> f64 {{\n        std::f64::consts::PI * self.radius * self.radius\n    }}\n}}", code)
}

/// Benchmark small incremental edits
fn bench_small_edit_latency(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    let pipeline = CstToAstPipeline::new();
    
    let base_code = generate_base_sample();
    fs::write(&file_path, &base_code).unwrap();
    
    // Initial parse
    let _ = pipeline.process_file(&file_path);
    
    c.bench_function("small_edit", |b| {
        b.iter(|| {
            let edited = apply_small_edit(&base_code);
            fs::write(&file_path, &edited).unwrap();
            black_box(pipeline.process_file(&file_path).unwrap());
        });
    });
}

/// Benchmark medium incremental edits
fn bench_medium_edit_latency(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    let pipeline = CstToAstPipeline::new();
    
    let base_code = generate_base_sample();
    fs::write(&file_path, &base_code).unwrap();
    
    // Initial parse
    let _ = pipeline.process_file(&file_path);
    
    c.bench_function("medium_edit", |b| {
        b.iter(|| {
            let edited = apply_medium_edit(&base_code);
            fs::write(&file_path, &edited).unwrap();
            black_box(pipeline.process_file(&file_path).unwrap());
        });
    });
}

/// Benchmark large incremental edits
fn bench_large_edit_latency(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    let pipeline = CstToAstPipeline::new();
    
    let base_code = generate_base_sample();
    fs::write(&file_path, &base_code).unwrap();
    
    // Initial parse
    let _ = pipeline.process_file(&file_path);
    
    c.bench_function("large_edit", |b| {
        b.iter(|| {
            let edited = apply_large_edit(&base_code);
            fs::write(&file_path, &edited).unwrap();
            black_box(pipeline.process_file(&file_path).unwrap());
        });
    });
}

/// Benchmark sequential edits (simulating typing)
fn bench_sequential_edits(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    let pipeline = CstToAstPipeline::new();
    
    let mut code = generate_base_sample();
    fs::write(&file_path, &code).unwrap();
    
    // Initial parse
    let _ = pipeline.process_file(&file_path);
    
    c.bench_function("sequential_edits_10", |b| {
        b.iter(|| {
            for i in 0..10 {
                code = code.replace(&format!("x = {};", 42 + i), &format!("x = {};", 43 + i));
                fs::write(&file_path, &code).unwrap();
                black_box(pipeline.process_file(&file_path).unwrap());
            }
        });
    });
}

criterion_group!(
    benches,
    bench_small_edit_latency,
    bench_medium_edit_latency,
    bench_large_edit_latency,
    bench_sequential_edits
);

criterion_main!(benches);
