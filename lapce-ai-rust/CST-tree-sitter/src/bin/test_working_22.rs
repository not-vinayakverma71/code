use lapce_tree_sitter::codex_exact_format::parse_source_code_definitions_for_file;

fn main() {
    println!("Testing 22 Working Languages\n");
    println!("="  .repeat(50));
    
    let mut passed = 0;
    let mut failed = Vec::new();
    
    // Test only the languages that should work based on available dependencies
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
        ("test.css", ".test {\n    margin: 10px;\n    padding: 20px;\n}", "CSS"),
        ("test.html", "<div>\n    <h1>Test</h1>\n    <p>Content</p>\n</div>", "HTML"),
        ("test.ml", "let test x =\n  x + 42", "OCaml"),
        ("test.lua", "function test()\n    return 42\nend", "Lua"),
        ("test.ex", "defmodule Test do\n  def get, do: 42\nend", "Elixir"),
        ("test.scala", "class Test {\n  def get: Int = 42\n}", "Scala"),
        ("test.sh", "test_function() {\n    echo \"test\"\n    return 0\n}", "Bash"),
        ("test.json", "{\n  \"name\": \"test\",\n  \"value\": 42\n}", "JSON"),
        ("test.elm", "module Test exposing (..)\n\ntest : Int -> Int\ntest x =\n    x + 42", "Elm"),
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
                failed.push(name);
            }
        }
    }
    
    // Also test Markdown separately
    print!("{:15} ", "Markdown:");
    let md_code = "# Header\n\n## Section One\n\nContent here.\n\n## Section Two\n\nMore content.";
    match parse_source_code_definitions_for_file("test.md", md_code) {
        Some(_) => {
            println!("✅");
            passed += 1;
        }
        None => {
            println!("❌");
            failed.push("Markdown");
        }
    }
    
    println!("\n" + &"=".repeat(50));
    println!("Results: {}/23 passed", passed);
    
    if !failed.is_empty() {
        println!("\nFailed languages:");
        for lang in &failed {
            println!("  - {}", lang);
        }
    }
    
    if passed == 23 {
        println!("\n🎉 SUCCESS: All 23 languages working!");
    } else {
        println!("\n⚠️  {} languages need fixing", failed.len());
    }
}
