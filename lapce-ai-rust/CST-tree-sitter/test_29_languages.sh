#!/bin/bash

echo "Testing 29 Codex Languages"
echo "=========================="

cd /home/verma/lapce/lapce-tree-sitter

# Build first
cargo build --release 2>&1 | tail -5

# Create simple test
cat > test_inline.rs << 'EOF'
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
EOF

# Compile and run
rustc --edition 2021 -L target/release/deps test_inline.rs -o test_inline \
  --extern lapce_tree_sitter=target/release/liblapse_tree_sitter.rlib \
  --extern tree_sitter=target/release/deps/libtree_sitter*.rlib \
  --extern tree_sitter_javascript=target/release/deps/libtree_sitter_javascript*.rlib \
  --extern tree_sitter_typescript=target/release/deps/libtree_sitter_typescript*.rlib \
  --extern tree_sitter_python=target/release/deps/libtree_sitter_python*.rlib \
  --extern tree_sitter_rust=target/release/deps/libtree_sitter_rust*.rlib \
  --extern tree_sitter_go=target/release/deps/libtree_sitter_go*.rlib \
  --extern tree_sitter_c=target/release/deps/libtree_sitter_c*.rlib \
  --extern tree_sitter_cpp=target/release/deps/libtree_sitter_cpp*.rlib \
  --extern tree_sitter_c_sharp=target/release/deps/libtree_sitter_c_sharp*.rlib \
  --extern tree_sitter_ruby=target/release/deps/libtree_sitter_ruby*.rlib \
  --extern tree_sitter_java=target/release/deps/libtree_sitter_java*.rlib \
  --extern tree_sitter_php=target/release/deps/libtree_sitter_php*.rlib \
  --extern tree_sitter_swift=target/release/deps/libtree_sitter_swift*.rlib \
  --extern tree_sitter_kotlin=target/release/deps/libtree_sitter_kotlin*.rlib \
  --extern tree_sitter_css=target/release/deps/libtree_sitter_css*.rlib \
  --extern tree_sitter_html=target/release/deps/libtree_sitter_html*.rlib \
  --extern tree_sitter_ocaml=target/release/deps/libtree_sitter_ocaml*.rlib \
  --extern tree_sitter_solidity=target/release/deps/libtree_sitter_solidity*.rlib \
  --extern tree_sitter_toml=target/release/deps/libtree_sitter_toml*.rlib \
  --extern tree_sitter_vue=target/release/deps/libtree_sitter_vue*.rlib \
  --extern tree_sitter_lua=target/release/deps/libtree_sitter_lua*.rlib \
  --extern tree_sitter_systemrdl=target/release/deps/libtree_sitter_systemrdl*.rlib \
  --extern tree_sitter_tlaplus=target/release/deps/libtree_sitter_tlaplus*.rlib \
  --extern tree_sitter_zig=target/release/deps/libtree_sitter_zig*.rlib \
  --extern tree_sitter_embedded_template=target/release/deps/libtree_sitter_embedded_template*.rlib \
  --extern tree_sitter_elisp=target/release/deps/libtree_sitter_elisp*.rlib \
  --extern tree_sitter_elixir=target/release/deps/libtree_sitter_elixir*.rlib \
  --extern tree_sitter_scala=target/release/deps/libtree_sitter_scala*.rlib \
  --extern tree_sitter_bash=target/release/deps/libtree_sitter_bash*.rlib \
  --extern tree_sitter_json=target/release/deps/libtree_sitter_json*.rlib \
  --extern tree_sitter_elm=target/release/deps/libtree_sitter_elm*.rlib \
  2>&1

if [ -f test_inline ]; then
    echo "Running test..."
    ./test_inline
else
    echo "Compilation failed"
fi
