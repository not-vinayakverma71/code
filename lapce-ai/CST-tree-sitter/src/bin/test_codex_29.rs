fn main() {
    println!("Testing 29 Codex Languages");
    println!("=" .repeat(50));
    
    // Test simple JavaScript
    let js_code = "function test() { return 42; }";
    test_language("test.js", js_code, "JavaScript");
    
    // Test Python
    let py_code = "def test():\n    return 42\n\nclass MyClass:\n    pass";
    test_language("test.py", py_code, "Python");
    
    // Test Rust
    let rust_code = "fn test() -> i32 {\n    42\n}";
    test_language("test.rs", rust_code, "Rust");
    
    // Test TypeScript
    let ts_code = "interface Test { value: number; }";
    test_language("test.ts", ts_code, "TypeScript");
    
    // Test all other languages
    let languages = vec![
        ("test.tsx", "const C = () => <div>Test</div>;", "TSX"),
        ("test.go", "func test() int { return 42 }", "Go"),
        ("test.c", "int test() { return 42; }", "C"),
        ("test.cpp", "class Test { public: int get() { return 42; } };", "C++"),
        ("test.cs", "public class Test { public int Get() => 42; }", "C#"),
        ("test.rb", "class Test\n  def get\n    42\n  end\nend", "Ruby"),
        ("test.java", "public class Test { public int get() { return 42; } }", "Java"),
        ("test.php", "<?php class Test { public function get() { return 42; } }", "PHP"),
        ("test.swift", "class Test { func get() -> Int { return 42 } }", "Swift"),
        ("test.kt", "class Test { fun get(): Int = 42 }", "Kotlin"),
        ("test.css", ".test { margin: 10px; padding: 20px; }", "CSS"),
        ("test.html", "<div><h1>Test</h1><p>Content</p></div>", "HTML"),
        ("test.ml", "let test x = x + 42", "OCaml"),
        ("test.sol", "contract Test { function get() returns (uint) { return 42; } }", "Solidity"),
        ("test.toml", "[package]\nname = \"test\"", "TOML"),
        ("test.vue", "<template><div>Test</div></template>", "Vue"),
        ("test.lua", "function test() return 42 end", "Lua"),
        ("test.rdl", "addrmap test { reg r { field {} f; }; };", "SystemRDL"),
        ("test.tla", "MODULE Test\nVARIABLE x", "TLA+"),
        ("test.zig", "pub fn test() i32 { return 42; }", "Zig"),
        ("test.ejs", "<% if (true) { %><div>Test</div><% } %>", "EJS"),
        ("test.el", "(defun test (x) (+ x 42))", "Elisp"),
        ("test.ex", "defmodule Test do\n  def get, do: 42\nend", "Elixir"),
        ("test.scala", "class Test { def get: Int = 42 }", "Scala"),
        ("test.md", "# Header\n\n## Section\n\nContent", "Markdown"),
    ];
    
    for (file, code, name) in languages {
        test_language(file, code, name);
    }
    
    println!("\nDone testing 29 languages");
}

fn test_language(file: &str, code: &str, name: &str) {
    use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;
    
    print!("{:<15} ", format!("{}:", name));
    match parse_source_code_definitions_for_file(file, code) {
        Some(_) => println!("✅"),
        None => println!("❌"),
    }
}
