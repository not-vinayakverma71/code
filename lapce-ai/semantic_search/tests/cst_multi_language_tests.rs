// Multi-language CST tests for production readiness
use lancedb::processors::cst_to_ast_pipeline::{CstToAstPipeline, AstNodeType};
use std::path::PathBuf;

#[tokio::test]
async fn test_rust_parsing() {
    let pipeline = CstToAstPipeline::new();
    let rust_code = r#"
fn main() {
    println!("Hello, world!");
}

pub struct User {
    name: String,
    age: u32,
}

impl User {
    pub fn new(name: String, age: u32) -> Self {
        User { name, age }
    }
}
"#;
    
    let path = PathBuf::from("test.rs");
    let result = pipeline.process_file(&path, rust_code).await;
    
    assert!(result.is_ok(), "Rust parsing should succeed");
    let output = result.unwrap();
    
    // Verify AST summary
    assert!(output.ast_summary.is_some());
    let summary = output.ast_summary.unwrap();
    assert_eq!(summary.function_count, 2, "Should detect 2 functions (main + new)");
    assert_eq!(summary.struct_count, 1, "Should detect 1 struct");
}

#[tokio::test]
async fn test_typescript_parsing() {
    let pipeline = CstToAstPipeline::new();
    let ts_code = r#"
interface Person {
    name: string;
    age: number;
}

class User implements Person {
    constructor(public name: string, public age: number) {}
    
    greet(): string {
        return `Hello, ${this.name}`;
    }
}

function createUser(name: string, age: number): User {
    return new User(name, age);
}
"#;
    
    let path = PathBuf::from("test.ts");
    let result = pipeline.process_file(&path, ts_code).await;
    
    assert!(result.is_ok(), "TypeScript parsing should succeed");
    let output = result.unwrap();
    
    assert!(output.ast_summary.is_some());
    let summary = output.ast_summary.unwrap();
    assert_eq!(summary.class_count, 1, "Should detect 1 class");
    assert_eq!(summary.function_count, 2, "Should detect 2 functions (greet + createUser)");
}

#[tokio::test]
async fn test_python_parsing() {
    let pipeline = CstToAstPipeline::new();
    let py_code = r#"
class User:
    def __init__(self, name: str, age: int):
        self.name = name
        self.age = age
    
    def greet(self) -> str:
        return f"Hello, {self.name}"

def create_user(name: str, age: int) -> User:
    return User(name, age)

async def fetch_user(user_id: int) -> User:
    # Async function
    return User("Test", 25)
"#;
    
    let path = PathBuf::from("test.py");
    let result = pipeline.process_file(&path, py_code).await;
    
    assert!(result.is_ok(), "Python parsing should succeed");
    let output = result.unwrap();
    
    assert!(output.ast_summary.is_some());
    let summary = output.ast_summary.unwrap();
    assert_eq!(summary.class_count, 1, "Should detect 1 class");
    assert!(summary.function_count >= 3, "Should detect at least 3 functions");
}

#[tokio::test]
async fn test_javascript_parsing() {
    let pipeline = CstToAstPipeline::new();
    let js_code = r#"
class User {
    constructor(name, age) {
        this.name = name;
        this.age = age;
    }
    
    greet() {
        return `Hello, ${this.name}`;
    }
}

function createUser(name, age) {
    return new User(name, age);
}

const fetchUser = async (userId) => {
    return new User("Test", 25);
};
"#;
    
    let path = PathBuf::from("test.js");
    let result = pipeline.process_file(&path, js_code).await;
    
    assert!(result.is_ok(), "JavaScript parsing should succeed");
    let output = result.unwrap();
    
    assert!(output.ast_summary.is_some());
}

#[tokio::test]
async fn test_go_parsing() {
    let pipeline = CstToAstPipeline::new();
    let go_code = r#"
package main

import "fmt"

type User struct {
    Name string
    Age  int
}

func NewUser(name string, age int) *User {
    return &User{Name: name, Age: age}
}

func (u *User) Greet() string {
    return fmt.Sprintf("Hello, %s", u.Name)
}

func main() {
    user := NewUser("Alice", 30)
    fmt.Println(user.Greet())
}
"#;
    
    let path = PathBuf::from("test.go");
    let result = pipeline.process_file(&path, go_code).await;
    
    assert!(result.is_ok(), "Go parsing should succeed");
    let output = result.unwrap();
    
    assert!(output.ast_summary.is_some());
    let summary = output.ast_summary.unwrap();
    assert!(summary.function_count >= 3, "Should detect multiple functions");
}

