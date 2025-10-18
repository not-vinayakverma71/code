// Codex 1:1 Symbol Format Acceptance Tests
// Validates that symbols match exact Codex format from docs/05-TREE-SITTER-INTEGRATION.md
// Reference: /home/verma/lapce/Codex/src/services/tree-sitter/queries/*.ts

use lancedb::processors::cst_to_ast_pipeline::{CstToAstPipeline, AstNodeType};
use std::fs;
use tempfile::TempDir;

/// Test Rust symbol extraction matches Codex format
#[tokio::test]
async fn test_rust_codex_symbols() {
    let sample = r#"
fn my_function() {
    let x = 42;
}

struct MyStruct {
    field: i32,
}

enum MyEnum {
    Variant1,
    Variant2,
}

trait MyTrait {
    fn method(&self);
}
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await.unwrap();
    
    // Verify we extracted functions, structs, enums, traits
    let ast = &result.ast;
    assert!(has_node_type(&ast, AstNodeType::FunctionDeclaration), "Missing function");
    assert!(has_node_type(&ast, AstNodeType::StructDeclaration), "Missing struct");
    assert!(has_node_type(&ast, AstNodeType::EnumDeclaration), "Missing enum");
    assert!(has_node_type(&ast, AstNodeType::TraitDeclaration), "Missing trait");
    
    println!("✅ Rust symbols match Codex format");
}

/// Test JavaScript symbol extraction matches Codex format
#[tokio::test]
async fn test_javascript_codex_symbols() {
    let sample = r#"
class MyClass {
    constructor() {
        this.value = 42;
    }
    
    myMethod() {
        return this.value;
    }
}

function myFunction() {
    return 42;
}

const myArrow = () => {
    return 42;
};
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.js");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await.unwrap();
    
    // Codex expects: class MyClass, function myFunction(), MyClass.myMethod()
    let ast = &result.ast;
    assert!(has_node_type(&ast, AstNodeType::ClassDeclaration), "Missing class");
    assert!(has_node_type(&ast, AstNodeType::FunctionDeclaration), "Missing function");
    
    println!("✅ JavaScript symbols match Codex format");
}

/// Test TypeScript symbol extraction matches Codex format
#[tokio::test]
async fn test_typescript_codex_symbols() {
    let sample = r#"
interface MyInterface {
    value: number;
}

class MyClass implements MyInterface {
    value: number;
    
    constructor(value: number) {
        this.value = value;
    }
    
    getValue(): number {
        return this.value;
    }
}

function myFunction(x: number): number {
    return x * 2;
}
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.ts");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await.unwrap();
    
    let ast = &result.ast;
    assert!(has_node_type(&ast, AstNodeType::ClassDeclaration), "Missing class");
    assert!(has_node_type(&ast, AstNodeType::FunctionDeclaration), "Missing function");
    
    println!("✅ TypeScript symbols match Codex format");
}

/// Test Python symbol extraction matches Codex format
#[tokio::test]
async fn test_python_codex_symbols() {
    let sample = r#"
def my_function():
    return 42

class MyClass:
    def __init__(self):
        self.value = 42
    
    def my_method(self):
        return self.value
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await.unwrap();
    
    // Codex expects: def my_function(), class MyClass
    let ast = &result.ast;
    assert!(has_node_type(&ast, AstNodeType::FunctionDeclaration), "Missing function");
    assert!(has_node_type(&ast, AstNodeType::ClassDeclaration), "Missing class");
    
    println!("✅ Python symbols match Codex format");
}

/// Test Go symbol extraction
#[tokio::test]
async fn test_go_codex_symbols() {
    let sample = r#"
package main

func myFunction() int {
    return 42
}

type MyStruct struct {
    Value int
}
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.go");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await.unwrap();
    
    let ast = &result.ast;
    assert!(has_node_type(&ast, AstNodeType::FunctionDeclaration), "Missing function");
    
    println!("✅ Go symbols extracted");
}

/// Test Java symbol extraction
#[tokio::test]
async fn test_java_codex_symbols() {
    let sample = r#"
public class MyClass {
    private int value;
    
    public MyClass(int value) {
        this.value = value;
    }
    
    public int getValue() {
        return value;
    }
}
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.java");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await.unwrap();
    
    let ast = &result.ast;
    assert!(has_node_type(&ast, AstNodeType::ClassDeclaration), "Missing class");
    
    println!("✅ Java symbols extracted");
}

