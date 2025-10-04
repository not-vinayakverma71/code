use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;

fn main() {
    println!("Testing Each Language Individually\n");
    println!("="  .repeat(50));
    
    let mut passed = 0;
    let mut failed = Vec::new();
    
    // Test 1: JavaScript
    print!("1. JavaScript: ");
    let js_code = r#"function testFunction() {
    console.log("Hello");
    return 42;
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.js", js_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("JavaScript");
    }
    
    // Test 2: TypeScript
    print!("2. TypeScript: ");
    let ts_code = r#"interface TestInterface {
    value: number;
    name: string;
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.ts", ts_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("TypeScript");
    }
    
    // Test 3: TSX
    print!("3. TSX: ");
    let tsx_code = r#"const Component = () => {
    return <div>Hello World</div>;
};"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.tsx", tsx_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("TSX");
    }
    
    // Test 4: Python
    print!("4. Python: ");
    let py_code = r#"def test_function(x, y):
    result = x + y
    return result

class TestClass:
    def __init__(self):
        self.value = 10"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.py", py_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Python");
    }
    
    // Test 5: Rust
    print!("5. Rust: ");
    let rs_code = r#"fn test_function() -> i32 {
    let x = 10;
    x + 20
}

struct TestStruct {
    value: i32,
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.rs", rs_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Rust");
    }
    
    // Test 6: Go
    print!("6. Go: ");
    let go_code = r#"func testFunction(x int) int {
    result := x + 10
    return result
}

type TestStruct struct {
    Value int
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.go", go_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Go");
    }
    
    // Test 7: C
    print!("7. C: ");
    let c_code = r#"int test_function(int x) {
    return x + 10;
}

struct TestStruct {
    int value;
};"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.c", c_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("C");
    }
    
    // Test 8: C++
    print!("8. C++: ");
    let cpp_code = r#"class TestClass {
public:
    int getValue() {
        return 42;
    }
};"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.cpp", cpp_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("C++");
    }
    
    // Test 9: C#
    print!("9. C#: ");
    let cs_code = r#"public class TestClass {
    public int GetValue() {
        return 42;
    }
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.cs", cs_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("C#");
    }
    
    // Test 10: Ruby
    print!("10. Ruby: ");
    let rb_code = r#"class TestClass
  def get_value
    return 42
  end
end"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.rb", rb_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Ruby");
    }
    
    // Test 11: Java
    print!("11. Java: ");
    let java_code = r#"public class TestClass {
    public int getValue() {
        return 42;
    }
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.java", java_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Java");
    }
    
    // Test 12: PHP
    print!("12. PHP: ");
    let php_code = r#"<?php
class TestClass {
    public function getValue() {
        return 42;
    }
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.php", php_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("PHP");
    }
    
    // Test 13: Swift
    print!("13. Swift: ");
    let swift_code = r#"class TestClass {
    func getValue() -> Int {
        return 42
    }
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.swift", swift_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Swift");
    }
    
    // Test 14: Kotlin
    print!("14. Kotlin: ");
    let kt_code = r#"class TestClass {
    fun getValue(): Int {
        return 42
    }
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.kt", kt_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Kotlin");
    }
    
    // Test 15: CSS
    print!("15. CSS: ");
    let css_code = r#".test-class {
    margin: 10px;
    padding: 20px;
    color: #333;
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.css", css_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("CSS");
    }
    
    // Test 16: HTML
    print!("16. HTML: ");
    let html_code = r#"<div class="container">
    <h1>Test Header</h1>
    <p>Test paragraph</p>
</div>"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.html", html_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("HTML");
    }
    
    // Test 17: OCaml
    print!("17. OCaml: ");
    let ml_code = r#"let test_function x =
  x + 10

type test_type = int"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.ml", ml_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("OCaml");
    }
    
    // Test 18: Solidity
    print!("18. Solidity: ");
    let sol_code = r#"contract TestContract {
    function getValue() public returns (uint) {
        return 42;
    }
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.sol", sol_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Solidity");
    }
    
    // Test 19: TOML
    print!("19. TOML: ");
    let toml_code = r#"[package]
name = "test"
version = "1.0.0"

[dependencies]
test-lib = "0.1""#;
    if let Some(_) = parse_source_code_definitions_for_file("test.toml", toml_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("TOML");
    }
    
    // Test 20: Vue
    print!("20. Vue: ");
    let vue_code = r#"<template>
  <div>{{ message }}</div>
</template>

<script>
export default {
  data() {
    return { message: 'Test' }
  }
}
</script>"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.vue", vue_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Vue");
    }
    
    // Test 21: Lua
    print!("21. Lua: ");
    let lua_code = r#"function test_function()
    local x = 10
    return x + 20
end"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.lua", lua_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Lua");
    }
    
    // Test 22: SystemRDL
    print!("22. SystemRDL: ");
    let rdl_code = r#"addrmap test_map {
    reg test_reg {
        field {} test_field;
    };
};"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.rdl", rdl_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("SystemRDL");
    }
    
    // Test 23: TLA+
    print!("23. TLA+: ");
    let tla_code = r#"MODULE Test
VARIABLE x
Init == x = 0
Next == x' = x + 1"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.tla", tla_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("TLA+");
    }
    
    // Test 24: Zig
    print!("24. Zig: ");
    let zig_code = r#"pub fn testFunction() i32 {
    return 42;
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.zig", zig_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Zig");
    }
    
    // Test 25: Embedded Template (EJS)
    print!("25. EJS: ");
    let ejs_code = r#"<% if (user) { %>
  <h2><%= user.name %></h2>
<% } %>"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.ejs", ejs_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("EJS");
    }
    
    // Test 26: Elisp
    print!("26. Elisp: ");
    let el_code = r#"(defun test-function (x)
  "Test function"
  (+ x 42))"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.el", el_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Elisp");
    }
    
    // Test 27: Elixir
    print!("27. Elixir: ");
    let ex_code = r#"defmodule TestModule do
  def test_function do
    42
  end
end"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.ex", ex_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Elixir");
    }
    
    // Test 28: Scala
    print!("28. Scala: ");
    let scala_code = r#"class TestClass {
  def getValue: Int = 42
}

object TestObject {
  def main(args: Array[String]): Unit = {
    println("Test")
  }
}"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.scala", scala_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Scala");
    }
    
    // Test 29: Markdown
    print!("29. Markdown: ");
    let md_code = r#"# Main Header

## Section One

This is a test paragraph.

## Section Two

```python
def test():
    pass
```"#;
    if let Some(_) = parse_source_code_definitions_for_file("test.md", md_code) {
        println!("✅");
        passed += 1;
    } else {
        println!("❌");
        failed.push("Markdown");
    }
    
    println!("\n" + &"=".repeat(50));
    println!("Results: {}/29 passed", passed);
    
    if !failed.is_empty() {
        println!("\nFailed languages:");
        for lang in &failed {
            println!("  - {}", lang);
        }
    }
    
    if passed == 29 {
        println!("\n🎉 SUCCESS: All 29 languages working!");
    } else {
        println!("\n⚠️  {} languages need fixing", failed.len());
    }
}
