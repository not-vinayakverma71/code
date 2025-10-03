//! Test symbol extraction with EXACT Codex format
//! CRITICAL: Symbol format took YEARS to perfect - must match exactly

use lapce_tree_sitter::{NativeParserManager, SymbolExtractor};
use std::sync::Arc;
use tempfile::NamedTempFile;
use std::io::Write;

#[tokio::test]
async fn test_rust_symbol_extraction() {
    // Create a temporary Rust file with various symbol types
    let rust_code = r#"
pub struct MyStruct {
    field: String,
}

impl MyStruct {
    pub fn new() -> Self {
        MyStruct { field: String::new() }
    }
    
    pub fn method(&self) -> &str {
        &self.field
    }
}

pub fn standalone_function() -> i32 {
    42
}

pub trait MyTrait {
    fn trait_method(&self);
}

const MY_CONSTANT: i32 = 100;
let my_variable = 42;
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(rust_code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    // Parse and extract symbols
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let symbol_extractor = SymbolExtractor::new(parser_manager);
    
    let symbols = symbol_extractor
        .extract_symbols(temp_file.path())
        .await
        .unwrap();
    
    // Verify EXACT Codex format
    let symbol_names: Vec<String> = symbols.iter().map(|s| s.name.clone()).collect();
    
    // These must match EXACT Codex format
    assert!(symbol_names.contains(&"struct MyStruct".to_string()), "Struct format must be 'struct MyStruct'");
    assert!(symbol_names.contains(&"function standalone_function()".to_string()), "Function format must be 'function standalone_function()'");
    assert!(symbol_names.contains(&"trait MyTrait".to_string()), "Trait format must be 'trait MyTrait'");
    assert!(symbol_names.contains(&"const MY_CONSTANT".to_string()), "Const format must be 'const MY_CONSTANT'");
    
    // Methods should be formatted as ClassName.method()
    assert!(symbol_names.iter().any(|n| n == "MyStruct.new()" || n == "MyStruct.method()"), 
        "Methods must be formatted as 'ClassName.method()'");
}

#[tokio::test]
async fn test_javascript_symbol_extraction() {
    // Create a temporary JavaScript file
    let js_code = r#"
class MyClass {
    constructor() {
        this.value = 0;
    }
    
    myMethod() {
        return this.value;
    }
}

function myFunction() {
    return 42;
}

const myConst = 100;
let myLet = 200;
var myVar = 300;
"#;

    let mut temp_file = NamedTempFile::with_suffix(".js").unwrap();
    temp_file.write_all(js_code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let symbol_extractor = SymbolExtractor::new(parser_manager);
    
    let symbols = symbol_extractor
        .extract_symbols(temp_file.path())
        .await
        .unwrap();
    
    let symbol_names: Vec<String> = symbols.iter().map(|s| s.name.clone()).collect();
    
    // Verify EXACT Codex format for JavaScript
    assert!(symbol_names.contains(&"class MyClass".to_string()), "Class format must be 'class MyClass'");
    assert!(symbol_names.contains(&"function myFunction()".to_string()), "Function format must be 'function myFunction()'");
    assert!(symbol_names.contains(&"const myConst".to_string()), "Const format must be 'const myConst'");
    assert!(symbol_names.contains(&"let myLet".to_string()), "Let format must be 'let myLet'");
    assert!(symbol_names.contains(&"var myVar".to_string()), "Var format must be 'var myVar'");
    
    // Methods should be formatted as MyClass.myMethod()
    assert!(symbol_names.iter().any(|n| n.contains("MyClass.") && n.contains("()")),
        "Methods must be formatted as 'MyClass.method()'");
}

#[tokio::test]
async fn test_python_symbol_extraction() {
    let python_code = r#"
class MyClass:
    def __init__(self):
        self.value = 0
    
    def my_method(self):
        return self.value

def my_function():
    return 42

MY_CONSTANT = 100
my_variable = 200
"#;

    let mut temp_file = NamedTempFile::with_suffix(".py").unwrap();
    temp_file.write_all(python_code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let symbol_extractor = SymbolExtractor::new(parser_manager);
    
    let symbols = symbol_extractor
        .extract_symbols(temp_file.path())
        .await
        .unwrap();
    
    let symbol_names: Vec<String> = symbols.iter().map(|s| s.name.clone()).collect();
    
    // Verify Python symbols follow Codex format
    assert!(symbol_names.contains(&"class MyClass".to_string()), "Python class format must be 'class MyClass'");
    assert!(symbol_names.contains(&"function my_function()".to_string()), "Python function format must be 'function my_function()'");
    
    // Python methods should be MyClass.method()
    assert!(symbol_names.iter().any(|n| n == "MyClass.__init__()" || n == "MyClass.my_method()"),
        "Python methods must be 'ClassName.method()'");
}

#[tokio::test]
async fn test_minimum_lines_requirement() {
    // Test that components with less than 4 lines are filtered out
    let short_code = r#"
fn short() { 42 }
fn also_short() { 
    1 
}
"#;

    let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
    temp_file.write_all(short_code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let symbol_extractor = SymbolExtractor::new(parser_manager);
    
    let symbols = symbol_extractor
        .extract_symbols(temp_file.path())
        .await
        .unwrap();
    
    // Functions less than 4 lines should be filtered out
    assert_eq!(symbols.len(), 0, "Functions with less than 4 lines should be filtered out");
}

#[tokio::test]
async fn test_typescript_symbol_extraction() {
    let ts_code = r#"
interface MyInterface {
    value: number;
    method(): string;
}

type MyType = {
    field: string;
}

class MyClass implements MyInterface {
    value: number = 0;
    
    method(): string {
        return "test";
    }
}

function myFunction(): number {
    return 42;
}

const myConst: number = 100;
"#;

    let mut temp_file = NamedTempFile::with_suffix(".ts").unwrap();
    temp_file.write_all(ts_code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    
    let parser_manager = Arc::new(NativeParserManager::new().unwrap());
    let symbol_extractor = SymbolExtractor::new(parser_manager);
    
    let symbols = symbol_extractor
        .extract_symbols(temp_file.path())
        .await
        .unwrap();
    
    let symbol_names: Vec<String> = symbols.iter().map(|s| s.name.clone()).collect();
    
    // Verify TypeScript-specific formats
    assert!(symbol_names.contains(&"interface MyInterface".to_string()), "Interface format must be 'interface MyInterface'");
    assert!(symbol_names.contains(&"type MyType".to_string()), "Type alias format must be 'type MyType'");
    assert!(symbol_names.contains(&"class MyClass".to_string()), "TypeScript class format must be 'class MyClass'");
    assert!(symbol_names.contains(&"function myFunction()".to_string()), "TypeScript function format must be 'function myFunction()'");
}
