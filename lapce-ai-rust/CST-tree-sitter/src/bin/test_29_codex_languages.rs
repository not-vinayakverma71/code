use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;

fn main() {
    println!("Testing 29 Codex Languages\n");
    println!("="  .repeat(50));
    
    // Test cases for all 29 Codex languages
    let test_cases = vec![
        ("test.js", "function test() { return 42; }", "JavaScript"),
        ("test.ts", "interface Test { value: number; }", "TypeScript"),
        ("test.tsx", "const Component = () => <div>Test</div>;", "TSX"),
        ("test.py", "def test():\n    return 42\n\nclass MyClass:\n    pass", "Python"),
        ("test.rs", "fn test() -> i32 {\n    42\n}\n\nstruct MyStruct {\n    value: i32\n}", "Rust"),
        ("test.go", "func test() int {\n    return 42\n}\n\ntype MyStruct struct {\n    Value int\n}", "Go"),
        ("test.c", "int test() {\n    return 42;\n}\n\nstruct MyStruct {\n    int value;\n};", "C"),
        ("test.cpp", "class Test {\npublic:\n    int getValue() { return 42; }\n};", "C++"),
        ("test.cs", "public class Test {\n    public int GetValue() => 42;\n}", "C#"),
        ("test.rb", "class Test\n  def get_value\n    42\n  end\nend", "Ruby"),
        ("test.java", "public class Test {\n    public int getValue() {\n        return 42;\n    }\n}", "Java"),
        ("test.php", "<?php\nclass Test {\n    public function getValue() {\n        return 42;\n    }\n}", "PHP"),
        ("test.swift", "class Test {\n    func getValue() -> Int {\n        return 42\n    }\n}", "Swift"),
        ("test.kt", "class Test {\n    fun getValue(): Int = 42\n}", "Kotlin"),
        ("test.css", ".test-class {\n    margin: 10px;\n    padding: 20px;\n}", "CSS"),
        ("test.html", "<div class=\"test\">\n    <h1>Test Header</h1>\n    <p>Test content</p>\n</div>", "HTML"),
        ("test.ml", "let test_function x =\n  x + 42\n\ntype my_type = int", "OCaml"),
        ("test.sol", "contract Test {\n    function getValue() public returns (uint) {\n        return 42;\n    }\n}", "Solidity"),
        ("test.toml", "[package]\nname = \"test\"\nversion = \"1.0.0\"\n\n[dependencies]\ntest = \"0.1\"", "TOML"),
        ("test.vue", "<template>\n  <div>{{ message }}</div>\n</template>\n\n<script>\nexport default {\n  data() {\n    return { message: 'Test' }\n  }\n}\n</script>", "Vue"),
        ("test.lua", "function test_function()\n    return 42\nend\n\nlocal MyClass = {}\nfunction MyClass:new()\n    return setmetatable({}, self)\nend", "Lua"),
        ("test.rdl", "addrmap test_map {\n    reg test_reg {\n        field {} test_field;\n    };\n};", "SystemRDL"),
        ("test.tla", "MODULE Test\nVARIABLE x\nInit == x = 0\nNext == x' = x + 1\nSpec == Init /\\ [][Next]_x", "TLA+"),
        ("test.zig", "pub fn testFunction() i32 {\n    return 42;\n}\n\nconst MyStruct = struct {\n    value: i32,\n};", "Zig"),
        ("test.ejs", "<% if (true) { %>\n  <div><%= test %></div>\n<% } %>\n\n<% function renderItem(item) { %>\n  <li><%= item %></li>\n<% } %>", "Embedded Template"),
        ("test.el", "(defun test-function (x)\n  \"Test function\"\n  (+ x 42))\n\n(defvar test-var 42)", "Elisp"),
        ("test.ex", "defmodule Test do\n  def get_value do\n    42\n  end\nend", "Elixir"),
        ("test.scala", "class Test {\n  def getValue: Int = 42\n}\n\nobject TestObject {\n  def main(args: Array[String]): Unit = {\n    println(\"Test\")\n  }\n}", "Scala"),
        ("test.md", "# Test Header\n\n## Section 1\n\nThis is a test paragraph.\n\n## Section 2\n\n```python\ndef test():\n    pass\n```", "Markdown"),
    ];
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (filename, code, lang_name) in test_cases {
        print!("Testing {:<20} ", format!("{} ({}):", lang_name, filename));
        
        match parse_source_code_definitions_for_file(filename, code) {
            Some(result) => {
                // Check if result contains expected format
                if result.contains(&format!("# {}", filename)) {
                    println!("✅ PASS");
                    passed += 1;
                    // Uncomment to see output:
                    // println!("  Output:\n{}", result.lines().take(3).collect::<Vec<_>>().join("\n"));
                } else {
                    println!("❌ FAIL (unexpected format)");
                    failed += 1;
                }
            }
            None => {
                println!("❌ FAIL (returned None)");
                failed += 1;
            }
        }
    }
    
    println!("\n" + &"=".repeat(50));
    println!("Results: {}/{} passed", passed, passed + failed);
    
    if passed == 29 {
        println!("🎉 SUCCESS: All 29 Codex languages working!");
    } else {
        println!("⚠️  {} languages failed", failed);
    }
}
