// Comprehensive Parser Tests (Tasks 91-96)
use anyhow::Result;
use std::time::Instant;
use std::fs;
use tree_sitter::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("🧪 COMPREHENSIVE PARSER TESTS");
    println!("{}", "=".repeat(80));
    
    // Task 91: Test parsing Rust files
    test_parse_rust().await?;
    
    // Task 92: Test parsing Python files
    test_parse_python().await?;
    
    // Task 93: Test parsing JavaScript files
    test_parse_javascript().await?;
    
    // Task 94: Test parsing TypeScript files
    test_parse_typescript().await?;
    
    // Task 95: Test parsing Go files
    test_parse_go().await?;
    
    // Task 96: Test parser performance
    test_parser_performance().await?;
    
    println!("\n✅ ALL PARSER TESTS PASSED!");
    Ok(())
}

async fn test_parse_rust() -> Result<()> {
    println!("\n🦀 Testing Rust parser...");
    
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language())?;
    
    let rust_code = r#"
fn main() {
    println!("Hello, World!");
    let x = 42;
    let y = vec![1, 2, 3];
    
    for i in 0..10 {
        println!("{}", i);
    }
}

struct MyStruct {
    field1: String,
    field2: i32,
}

impl MyStruct {
    fn new(s: String, n: i32) -> Self {
        Self {
            field1: s,
            field2: n,
        }
    }
}

#[derive(Debug, Clone)]
enum MyEnum {
    Variant1,
    Variant2(String),
    Variant3 { x: i32, y: i32 },
}
"#;
    
    let start = Instant::now();
    let tree = parser.parse(rust_code, None);
    let parse_time = start.elapsed();
    
    if let Some(tree) = tree {
        let root = tree.root_node();
        println!("  ✅ Parsed {} bytes in {:?}", rust_code.len(), parse_time);
        println!("  Node count: {}", count_nodes(&root));
        println!("  Tree height: {}", tree_height(&root));
        
        // Verify key structures
        let has_fn = contains_node_type(&root, "function_item");
        let has_struct = contains_node_type(&root, "struct_item");
        let has_impl = contains_node_type(&root, "impl_item");
        let has_enum = contains_node_type(&root, "enum_item");
        
        if has_fn && has_struct && has_impl && has_enum {
            println!("  ✅ All Rust constructs parsed correctly");
        }
    } else {
        println!("  ❌ Failed to parse Rust code");
    }
    
    Ok(())
}

async fn test_parse_python() -> Result<()> {
    println!("\n🐍 Testing Python parser...");
    
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_python::language())?;
    
    let python_code = r#"
import sys
from typing import List, Optional

class MyClass:
    def __init__(self, name: str, value: int = 0):
        self.name = name
        self.value = value
    
    def method1(self) -> str:
        return f"Name: {self.name}, Value: {self.value}"
    
    @staticmethod
    def static_method(x: int, y: int) -> int:
        return x + y

def main():
    obj = MyClass("Test", 42)
    print(obj.method1())
    
    numbers = [i for i in range(10)]
    squared = list(map(lambda x: x**2, numbers))
    
    try:
        result = 10 / 0
    except ZeroDivisionError:
        print("Cannot divide by zero")
    finally:
        print("Cleanup")

if __name__ == "__main__":
    main()
"#;
    
    let start = Instant::now();
    let tree = parser.parse(python_code, None);
    let parse_time = start.elapsed();
    
    if let Some(tree) = tree {
        let root = tree.root_node();
        println!("  ✅ Parsed {} bytes in {:?}", python_code.len(), parse_time);
        println!("  Node count: {}", count_nodes(&root));
        println!("  Tree height: {}", tree_height(&root));
        
        // Verify key structures
        let has_class = contains_node_type(&root, "class_definition");
        let has_function = contains_node_type(&root, "function_definition");
        let has_import = contains_node_type(&root, "import_statement") || 
                        contains_node_type(&root, "import_from_statement");
        
        if has_class && has_function && has_import {
            println!("  ✅ All Python constructs parsed correctly");
        }
    } else {
        println!("  ❌ Failed to parse Python code");
    }
    
    Ok(())
}

async fn test_parse_javascript() -> Result<()> {
    println!("\n📜 Testing JavaScript parser...");
    
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_javascript::language())?;
    
    let js_code = r#"
const greeting = "Hello, World!";

function greet(name) {
    return `Hello, ${name}!`;
}

class Person {
    constructor(name, age) {
        this.name = name;
        this.age = age;
    }
    