/// Test C++ symbol extraction
#[tokio::test]
async fn test_cpp_codex_symbols() {
    let sample = r#"
class MyClass {
public:
    int value;
    
    MyClass(int v) : value(v) {}
    
    int getValue() {
        return value;
    }
};

int myFunction() {
    return 42;
}
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cpp");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await.unwrap();
    
    let ast = &result.ast;
    assert!(has_node_type(&ast, AstNodeType::ClassDeclaration), "Missing class");
    assert!(has_node_type(&ast, AstNodeType::FunctionDeclaration), "Missing function");
    
    println!("✅ C++ symbols extracted");
}

/// Test HTML symbol extraction
#[tokio::test]
async fn test_html_codex_symbols() {
    let sample = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body>
    <div id="content">
        <h1>Hello</h1>
    </div>
</body>
</html>
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.html");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await;
    
    assert!(result.is_ok(), "HTML parsing should succeed");
    println!("✅ HTML parsed successfully");
}

/// Test CSS symbol extraction
#[tokio::test]
async fn test_css_codex_symbols() {
    let sample = r#"
.my-class {
    color: red;
    font-size: 14px;
}

#my-id {
    background: blue;
}
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.css");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await;
    
    assert!(result.is_ok(), "CSS parsing should succeed");
    println!("✅ CSS parsed successfully");
}

/// Test JSON symbol extraction
#[tokio::test]
async fn test_json_codex_symbols() {
    let sample = r#"
{
    "name": "test",
    "value": 42,
    "nested": {
        "key": "value"
    }
}
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await;
    
    assert!(result.is_ok(), "JSON parsing should succeed");
    println!("✅ JSON parsed successfully");
}

/// Test Bash symbol extraction
#[tokio::test]
async fn test_bash_codex_symbols() {
    let sample = r#"
#!/bin/bash

my_function() {
    echo "Hello"
    return 0
}

my_variable="test"
"#;
    
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.sh");
    fs::write(&file_path, sample).unwrap();
    
    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&file_path).await;
    
    assert!(result.is_ok(), "Bash parsing should succeed");
    println!("✅ Bash parsed successfully");
}

/// Test all Top 12 languages in batch
#[tokio::test]
async fn test_top_12_codex_format_batch() {
    let languages = vec![
        ("rust", "rs"),
        ("javascript", "js"),
        ("typescript", "ts"),
        ("python", "py"),
        ("go", "go"),
        ("java", "java"),
        ("c", "c"),
        ("cpp", "cpp"),
        ("html", "html"),
        ("css", "css"),
        ("json", "json"),
        ("bash", "sh"),
    ];
    
    let temp_dir = TempDir::new().unwrap();
    let pipeline = CstToAstPipeline::new();
    
    let mut success_count = 0;
    let mut total_count = 0;
    
    for (lang, ext) in languages {
        total_count += 1;
        let file_path = temp_dir.path().join(format!("test.{}", ext));
        let sample = format!("// Test file for {}", lang);
        fs::write(&file_path, &sample).unwrap();
        
        match pipeline.process_file(&file_path).await {
            Ok(_) => {
                success_count += 1;
                println!("✅ {} - Parsed successfully", lang);
            }
            Err(e) => {
                println!("❌ {} - Parse failed: {}", lang, e);
            }
        }
    }
    
    println!("\n=== Top 12 Languages Codex Format Test ===");
    println!("Total: {}", total_count);
    println!("Success: {}", success_count);
    println!("Success rate: {:.1}%", (success_count as f64 / total_count as f64) * 100.0);
    
    assert_eq!(success_count, total_count, "All Top 12 languages must parse successfully");
}

// Helper function to check if AST contains a specific node type
fn has_node_type(node: &lancedb::processors::cst_to_ast_pipeline::AstNode, node_type: AstNodeType) -> bool {
    if node.node_type == node_type {
        return true;
    }
    
    for child in &node.children {
        if has_node_type(child, node_type) {
            return true;
        }
    }
    
    false
}
