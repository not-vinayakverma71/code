use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lapce_ai_rust::core::tools::{
    registry::ToolRegistry,
    xml_util::{XmlParser, XmlGenerator},
    traits::{Tool, ToolContext, ToolResult, ToolError, ToolOutput},
};
use lapce_ai_rust::core::tools::permissions::rooignore::RooIgnore;
use serde_json::json;
use std::time::Duration;
use std::sync::Arc;
use std::path::PathBuf;
use tempfile::TempDir;

// Test tool for benchmarking
struct BenchTool {
    name: String,
}

#[async_trait::async_trait]
impl Tool for BenchTool {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }
    
    fn description(&self) -> &'static str {
        "Benchmark test tool"
    }
    
    async fn execute(&self, args: serde_json::Value, _ctx: ToolContext) -> ToolResult {
        Ok(ToolOutput {
            success: true,
            result: args,
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }
}

fn benchmark_registry(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_registry");
    
    // Benchmark registry lookup with different sizes
    for size in [10, 100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("lookup", size),
            &size,
            |b, &size| {
                let registry = ToolRegistry::new();
                
                // Register tools
                for i in 0..size {
                    let tool = BenchTool {
                        name: format!("tool_{}", i),
                    };
                    registry.register(tool).unwrap();
                }
                
                b.iter(|| {
                    // Lookup middle tool
                    let name = format!("tool_{}", size / 2);
                    registry.get(&name)
                });
            }
        );
    }
    
    group.finish();
}

fn benchmark_xml_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("xml_parser");
    
    // Simple XML
    let simple_xml = r#"
        <tool>
            <name>readFile</name>
            <args>
                <path>/test/file.txt</path>
            </args>
        </tool>
    "#;
    
    group.bench_function("parse_simple", |b| {
        let parser = XmlParser::new();
        b.iter(|| {
            parser.parse(black_box(simple_xml)).unwrap()
        });
    });
    
    // Complex multi-file XML
    let complex_xml = r#"
        <tool>
            <name>multiEdit</name>
            <files>
                <file>
                    <path>file1.rs</path>
                    <lineStart>1</lineStart>
                    <lineEnd>10</lineEnd>
                    <content>Test content 1</content>
                </file>
                <file>
                    <path>file2.rs</path>
                    <lineStart>20</lineStart>
                    <lineEnd>30</lineEnd>
                    <content>Test content 2</content>
                </file>
                <file>
                    <path>file3.rs</path>
                    <lineStart>40</lineStart>
                    <lineEnd>50</lineEnd>
                    <content>Test content 3</content>
                </file>
            </files>
            <metadata>
                <requestId>abc123</requestId>
                <timestamp>2024-01-01T00:00:00Z</timestamp>
            </metadata>
        </tool>
    "#;
    
    group.bench_function("parse_complex", |b| {
        let parser = XmlParser::new();
        b.iter(|| {
            parser.parse(black_box(complex_xml)).unwrap()
        });
    });
    
    group.finish();
}

fn benchmark_xml_generator(c: &mut Criterion) {
    let mut group = c.benchmark_group("xml_generator");
    
    // Simple JSON
    let simple_json = json!({
        "status": "success",
        "content": "File content here"
    });
    
    group.bench_function("generate_simple", |b| {
        let generator = XmlGenerator::new();
        b.iter(|| {
            generator.generate(black_box(&simple_json)).unwrap()
        });
    });
    
    // Complex JSON
    let complex_json = json!({
        "files": [
            {
                "path": "file1.rs",
                "content": "Long content string that simulates real file content",
                "metadata": {
                    "size": 1024,
                    "modified": "2024-01-01T00:00:00Z",
                    "permissions": "rw-r--r--"
                }
            },
            {
                "path": "file2.rs",
                "content": "Another long content string for testing",
                "metadata": {
                    "size": 2048,
                    "modified": "2024-01-02T00:00:00Z",
                    "permissions": "rw-rw-r--"
                }
            }
        ],
        "summary": {
            "total_files": 2,
            "total_size": 3072,
            "status": "completed"
        }
    });
    
    group.bench_function("generate_complex", |b| {
        let generator = XmlGenerator::new();
        b.iter(|| {
            generator.generate(black_box(&complex_json)).unwrap()
        });
    });
    
    group.finish();
}

fn benchmark_xml_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("xml_roundtrip");
    
    let test_data = json!({
        "tool": "editFile",
        "args": {
            "path": "/workspace/src/main.rs",
            "lineRange": {
                "start": 10,
                "end": 20
            },
            "content": "Updated content for the file\nWith multiple lines\nAnd special characters: <>&\"'",
            "encoding": "utf-8"
        },
        "metadata": {
            "requestId": "req-12345",
            "timestamp": "2024-01-01T12:00:00Z",
            "user": "test-user"
        }
    });
    
    group.bench_function("full_roundtrip", |b| {
        let parser = XmlParser::new();
        let generator = XmlGenerator::new();
        
        b.iter(|| {
            let xml = generator.generate(black_box(&test_data)).unwrap();
            parser.parse(&xml).unwrap()
        });
    });
    
    group.finish();
}

fn benchmark_rooignore(c: &mut Criterion) {
    let mut group = c.benchmark_group("rooignore");
    
    // Create temp dir and rooignore
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
__pycache__/
*.pyc
*.pyo
.DS_Store
Thumbs.db
"#;
    
    rooignore.load_from_string(patterns).unwrap();
    
    // Benchmark single path check (uncached)
    group.bench_function("single_path_uncached", |b| {
        b.iter(|| {
            rooignore.clear_cache();
            rooignore.is_allowed(&temp_dir.path().join("src/main.rs"))
        });
    });
    
    // Benchmark single path check (cached)
    group.bench_function("single_path_cached", |b| {
        // Prime cache
        rooignore.is_allowed(&temp_dir.path().join("src/main.rs"));
        
        b.iter(|| {
            rooignore.is_allowed(&temp_dir.path().join("src/main.rs"))
        });
    });
    
    // Benchmark batch path filtering
    let paths: Vec<PathBuf> = (0..100)
        .flat_map(|i| vec![
            temp_dir.path().join(format!("file{}.rs", i)),
            temp_dir.path().join(format!("file{}.log", i)),
            temp_dir.path().join(format!("node_modules/package{}/index.js", i)),
            temp_dir.path().join(format!("src/module{}.rs", i)),
        ])
        .collect();
    
    group.bench_function("batch_filter_400_paths", |b| {
        b.iter(|| {
            rooignore.clear_cache();
            rooignore.filter_allowed(black_box(&paths))
        });
    });
    
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3));
    targets = benchmark_registry, 
              benchmark_xml_parser, 
              benchmark_xml_generator,
              benchmark_xml_roundtrip,
              benchmark_rooignore
}

criterion_main!(benches);