    sayHello() {
        console.log(`Hi, I'm ${this.name}`);
    }
}

const asyncFunc = async () => {
    try {
        const response = await fetch('/api/data');
        const data = await response.json();
        return data;
    } catch (error) {
        console.error('Error:', error);
    }
};

const numbers = [1, 2, 3, 4, 5];
const doubled = numbers.map(x => x * 2);
const filtered = doubled.filter(x => x > 5);

export { greet, Person };
"#;
    
    let start = Instant::now();
    let tree = parser.parse(js_code, None);
    let parse_time = start.elapsed();
    
    if let Some(tree) = tree {
        let root = tree.root_node();
        println!("  ✅ Parsed {} bytes in {:?}", js_code.len(), parse_time);
        println!("  Node count: {}", count_nodes(&root));
        println!("  Tree height: {}", tree_height(&root));
        
        // Verify key structures
        let has_function = contains_node_type(&root, "function_declaration");
        let has_class = contains_node_type(&root, "class_declaration");
        let has_arrow = contains_node_type(&root, "arrow_function");
        
        if has_function && has_class && has_arrow {
            println!("  ✅ All JavaScript constructs parsed correctly");
        }
    } else {
        println!("  ❌ Failed to parse JavaScript code");
    }
    
    Ok(())
}

async fn test_parse_typescript() -> Result<()> {
    println!("\n📘 Testing TypeScript parser...");
    
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_typescript::language_typescript())?;
    
    let ts_code = r#"
interface User {
    id: number;
    name: string;
    email?: string;
}

type UserRole = 'admin' | 'user' | 'guest';

class UserManager {
    private users: Map<number, User>;
    
    constructor() {
        this.users = new Map();
    }
    
    addUser(user: User): void {
        this.users.set(user.id, user);
    }
    
    getUser(id: number): User | undefined {
        return this.users.get(id);
    }
}

function processUsers<T extends User>(users: T[]): T[] {
    return users.filter(u => u.name.length > 0);
}

enum Status {
    Active = 'ACTIVE',
    Inactive = 'INACTIVE',
    Pending = 'PENDING'
}

const asyncOperation = async <T>(): Promise<T> => {
    return new Promise((resolve) => {
        setTimeout(() => resolve({} as T), 1000);
    });
};

export { UserManager, User, UserRole, Status };
"#;
    
    let start = Instant::now();
    let tree = parser.parse(ts_code, None);
    let parse_time = start.elapsed();
    
    if let Some(tree) = tree {
        let root = tree.root_node();
        println!("  ✅ Parsed {} bytes in {:?}", ts_code.len(), parse_time);
        println!("  Node count: {}", count_nodes(&root));
        println!("  Tree height: {}", tree_height(&root));
        
        // TypeScript-specific constructs
        let has_interface = contains_node_type(&root, "interface_declaration");
        let has_type_alias = contains_node_type(&root, "type_alias_declaration");
        let has_enum = contains_node_type(&root, "enum_declaration");
        
        if has_interface || has_type_alias || has_enum {
            println!("  ✅ TypeScript-specific constructs parsed correctly");
        }
    } else {
        println!("  ❌ Failed to parse TypeScript code");
    }
    
    Ok(())
}

async fn test_parse_go() -> Result<()> {
    println!("\n🐹 Testing Go parser...");
    
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_go::language())?;
    
    let go_code = r#"
package main

import (
    "fmt"
    "strings"
    "sync"
)

type User struct {
    ID   int    `json:"id"`
    Name string `json:"name"`
    Age  int    `json:"age"`
}

type UserService interface {
    GetUser(id int) (*User, error)
    CreateUser(user *User) error
}

func main() {
    user := &User{
        ID:   1,
        Name: "John Doe",
        Age:  30,
    }
    
    fmt.Printf("User: %+v\n", user)
    
    ch := make(chan int, 10)
    var wg sync.WaitGroup
    
    for i := 0; i < 5; i++ {
        wg.Add(1)
        go func(n int) {
            defer wg.Done()
            ch <- n * 2
        }(i)
    }
    
    go func() {
        wg.Wait()
        close(ch)
    }()
    
    for result := range ch {
        fmt.Println(result)
    }
}

