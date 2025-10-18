// Benchmark for core tools module
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lapce_ai_rust::core::tools::{
    ToolRegistry, ToolContext, ToolOutput,
    RooIgnore,
    parse_tool_xml, generate_tool_xml,
};
use lapce_ai_rust::core::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;

struct MockTool {
    name: &'static str,
}

#[async_trait]
impl Tool for MockTool {
    fn name(&self) -> &'static str {
        self.name
    }
    
    fn description(&self) -> &'static str {
        "Mock tool for benchmarking"
    }
    
    async fn execute(&self, _args: Value, _context: ToolContext) -> lapce_ai_rust::core::tools::traits::ToolResult {
        Ok(ToolOutput::success(Value::Null))
    }
}

fn bench_registry_lookup(c: &mut Criterion) {
    let registry = ToolRegistry::new();
    
    // Register 1000 tools
    for i in 0..1000 {
        let name = Box::leak(format!("tool_{}", i).into_boxed_str());
        let tool = MockTool { name };
        registry.register(tool).unwrap();
    }
    
    c.bench_function("registry_lookup", |b| {
        b.iter(|| {
            registry.get(black_box("tool_500"))
        });
    });
}

fn bench_xml_parse(c: &mut Criterion) {
    let xml = r#"
        <tool_use name="readFile">
            <path>/test/file.txt</path>
            <encoding>utf-8</encoding>
        </tool_use>
    "#;
    
    c.bench_function("xml_parse", |b| {
        b.iter(|| {
            parse_tool_xml(black_box(xml))
        });
    });
}

fn bench_xml_multi_file_parse(c: &mut Criterion) {
    let xml = r#"
        <tool_use name="readFiles">
            <file path="file1.txt" start_line="10" end_line="20" />
            <file>
                <path>file2.txt</path>
                <start_line>5</start_line>
                <end_line>15</end_line>
            </file>
        </tool_use>
    "#;
    
    c.bench_function("xml_multi_file_parse", |b| {
        b.iter(|| {
            parse_tool_xml(black_box(xml))
        });
    });
}

fn bench_xml_generate(c: &mut Criterion) {
    let result = serde_json::json!({
        "content": "Hello, World!",
        "lines": 42,
        "success": true,
        "nested": {
            "key": "value"
        }
    });
    
    c.bench_function("xml_generate", |b| {
        b.iter(|| {
            generate_tool_xml(black_box("testTool"), black_box(&result))
        });
    });
}

fn bench_rooignore(c: &mut Criterion) {
    use tempfile::TempDir;
    
    let temp_dir = TempDir::new().unwrap();
    let mut rooignore = RooIgnore::new(temp_dir.path().to_path_buf());
    
    let patterns = r#"
*.log
*.tmp
*.cache
node_modules/
target/
build/
dist/
.git/
.vscode/
!build/important.txt
"#;
    
    rooignore.load_from_string(patterns).unwrap();
    
    let test_path = temp_dir.path().join("src/main.rs");
    
    c.bench_function("rooignore_match", |b| {
        b.iter(|| {
            rooignore.is_allowed(black_box(&test_path))
        });
    });
}

fn bench_rooignore_many_paths(c: &mut Criterion) {
    use tempfile::TempDir;
    
    let temp_dir = TempDir::new().unwrap();
    let mut rooignore = RooIgnore::new(temp_dir.path().to_path_buf());
    
    let patterns = r#"
*.log
*.tmp
node_modules/
target/
"#;
    
    rooignore.load_from_string(patterns).unwrap();
    
    // Create 100 test paths
    let paths: Vec<PathBuf> = (0..100)
        .map(|i| temp_dir.path().join(format!("src/file{}.rs", i)))
        .collect();
    
    c.bench_function("rooignore_match_many_paths", |b| {
        b.iter(|| {
            for path in &paths {
                black_box(rooignore.is_allowed(path));
            }
        });
    });
}

fn bench_diff_apply(c: &mut Criterion) {
    use tempfile::TempDir;
    use std::fs;
    use lapce_ai_rust::core::tools::diff_engine::DiffEngine;
    
    let temp_dir = TempDir::new().unwrap();
    
    // Create a 1k-line file
    let original = (0..1000)
        .map(|i| format!("Line {} with some content\n", i))
        .collect::<String>();
    
    let modified = (0..1000)
        .map(|i| {
            if i % 10 == 0 {
                format!("Modified line {} with different content\n", i)
            } else {
                format!("Line {} with some content\n", i)
            }
        })
        .collect::<String>();
    
    let engine = DiffEngine::new();
    
    c.bench_function("diff_apply_1k_lines", |b| {
        b.iter(|| {
            engine.generate_diff(black_box(&original), black_box(&modified))
        });
    });
}

fn bench_multi_file_read(c: &mut Criterion) {
    use tempfile::TempDir;
    use std::fs;
    
    let temp_dir = TempDir::new().unwrap();
    
    // Create 10 files with 100 lines each
    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("file{}.txt", i));
        let content = (0..100)
            .map(|j| format!("Line {} in file {}\n", j, i))
            .collect::<String>();
        fs::write(&file_path, content).unwrap();
    }
    
    let file_paths: Vec<PathBuf> = (0..10)
        .map(|i| temp_dir.path().join(format!("file{}.txt", i)))
        .collect();
    
    c.bench_function("multi_file_read_10_files", |b| {
        b.iter(|| {
            for path in &file_paths {
                black_box(fs::read_to_string(path).unwrap());
            }
        });
    });
}

criterion_group!(
    benches,
    bench_registry_lookup,
    bench_xml_parse,
    bench_xml_multi_file_parse,
    bench_xml_generate,
    bench_rooignore,
    bench_rooignore_many_paths,
    bench_diff_apply,
    bench_multi_file_read
);
criterion_main!(benches);
