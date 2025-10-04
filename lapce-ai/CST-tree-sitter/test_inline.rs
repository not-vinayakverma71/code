use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;

fn main() {
    let mut passed = 0;
    let mut failed = 0;
    
    // Test each language
    let tests = vec![
        ("test.js", "function test() {\n    return 42;\n}", "JavaScript"),
        ("test.ts", "interface Test {\n    value: number;\n}", "TypeScript"),
        ("test.tsx", "const Component = () => <div>Test</div>;", "TSX"),
        ("test.py", "def test():\n    return 42\n\nclass MyClass:\n    pass", "Python"),
        ("test.rs", "fn test() -> i32 {\n    42\n}", "Rust"),
        ("test.go", "func test() int {\n    return 42\n}", "Go"),
        ("test.c", "int test() {\n    return 42;\n}", "C"),
        ("test.cpp", "class Test {\npublic:\n    int get() { return 42; }\n};", "C++"),
        ("test.cs", "public class Test {\n    public int Get() => 42;\n}", "C#"),
        ("test.rb", "class Test\n  def get\n    42\n  end\nend", "Ruby"),
        ("test.java", "public class Test {\n    public int get() { return 42; }\n}", "Java"),
        ("test.php", "<?php\nclass Test {\n    public function get() { return 42; }\n}", "PHP"),
        ("test.swift", "class Test {\n    func get() -> Int { return 42 }\n}", "Swift"),
        ("test.kt", "class Test {\n    fun get(): Int = 42\n}", "Kotlin"),
        ("test.css", ".test {\n    margin: 10px;\n    padding: 20px;\n}", "CSS"),
        ("test.html", "<div>\n    <h1>Test</h1>\n    <p>Content</p>\n</div>", "HTML"),
        ("test.ml", "let test x =\n  x + 42", "OCaml"),
        ("test.sol", "contract Test {\n    function get() returns (uint) {\n        return 42;\n    }\n}", "Solidity"),
        ("test.toml", "[package]\nname = \"test\"\nversion = \"1.0\"", "TOML"),
        ("test.vue", "<template>\n  <div>Test</div>\n</template>", "Vue"),
        ("test.lua", "function test()\n    return 42\nend", "Lua"),
        ("test.rdl", "addrmap test {\n    reg r {\n        field {} f;\n    };\n};", "SystemRDL"),
        ("test.tla", "MODULE Test\nVARIABLE x\nInit == x = 0", "TLA+"),
        ("test.zig", "pub fn test() i32 {\n    return 42;\n}", "Zig"),
        ("test.ejs", "<% if (true) { %>\n  <div>Test</div>\n<% } %>", "EJS"),
        ("test.el", "(defun test (x)\n  (+ x 42))", "Elisp"),
        ("test.ex", "defmodule Test do\n  def get, do: 42\nend", "Elixir"),
        ("test.scala", "class Test {\n  def get: Int = 42\n}", "Scala"),
        ("test.md", "# Header\n\n## Section\n\nContent here.", "Markdown"),
    ];
    
    for (file, code, name) in tests {
        print!("{:15} ", format!("{}:", name));
        match parse_source_code_definitions_for_file(file, code) {
            Some(_) => {
                println!("✅");
                passed += 1;
            }
            None => {
                println!("❌");
                failed += 1;
            }
        }
    }
    
    println!("\n==========================");
    println!("Results: {}/{} passed", passed, passed + failed);
    
    if passed == 29 {
        println!("🎉 All 29 languages working!");
    } else {
        println!("⚠️  {} languages need fixing", failed);
    }
}