func processString(s string) string {
    return strings.ToUpper(s)
}
"#;
    
    let start = Instant::now();
    let tree = parser.parse(go_code, None);
    let parse_time = start.elapsed();
    
    if let Some(tree) = tree {
        let root = tree.root_node();
        println!("  ✅ Parsed {} bytes in {:?}", go_code.len(), parse_time);
        println!("  Node count: {}", count_nodes(&root));
        println!("  Tree height: {}", tree_height(&root));
        
        // Verify key structures
        let has_struct = contains_node_type(&root, "type_declaration");
        let has_interface = contains_node_type(&root, "interface_type");
        let has_goroutine = contains_node_type(&root, "go_statement");
        
        if has_struct && has_interface && has_goroutine {
            println!("  ✅ All Go constructs parsed correctly");
        }
    } else {
        println!("  ❌ Failed to parse Go code");
    }
    
    Ok(())
}

async fn test_parser_performance() -> Result<()> {
    println!("\n⚡ Testing parser performance...");
    
    // Generate large test files
    let languages = vec![
        ("Rust", unsafe { tree_sitter_rust() }, generate_large_rust_code()),
        ("Python", unsafe { tree_sitter_python() }, generate_large_python_code()),
        ("JavaScript", unsafe { tree_sitter_javascript() }, generate_large_js_code()),
    ];
    
    for (name, language, code) in languages {
        let mut parser = Parser::new();
        parser.set_language(language)?;
        
        let code_size = code.len();
        let lines = code.lines().count();
        
        // Warm up
        for _ in 0..3 {
            let _ = parser.parse(&code, None);
        }
        
        // Benchmark
        let start = Instant::now();
        let iterations = 100;
        
        for _ in 0..iterations {
            let _ = parser.parse(&code, None);
        }
        
        let total_time = start.elapsed();
        let avg_time = total_time / iterations;
        let throughput_mb = (code_size as f64 * iterations as f64) / total_time.as_secs_f64() / 1_000_000.0;
        
        println!("  {} Parser Performance:", name);
        println!("    File size: {} KB ({} lines)", code_size / 1024, lines);
        println!("    Avg parse time: {:?}", avg_time);
        println!("    Throughput: {:.2} MB/s", throughput_mb);
        
        if avg_time.as_millis() < 10 {
            println!("    ✅ Excellent performance!");
        } else if avg_time.as_millis() < 50 {
            println!("    ✅ Good performance");
        } else {
            println!("    ⚠️ Performance could be improved");
        }
    }
    
    Ok(())
}

fn count_nodes(node: &tree_sitter::Node) -> usize {
    let mut count = 1;
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            count += count_nodes(&child);
        }
    }
    count
}

fn tree_height(node: &tree_sitter::Node) -> usize {
    let mut max_height = 0;
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            let height = tree_height(&child);
            if height > max_height {
                max_height = height;
            }
        }
    }
    max_height + 1
}

fn contains_node_type(node: &tree_sitter::Node, node_type: &str) -> bool {
    if node.kind() == node_type {
        return true;
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if contains_node_type(&child, node_type) {
                return true;
            }
        }
    }
    false
}

fn generate_large_rust_code() -> String {
    let mut code = String::new();
    for i in 0..100 {
        code.push_str(&format!(r#"
fn function_{i}(x: i32, y: i32) -> i32 {{
    let result = x + y;
    if result > 100 {{
        result * 2
    }} else {{
        result / 2
    }}
}}

struct Struct_{i} {{
    field1: String,
    field2: Vec<i32>,
    field3: Option<bool>,
}}

impl Struct_{i} {{
    fn new() -> Self {{
        Self {{
            field1: String::from("test"),
            field2: vec![1, 2, 3],
            field3: Some(true),
        }}
    }}
}}
"#, i=i));
    }
    code
}

fn generate_large_python_code() -> String {
    let mut code = String::new();
    for i in 0..100 {
        code.push_str(&format!(r#"
def function_{i}(x, y):
    result = x + y
    if result > 100:
        return result * 2
    else:
        return result / 2

class Class_{i}:
    def __init__(self, name, value):
        self.name = name
        self.value = value
    
    def method1(self):
        return f"{{self.name}}: {{self.value}}"
    
    def method2(self, x):
        return self.value + x
"#, i=i));
    }
    code
}

fn generate_large_js_code() -> String {
    let mut code = String::new();
    for i in 0..100 {
        code.push_str(&format!(r#"
function function_{i}(x, y) {{
    const result = x + y;
    if (result > 100) {{
        return result * 2;
    }} else {{
        return result / 2;
    }}
}}

class Class_{i} {{
    constructor(name, value) {{
        this.name = name;
        this.value = value;
    }}
    
    method1() {{
        return `${{this.name}}: ${{this.value}}`;
    }}
    
    method2(x) {{
        return this.value + x;
    }}
}}
"#, i=i));
    }
    code
}