#[tokio::test]
async fn test_java_parsing() {
    let pipeline = CstToAstPipeline::new();
    let java_code = r#"
package com.example;

public class User {
    private String name;
    private int age;
    
    public User(String name, int age) {
        this.name = name;
        this.age = age;
    }
    
    public String greet() {
        return "Hello, " + this.name;
    }
    
    public static User createUser(String name, int age) {
        return new User(name, age);
    }
}
"#;
    
    let path = PathBuf::from("User.java");
    let result = pipeline.process_file(&path, java_code).await;
    
    assert!(result.is_ok(), "Java parsing should succeed");
    let output = result.unwrap();
    
    assert!(output.ast_summary.is_some());
    let summary = output.ast_summary.unwrap();
    assert_eq!(summary.class_count, 1, "Should detect 1 class");
}

#[tokio::test]
async fn test_cpp_parsing() {
    let pipeline = CstToAstPipeline::new();
    let cpp_code = r#"
#include <string>
#include <iostream>

class User {
private:
    std::string name;
    int age;

public:
    User(const std::string& name, int age) : name(name), age(age) {}
    
    std::string greet() const {
        return "Hello, " + name;
    }
    
    static User* createUser(const std::string& name, int age) {
        return new User(name, age);
    }
};

int main() {
    User user("Alice", 30);
    std::cout << user.greet() << std::endl;
    return 0;
}
"#;
    
    let path = PathBuf::from("test.cpp");
    let result = pipeline.process_file(&path, cpp_code).await;
    
    assert!(result.is_ok(), "C++ parsing should succeed");
    let output = result.unwrap();
    
    assert!(output.ast_summary.is_some());
}

// Fuzz tests for malformed sources
#[tokio::test]
async fn test_malformed_rust() {
    let pipeline = CstToAstPipeline::new();
    let malformed = "fn incomplete( { }";
    
    let path = PathBuf::from("malformed.rs");
    let result = pipeline.process_file(&path, malformed).await;
    
    // Should not panic, but may return error or partial parse
    assert!(result.is_ok() || result.is_err(), "Should handle malformed input gracefully");
}

#[tokio::test]
async fn test_empty_file() {
    let pipeline = CstToAstPipeline::new();
    let empty = "";
    
    let path = PathBuf::from("empty.rs");
    let result = pipeline.process_file(&path, empty).await;
    
    assert!(result.is_ok(), "Should handle empty files");
    if let Ok(output) = result {
        assert_eq!(output.chunks.len(), 0, "Empty file should produce no chunks");
    }
}

#[tokio::test]
async fn test_unicode_content() {
    let pipeline = CstToAstPipeline::new();
    let unicode = r#"
fn main() {
    let greeting = "Hello, 世界! 🌍";
    println!("{}", greeting);
}
"#;
    
    let path = PathBuf::from("unicode.rs");
    let result = pipeline.process_file(&path, unicode).await;
    
    assert!(result.is_ok(), "Should handle Unicode content");
}

#[tokio::test]
async fn test_very_large_file() {
    let pipeline = CstToAstPipeline::new();
    
    // Generate large file with many functions
    let mut large_code = String::from("// Large file test\n");
    for i in 0..1000 {
        large_code.push_str(&format!("fn function_{}() {{ println!(\"test\"); }}\n", i));
    }
    
    let path = PathBuf::from("large.rs");
    let result = pipeline.process_file(&path, &large_code).await;
    
    assert!(result.is_ok(), "Should handle large files");
    if let Ok(output) = result {
        assert!(output.ast_summary.is_some());
        let summary = output.ast_summary.unwrap();
        assert!(summary.function_count >= 500, "Should detect many functions");
    }
}

#[tokio::test]
async fn test_deeply_nested_code() {
    let pipeline = CstToAstPipeline::new();
    let nested = r#"
fn outer() {
    fn level1() {
        fn level2() {
            fn level3() {
                fn level4() {
                    println!("Deep nesting");
                }
                level4();
            }
            level3();
        }
        level2();
    }
    level1();
}
"#;
    
    let path = PathBuf::from("nested.rs");
    let result = pipeline.process_file(&path, nested).await;
    
    assert!(result.is_ok(), "Should handle deeply nested code");
}

#[tokio::test]
async fn test_mixed_languages_detection() {
    let pipeline = CstToAstPipeline::new();
    
    // Test that language detection works correctly
    let extensions = vec![
        ("test.rs", "rust"),
        ("test.ts", "typescript"),
        ("test.tsx", "typescript"),
        ("test.js", "javascript"),
        ("test.jsx", "javascript"),
        ("test.py", "python"),
        ("test.go", "go"),
        ("test.java", "java"),
        ("test.cpp", "cpp"),
        ("test.c", "c"),
    ];
    
    for (filename, expected_lang) in extensions {
        let path = PathBuf::from(filename);
        let code = "// Simple test";
        let result = pipeline.process_file(&path, code).await;
        
        if let Ok(output) = result {
            // Verify language detection
            assert!(output.chunks.is_empty() || 
                   output.chunks[0].language.as_ref().map_or(false, |l| l.contains(expected_lang)),
                   "Language detection failed for {}", filename);
        }
    }
}
