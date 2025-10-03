use std::time::Instant;
use tree_sitter::{Parser, Language};

fn main() {
    println!("\n🚀 TESTING ALL 39 LANGUAGES WITH TREE-SITTER 0.24");
    println!("=" . repeat(60));
    
    let languages = vec![
        // Original 22 languages
        ("JavaScript", tree_sitter_javascript::LANGUAGE),
        ("TypeScript", tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
        ("TSX", tree_sitter_typescript::LANGUAGE_TSX),
        ("Python", tree_sitter_python::LANGUAGE),
        ("Rust", tree_sitter_rust::LANGUAGE),
        ("Go", tree_sitter_go::LANGUAGE),
        ("C", tree_sitter_c::LANGUAGE),
        ("C++", tree_sitter_cpp::LANGUAGE),
        ("C#", tree_sitter_c_sharp::LANGUAGE),
        ("Ruby", tree_sitter_ruby::LANGUAGE),
        ("Java", tree_sitter_java::LANGUAGE),
        ("PHP", tree_sitter_php::LANGUAGE_PHP),
        ("Swift", tree_sitter_swift::LANGUAGE),
        ("Lua", tree_sitter_lua::LANGUAGE),
        ("Elixir", tree_sitter_elixir::LANGUAGE),
        ("Scala", tree_sitter_scala::LANGUAGE),
        ("Bash", tree_sitter_bash::LANGUAGE),
        ("CSS", tree_sitter_css::LANGUAGE),
        ("JSON", tree_sitter_json::LANGUAGE),
        ("HTML", tree_sitter_html::LANGUAGE),
        ("Elm", tree_sitter_elm::LANGUAGE),
        ("OCaml", tree_sitter_ocaml::LANGUAGE_OCAML),
        
        // New Phase 2 languages (17 added)
        ("Kotlin", tree_sitter_kotlin::language()),
        ("YAML", tree_sitter_yaml::language()),
        ("SQL", tree_sitter_sql::language()),
        ("GraphQL", tree_sitter_graphql::language()),
        ("Dart", tree_sitter_dart::language()),
        ("Haskell", tree_sitter_haskell::language()),
        ("R", tree_sitter_r::language()),
        ("Julia", tree_sitter_julia::language()),
        ("Clojure", tree_sitter_clojure::language()),
        ("Zig", tree_sitter_zig::language()),
        ("Nix", tree_sitter_nix::LANGUAGE),
        ("LaTeX", tree_sitter_latex::LANGUAGE),
        ("Make", tree_sitter_make::LANGUAGE),
        ("CMake", tree_sitter_cmake::LANGUAGE),
        ("Verilog", tree_sitter_verilog::LANGUAGE),
        ("Erlang", tree_sitter_erlang::LANGUAGE),
        ("D", tree_sitter_d::LANGUAGE),
    ];
    
    let mut successful = 0;
    let mut failed = 0;
    
    println!("\nTesting {} languages:", languages.len());
    println!();
    
    for (name, lang) in &languages {
        let mut parser = Parser::new();
        
        // Generate simple test code for each language
        let test_code = match *name {
            "JavaScript" => "function test() { return 42; }",
            "TypeScript" => "const x: number = 42;",
            "TSX" => "const C = () => <div>Test</div>;",
            "Python" => "def test():\n    return 42",
            "Rust" => "fn test() -> i32 { 42 }",
            "Go" => "func test() int { return 42 }",
            "C" => "int test() { return 42; }",
            "C++" => "class Test { public: int get() { return 42; } };",
            "C#" => "public class Test { public int Get() => 42; }",
            "Ruby" => "def test\n  42\nend",
            "Java" => "public class Test { public int get() { return 42; } }",
            "PHP" => "<?php function test() { return 42; }",
            "Swift" => "func test() -> Int { return 42 }",
            "Lua" => "function test() return 42 end",
            "Elixir" => "def test(), do: 42",
            "Scala" => "def test: Int = 42",
            "Bash" => "function test() { echo 42; }",
            "CSS" => ".test { margin: 42px; }",
            "JSON" => r#"{"test": 42}"#,
            "HTML" => "<div>Test</div>",
            "Elm" => "test : Int\ntest = 42",
            "OCaml" => "let test () = 42",
            "Kotlin" => "fun test(): Int = 42",
            "YAML" => "test: 42",
            "SQL" => "SELECT * FROM test WHERE id = 42;",
            "GraphQL" => "query { test { id } }",
            "Dart" => "int test() => 42;",
            "Haskell" => "test :: Int\ntest = 42",
            "R" => "test <- function() { 42 }",
            "Julia" => "function test() 42 end",
            "Clojure" => "(defn test [] 42)",
            "Zig" => "fn test() i32 { return 42; }",
            "Nix" => "{ test = 42; }",
            "LaTeX" => "\\section{Test}",
            "Make" => "test:\n\techo 42",
            "CMake" => "set(TEST 42)",
            "Verilog" => "module test; endmodule",
            "Erlang" => "-module(test).\ntest() -> 42.",
            "D" => "int test() { return 42; }",
            _ => "test",
        };
        
        match parser.set_language(&lang.into()) {
            Ok(_) => {
                let start = Instant::now();
                match parser.parse(test_code, None) {
                    Some(tree) => {
                        let elapsed = start.elapsed();
                        let nodes = tree.root_node().descendant_count();
                        println!("✅ {:<12} | Parse: {:6.2}ms | Nodes: {:5}", 
                                 name, 
                                 elapsed.as_secs_f64() * 1000.0,
                                 nodes);
                        successful += 1;
                    }
                    None => {
                        println!("❌ {:<12} | Failed to parse", name);
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                println!("❌ {:<12} | Failed to set language: {:?}", name, e);
                failed += 1;
            }
        }
    }
    
    println!("\n" + &"=".repeat(60));
    println!("📊 RESULTS");
    println!(&"=".repeat(60));
    println!("Total Languages: {}", languages.len());
    println!("✅ Successful: {} ({:.1}%)", successful, (successful as f64 / languages.len() as f64) * 100.0);
    println!("❌ Failed: {} ({:.1}%)", failed, (failed as f64 / languages.len() as f64) * 100.0);
    
    println!("\n📈 COVERAGE vs 60 ESSENTIAL LANGUAGES:");
    println!("   Current: {}/60 ({:.1}%)", successful, (successful as f64 / 60.0) * 100.0);
    
    if successful >= 35 {
        println!("\n🎉 SUCCESS: Reached {} languages (target was 22+)", successful);
    }
}
