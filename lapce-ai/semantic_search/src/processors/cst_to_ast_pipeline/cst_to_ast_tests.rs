// Unit tests for CST to AST pipeline with canonical mapping
use super::*;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use std::io::Write;

#[tokio::test]
    async fn test_rust_canonical_mapping() {
        let code = r#"
fn main() {
    let x = 42;
    println!("Hello");
}

struct Person {
    name: String,
    age: u32,
}
"#;
        
        let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
        temp_file.write_all(code.as_bytes()).unwrap();
        let path = temp_file.path();
        
        let pipeline = CstToAstPipeline::new();
        let result = pipeline.process_file(path).await.unwrap();
        
        // Root should be Program/Module
        assert!(matches!(result.ast.node_type, AstNodeType::Program | AstNodeType::Module));
        
        // Find function declaration
        let func = find_node_by_type(&result.ast, AstNodeType::FunctionDeclaration);
        assert!(func.is_some(), "Should find function declaration");
        
        #[cfg(feature = "cst_ts")]
        {
            let func = func.unwrap();
            assert_eq!(func.identifier, Some("main".to_string()));
        }
        
        // Find struct declaration
        let struct_node = find_node_by_type(&result.ast, AstNodeType::StructDeclaration);
        assert!(struct_node.is_some(), "Should find struct declaration");
        
        #[cfg(feature = "cst_ts")]
        {
            let struct_node = struct_node.unwrap();
            assert_eq!(struct_node.identifier, Some("Person".to_string()));
        }
        
        // Find variable declaration
        let var = find_node_by_type(&result.ast, AstNodeType::VariableDeclaration);
        assert!(var.is_some(), "Should find variable declaration");
        
        // Find string literal
        let string_lit = find_node_by_type(&result.ast, AstNodeType::StringLiteral);
        assert!(string_lit.is_some(), "Should find string literal");
        
        #[cfg(feature = "cst_ts")]
        {
            let string_lit = string_lit.unwrap();
            assert!(string_lit.value.is_some());
        }
    }

    #[tokio::test]
    async fn test_javascript_canonical_mapping() {
        let code = r#"
function greet(name) {
    return "Hello, " + name;
}

class Animal {
    constructor(type) {
        this.type = type;
    }
}

const PI = 3.14159;
"#;
        
        let mut temp_file = NamedTempFile::with_suffix(".js").unwrap();
        temp_file.write_all(code.as_bytes()).unwrap();
        let path = temp_file.path();
        
        let pipeline = CstToAstPipeline::new();
        let result = pipeline.process_file(path).await.unwrap();
        
        // Root should be Program/Module
        assert!(matches!(result.ast.node_type, AstNodeType::Program | AstNodeType::Module));
        
        // Find function declaration
        let func = find_node_by_type(&result.ast, AstNodeType::FunctionDeclaration);
        assert!(func.is_some(), "Should find function declaration");
        
        #[cfg(feature = "cst_ts")]
        {
            let func = func.unwrap();
            assert_eq!(func.identifier, Some("greet".to_string()));
        }
        
        // Find class declaration
        let class = find_node_by_type(&result.ast, AstNodeType::ClassDeclaration);
        assert!(class.is_some(), "Should find class declaration");
        
        #[cfg(feature = "cst_ts")]
        {
            let class = class.unwrap();
            assert_eq!(class.identifier, Some("Animal".to_string()));
        }
        
        // Find variable declaration
        let var = find_node_by_type(&result.ast, AstNodeType::VariableDeclaration);
        assert!(var.is_some(), "Should find variable declaration");
        
        // Find number literal
        let num_lit = find_node_by_type(&result.ast, AstNodeType::NumberLiteral);
        assert!(num_lit.is_some(), "Should find number literal");
        
        #[cfg(feature = "cst_ts")]
        {
            let num_lit = num_lit.unwrap();
            assert!(num_lit.value.is_some());
        }
    }

    #[tokio::test]
    async fn test_python_canonical_mapping() {
        let code = r#"
def calculate(x, y):
    return x + y

class Calculator:
    def __init__(self):
        self.result = 0
    
    def add(self, value):
        self.result += value

if __name__ == "__main__":
    calc = Calculator()
    calc.add(5)
"#;
        
        let mut temp_file = NamedTempFile::with_suffix(".py").unwrap();
        temp_file.write_all(code.as_bytes()).unwrap();
        let path = temp_file.path();
        
        let pipeline = CstToAstPipeline::new();
        let result = pipeline.process_file(path).await.unwrap();
        
        // Root should be Module
        assert_eq!(result.ast.node_type, AstNodeType::Module);
        
        // Find function declarations
        let funcs = find_all_nodes_by_type(&result.ast, AstNodeType::FunctionDeclaration);
        assert!(funcs.len() >= 2, "Should find at least 2 function declarations");
        
        #[cfg(feature = "cst_ts")]
        {
            let func_names: Vec<_> = funcs.iter()
                .filter_map(|f| f.identifier.as_ref())
                .collect();
            assert!(func_names.contains(&&"calculate".to_string()));
            assert!(func_names.contains(&&"add".to_string()) || func_names.contains(&&"__init__".to_string()));
        }
        
        // Find class declaration
        let class = find_node_by_type(&result.ast, AstNodeType::ClassDeclaration);
        assert!(class.is_some(), "Should find class declaration");
        
        #[cfg(feature = "cst_ts")]
        {
            let class = class.unwrap();
            assert_eq!(class.identifier, Some("Calculator".to_string()));
        }
        
        // Find if statement
        let if_stmt = find_node_by_type(&result.ast, AstNodeType::IfStatement);
        assert!(if_stmt.is_some(), "Should find if statement");
    }

    #[tokio::test]
    async fn test_go_canonical_mapping() {
        let code = r#"
package main

import "fmt"

func main() {
    message := "Hello, Go!"
    fmt.Println(message)
}

type Server struct {
    Port int
    Host string
}
"#;
        
        let mut temp_file = NamedTempFile::with_suffix(".go").unwrap();
        temp_file.write_all(code.as_bytes()).unwrap();
        let path = temp_file.path();
        
        let pipeline = CstToAstPipeline::new();
        let result = pipeline.process_file(path).await.unwrap();
        
        // Root should be Program or Module
        assert!(matches!(result.ast.node_type, AstNodeType::Program | AstNodeType::Module));
        
        // Package and import may or may not be present depending on parser
        let _pkg = find_node_by_type(&result.ast, AstNodeType::Package);
        let _import = find_node_by_type(&result.ast, AstNodeType::ImportStatement);
        
        // Find function declaration
        let func = find_node_by_type(&result.ast, AstNodeType::FunctionDeclaration);
        assert!(func.is_some(), "Should find function declaration");
        
        #[cfg(feature = "cst_ts")]
        {
            let func = func.unwrap();
            assert_eq!(func.identifier, Some("main".to_string()));
        }
        
        // Find struct declaration (mapped from type declaration)
        let struct_node = find_node_by_type(&result.ast, AstNodeType::StructDeclaration);
        assert!(struct_node.is_some(), "Should find struct declaration");
    }

    #[tokio::test]
    async fn test_java_canonical_mapping() {
        let code = r#"
package com.example;

public class Main {
    public static void main(String[] args) {
        System.out.println("Hello, Java!");
    }
}

interface Runnable {
    void run();
}
"#;
        
        let mut temp_file = NamedTempFile::with_suffix(".java").unwrap();
        temp_file.write_all(code.as_bytes()).unwrap();
        let path = temp_file.path();
        
        let pipeline = CstToAstPipeline::new();
        let result = pipeline.process_file(path).await.unwrap();
        
        // Root should be Program or Module
        assert!(matches!(result.ast.node_type, AstNodeType::Program | AstNodeType::Module));
        
        // Package may or may not be present depending on parser
        let _pkg = find_node_by_type(&result.ast, AstNodeType::Package);
        
        // Find class declaration
        let class = find_node_by_type(&result.ast, AstNodeType::ClassDeclaration);
        assert!(class.is_some(), "Should find class declaration");
        
        #[cfg(feature = "cst_ts")]
        {
            let class = class.unwrap();
            assert_eq!(class.identifier, Some("Main".to_string()));
        }
        
        // Find interface declaration
        let interface = find_node_by_type(&result.ast, AstNodeType::InterfaceDeclaration);
        assert!(interface.is_some(), "Should find interface declaration");
        
        #[cfg(feature = "cst_ts")]
        {
            let interface = interface.unwrap();
            assert_eq!(interface.identifier, Some("Runnable".to_string()));
        }
        
        // Find function/method declaration
        let method = find_node_by_type(&result.ast, AstNodeType::FunctionDeclaration);
        assert!(method.is_some(), "Should find method declaration");
    }

    #[tokio::test]
    #[cfg(feature = "cst_ts")]
    async fn test_cross_language_consistency() {
        // Test that similar constructs map to same AST types across languages
        
        let rust_func = "fn add(x: i32, y: i32) -> i32 { x + y }";
        let js_func = "function add(x, y) { return x + y; }";
        let python_func = "def add(x, y):\n    return x + y";
        let go_func = "func add(x int, y int) int { return x + y }";
        
        let test_cases = vec![
            (rust_func, "rs"),
            (js_func, "js"),
            (python_func, "py"),
            (go_func, "go"),
        ];
        
        for (code, ext) in test_cases {
            let mut temp_file = NamedTempFile::with_suffix(&format!(".{}", ext)).unwrap();
            temp_file.write_all(code.as_bytes()).unwrap();
            let path = temp_file.path();
            
            let pipeline = CstToAstPipeline::new();
            let result = pipeline.process_file(path).await.unwrap();
            
            // All should have a function declaration
            let func = find_node_by_type(&result.ast, AstNodeType::FunctionDeclaration);
            assert!(func.is_some(), "Should find function in {} code", ext);
            
            let func = func.unwrap();
            assert_eq!(func.identifier, Some("add".to_string()), 
                      "Function name should be 'add' in {} code", ext);
            
            // Return statement may be implicit in some languages (e.g., Rust expression)
            let _ret = find_node_by_type(&result.ast, AstNodeType::ReturnStatement);
        }
    }

    // Helper functions
    fn find_node_by_type(root: &AstNode, node_type: AstNodeType) -> Option<AstNode> {
        if root.node_type == node_type {
            return Some(root.clone());
        }
        
        for child in &root.children {
            if let Some(found) = find_node_by_type(child, node_type) {
                return Some(found);
            }
        }
        
        None
    }
    
    fn find_all_nodes_by_type(root: &AstNode, node_type: AstNodeType) -> Vec<AstNode> {
        let mut results = Vec::new();
        
        if root.node_type == node_type {
            results.push(root.clone());
        }
        
        for child in &root.children {
            results.extend(find_all_nodes_by_type(child, node_type));
        }
        
        results
    }
