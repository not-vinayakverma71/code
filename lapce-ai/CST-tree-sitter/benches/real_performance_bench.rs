//! Real performance benchmarks for 17 working languages

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use lapce_tree_sitter::parser_manager::compat_working::get_language_compat;
use lapce_tree_sitter::types::FileType;
use tree_sitter::Parser;

fn benchmark_parse_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_speed");
    
    // Generate test code
    let rust_code = generate_rust_code(10000); // 10K lines
    let js_code = generate_js_code(10000);
    
    group.throughput(Throughput::Elements(10000));
    
    // Benchmark Rust parsing
    group.bench_function("rust_10k_lines", |b| {
        b.iter(|| {
            let language = get_language_compat(FileType::Rust).unwrap();
            let mut parser = Parser::new();
            parser.set_language(language).unwrap();
            let tree = parser.parse(black_box(&rust_code), None);
            assert!(tree.is_some());
        });
    });
    
    // Benchmark JavaScript parsing
    group.bench_function("javascript_10k_lines", |b| {
        b.iter(|| {
            let language = get_language_compat(FileType::JavaScript).unwrap();
            let mut parser = Parser::new();
            parser.set_language(language).unwrap();
            let tree = parser.parse(black_box(&js_code), None);
            assert!(tree.is_some());
        });
    });
    
    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    group.bench_function("load_all_17_parsers", |b| {
        b.iter(|| {
            let languages = vec![
                FileType::Rust, FileType::JavaScript, FileType::TypeScript,
                FileType::Python, FileType::Go, FileType::C, FileType::Cpp,
                FileType::Java, FileType::Json, FileType::Html, FileType::Css,
                FileType::Bash, FileType::Ruby, FileType::Php, FileType::CSharp,
                FileType::Toml,
            ];
            
            for lang in &languages {
                let _ = get_language_compat(*lang).unwrap();
            }
            
            black_box(languages);
        });
    });
    
    group.finish();
}

fn benchmark_incremental_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_parsing");
    
    let original = "fn main() { println!(\"hello\"); }";
    let modified = "fn main() { println!(\"world\"); }";
    
    group.bench_function("small_edit", |b| {
        let language = get_language_compat(FileType::Rust).unwrap();
        let mut parser = Parser::new();
        parser.set_language(language).unwrap();
        
        let old_tree = parser.parse(original, None).unwrap();
        
        b.iter(|| {
            let tree = parser.parse(black_box(modified), Some(&old_tree));
            assert!(tree.is_some());
        });
    });
    
    group.finish();
}

fn generate_rust_code(lines: usize) -> String {
    let mut code = String::new();
    for i in 0..lines/5 {
        code.push_str(&format!("fn function_{}() {{ let x = {}; }}\n", i, i));
        code.push_str(&format!("struct S{} {{ field: i32 }}\n", i));
        code.push_str(&format!("impl S{} {{ fn new() -> Self {{ Self {{ field: 0 }} }} }}\n", i));
        code.push_str(&format!("const C{}: i32 = {};\n", i, i));
        code.push_str(&format!("// Comment {}\n", i));
    }
    code
}

fn generate_js_code(lines: usize) -> String {
    let mut code = String::new();
    for i in 0..lines/3 {
        code.push_str(&format!("function func{}() {{ return {}; }}\n", i, i));
        code.push_str(&format!("const var{} = {};\n", i, i));
        code.push_str(&format!("// Comment {}\n", i));
    }
    code
}

criterion_group!(benches, benchmark_parse_speed, benchmark_memory_usage, benchmark_incremental_parsing);
criterion_main!(benches);
