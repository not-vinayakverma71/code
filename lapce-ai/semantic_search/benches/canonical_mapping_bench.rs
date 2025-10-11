// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Performance benchmarks for canonical mapping

#[cfg(feature = "cst_ts")]
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use std::path::Path;
use std::io::Write;
use tempfile::NamedTempFile;

#[cfg(feature = "cst_ts")]
fn benchmark_parse_rust(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_rust");
    
    let small_code = r#"
fn main() {
    println!("Hello, world!");
}
"#;
    
    let medium_code = r#"
use std::collections::HashMap;

struct Config {
    settings: HashMap<String, String>,
}

impl Config {
    fn new() -> Self {
        Self { settings: HashMap::new() }
    }
    
    fn get(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }
    
    fn set(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }
}

fn main() {
    let mut config = Config::new();
    config.set("name".to_string(), "test".to_string());
    
    if let Some(name) = config.get("name") {
        println!("Name: {}", name);
    }
}
"#;

    let large_code = medium_code.repeat(10);
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pipeline = CstToAstPipeline::new();
    
    for (size_name, code) in &[("small", small_code), ("medium", medium_code), ("large", &large_code)] {
        group.bench_with_input(BenchmarkId::from_parameter(size_name), code, |b, code| {
            b.to_async(&rt).iter(|| async {
                let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
                temp_file.write_all(code.as_bytes()).unwrap();
                let path = temp_file.path();
                
                let result = pipeline.process_file(black_box(path)).await;
                black_box(result)
            });
        });
    }
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
fn benchmark_parse_javascript(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_javascript");
    
    let code = r#"
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

class Calculator {
    constructor() {
        this.result = 0;
    }
    
    add(a, b) {
        this.result = a + b;
        return this.result;
    }
    
    multiply(a, b) {
        this.result = a * b;
        return this.result;
    }
}

const calc = new Calculator();
console.log(calc.add(5, 3));
"#;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pipeline = CstToAstPipeline::new();
    
    group.bench_function("javascript", |b| {
        b.to_async(&rt).iter(|| async {
            let mut temp_file = NamedTempFile::with_suffix(".js").unwrap();
            temp_file.write_all(code.as_bytes()).unwrap();
            let path = temp_file.path();
            
            let result = pipeline.process_file(black_box(path)).await;
            black_box(result)
        });
    });
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
fn benchmark_parse_python(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_python");
    
    let code = r#"
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

class Calculator:
    def __init__(self):
        self.result = 0
    
    def add(self, a, b):
        self.result = a + b
        return self.result
    
    def multiply(self, a, b):
        self.result = a * b
        return self.result

if __name__ == "__main__":
    calc = Calculator()
    print(calc.add(5, 3))
"#;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pipeline = CstToAstPipeline::new();
    
    group.bench_function("python", |b| {
        b.to_async(&rt).iter(|| async {
            let mut temp_file = NamedTempFile::with_suffix(".py").unwrap();
            temp_file.write_all(code.as_bytes()).unwrap();
            let path = temp_file.path();
            
            let result = pipeline.process_file(black_box(path)).await;
            black_box(result)
        });
    });
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
fn benchmark_cross_language(c: &mut Criterion) {
    let mut group = c.benchmark_group("cross_language");
    
    let codes = vec![
        ("rust", ".rs", "fn add(x: i32, y: i32) -> i32 { x + y }"),
        ("javascript", ".js", "function add(x, y) { return x + y; }"),
        ("python", ".py", "def add(x, y):\n    return x + y"),
        ("go", ".go", "func add(x int, y int) int { return x + y }"),
        ("java", ".java", "public int add(int x, int y) { return x + y; }"),
    ];
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pipeline = CstToAstPipeline::new();
    
    for (lang, ext, code) in codes {
        group.bench_with_input(BenchmarkId::from_parameter(lang), &(ext, code), |b, (ext, code)| {
            b.to_async(&rt).iter(|| async {
                let mut temp_file = NamedTempFile::with_suffix(ext).unwrap();
                temp_file.write_all(code.as_bytes()).unwrap();
                let path = temp_file.path();
                
                let result = pipeline.process_file(black_box(path)).await;
                black_box(result)
            });
        });
    }
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
criterion_group!(
    benches,
    benchmark_parse_rust,
    benchmark_parse_javascript,
    benchmark_parse_python,
    benchmark_cross_language
);

#[cfg(feature = "cst_ts")]
criterion_main!(benches);

#[cfg(not(feature = "cst_ts"))]
fn main() {
    eprintln!("Error: Benchmarks require the 'cst_ts' feature.");
    eprintln!("Run with: cargo bench --features cst_ts");
}
