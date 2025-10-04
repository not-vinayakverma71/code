use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;

fn main() {
    println!("Testing Each Language Individually\n");
    
    // Test JavaScript
    println!("1. JavaScript:");
    let js_code = r#"function testFunction() {
    console.log("Hello");
    return 42;
}

class TestClass {
    constructor() {
        this.value = 10;
    }
    
    getValue() {
        return this.value;
    }
}"#;
    test_and_show("test.js", js_code);
    
    // Test TypeScript
    println!("\n2. TypeScript:");
    let ts_code = r#"interface TestInterface {
    value: number;
    name: string;
}

class TestClass implements TestInterface {
    value: number;
    name: string;
    
    constructor(v: number, n: string) {
        this.value = v;
        this.name = n;
    }
}"#;
    test_and_show("test.ts", ts_code);
    
    // Test Python
    println!("\n3. Python:");
    let py_code = r#"def test_function(x, y):
    """Test function"""
    result = x + y
    return result

class TestClass:
    def __init__(self):
        self.value = 10
    
    def get_value(self):
        return self.value"#;
    test_and_show("test.py", py_code);
    
    // Test Rust
    println!("\n4. Rust:");
    let rs_code = r#"fn test_function() -> i32 {
    let x = 10;
    let y = 20;
    x + y
}

struct TestStruct {
    value: i32,
    name: String,
}

impl TestStruct {
    fn new(value: i32) -> Self {
        TestStruct {
            value,
            name: String::from("test"),
        }
    }
}"#;
    test_and_show("test.rs", rs_code);
    
    // Test Go
    println!("\n5. Go:");
    let go_code = r#"package main

func testFunction(x int, y int) int {
    result := x + y
    return result
}

type TestStruct struct {
    Value int
    Name  string
}

func (t *TestStruct) GetValue() int {
    return t.Value
}"#;
    test_and_show("test.go", go_code);
}

fn test_and_show(filename: &str, code: &str) {
    match parse_source_code_definitions_for_file(filename, code) {
        Some(result) => {
            println!("✅ SUCCESS");
            println!("Output:\n{}", result);
        }
        None => {
            println!("❌ FAILED - Returned None");
        }
    }
}
