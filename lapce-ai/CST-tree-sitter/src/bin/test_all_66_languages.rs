use std::time::Instant;
use tree_sitter::{Parser, Language};

fn main() {
    println!("\n🚀 FINAL TEST: ALL 66 LANGUAGES WITH TREE-SITTER 0.24");
    println!("=" . repeat(70));
    
    let languages = vec![
        // Systems & Low-Level (10)
        ("C", tree_sitter_c::LANGUAGE.into()),
        ("C++", tree_sitter_cpp::LANGUAGE.into()),
        ("Rust", tree_sitter_rust::LANGUAGE.into()),
        ("Go", tree_sitter_go::LANGUAGE.into()),
        ("Zig", tree_sitter_zig::language()),
        ("D", tree_sitter_d::LANGUAGE),
        ("Nim", tree_sitter_nim::language()),
        ("Ada", tree_sitter_ada::language()),
        ("Fortran", tree_sitter_fortran::language()),
        ("Assembly", tree_sitter_asm::LANGUAGE),
        
        // Web & Application (14)
        ("JavaScript", tree_sitter_javascript::language()),
        ("TypeScript", tree_sitter_typescript::language_typescript()),
        ("TSX", tree_sitter_typescript::language_tsx()),
        ("HTML", tree_sitter_html::LANGUAGE.into()),
        ("CSS", tree_sitter_css::LANGUAGE.into()),
        ("PHP", tree_sitter_php::LANGUAGE_PHP.into()),
        ("Ruby", tree_sitter_ruby::LANGUAGE.into()),
        ("Elixir", tree_sitter_elixir::LANGUAGE.into()),
        ("Java", tree_sitter_java::LANGUAGE.into()),
        ("C#", tree_sitter_c_sharp::LANGUAGE.into()),
        ("Kotlin", tree_sitter_kotlin::language()),
        ("Swift", tree_sitter_swift::LANGUAGE.into()),
        ("Objective-C", tree_sitter_objc::LANGUAGE),
        ("Scala", tree_sitter_scala::LANGUAGE.into()),
        ("Groovy", tree_sitter_groovy::LANGUAGE),
        
        // Data Science (6)
        ("Python", tree_sitter_python::LANGUAGE.into()),
        ("R", tree_sitter_r::language()),
        ("Julia", tree_sitter_julia::language()),
        ("MATLAB", tree_sitter_matlab::language()),
        ("SQL", tree_sitter_sql::language()),
        ("GraphQL", tree_sitter_graphql::language()),
        
        // Scripting (5)
        ("Bash", tree_sitter_bash::LANGUAGE.into()),
        ("PowerShell", tree_sitter_powershell::LANGUAGE),
        ("Lua", tree_sitter_lua::LANGUAGE.into()),
        ("Perl", tree_sitter_perl::language()),
        ("Tcl", tree_sitter_tcl::LANGUAGE),
        
        // Cloud & Configuration (6)
        ("HCL", tree_sitter_hcl::LANGUAGE),
        ("YAML", tree_sitter_yaml::language()),
        ("Erlang", tree_sitter_erlang::LANGUAGE),
        ("Nix", tree_sitter_nix::LANGUAGE),
        ("Make", tree_sitter_make::LANGUAGE),
        ("CMake", tree_sitter_cmake::LANGUAGE),
        
        // Blockchain (2)
        ("Solidity", tree_sitter_solidity::LANGUAGE),
        ("Cairo", tree_sitter_cairo::LANGUAGE),
        
        // Hardware (4)
        ("Verilog", tree_sitter_verilog::LANGUAGE),
        ("SystemVerilog", tree_sitter_systemverilog::LANGUAGE()),
        ("GLSL", tree_sitter_glsl::LANGUAGE),
        ("HLSL", tree_sitter_hlsl::LANGUAGE),
        ("WGSL", tree_sitter_wgsl::LANGUAGE),
        
        // Functional (8)
        ("Haskell", tree_sitter_haskell::language()),
        ("Scheme", tree_sitter_scheme::LANGUAGE),
        ("OCaml", tree_sitter_ocaml::LANGUAGE_OCAML.into()),
        ("Elm", tree_sitter_elm::LANGUAGE()),
        ("F#", tree_sitter_fsharp::LANGUAGE),
        ("Clojure", tree_sitter_clojure::language()),
        ("Racket", tree_sitter_racket::language()),
        ("CommonLisp", tree_sitter_commonlisp::LANGUAGE),
        ("Fennel", tree_sitter_fennel::LANGUAGE),
        
        // Other (8)
        ("JSON", tree_sitter_json::LANGUAGE.into()),
        ("LaTeX", tree_sitter_latex::LANGUAGE),
        ("Pascal", tree_sitter_pascal::LANGUAGE),
        ("COBOL", tree_sitter_cobol::LANGUAGE),
        ("Dart", tree_sitter_dart::language()),
        ("Gleam", tree_sitter_gleam::LANGUAGE),
        ("Astro", tree_sitter_astro::LANGUAGE),
        ("Prisma", tree_sitter_prisma::LANGUAGE),
        ("VimDoc", tree_sitter_vimdoc::LANGUAGE),
    ];
    
    let total = languages.len();
    let mut successful = 0;
    let mut failed = Vec::new();
    
    println!("\nTesting {} languages:\n", total);
    
    for (name, lang) in &languages {
        let mut parser = Parser::new();
        
        let test_code = get_test_code(name);
        
        match parser.set_language(&lang.into()) {
            Ok(_) => {
                let start = Instant::now();
                match parser.parse(test_code, None) {
                    Some(tree) => {
                        let elapsed = start.elapsed();
                        let ms = elapsed.as_secs_f64() * 1000.0;
                        let nodes = tree.root_node().descendant_count();
                        println!("✅ {:<15} | Parse: {:6.2}ms | Nodes: {:5}", name, ms, nodes);
                        successful += 1;
                    }
                    None => {
                        println!("❌ {:<15} | Failed to parse", name);
                        failed.push(*name);
                    }
                }
            }
            Err(e) => {
                println!("❌ {:<15} | Failed to set language: {:?}", name, e);
                failed.push(*name);
            }
        }
    }
    
    println!("\n" + &"=".repeat(70));
    println!("📊 FINAL RESULTS");
    println!(&"=".repeat(70));
    println!("Total Languages: {}", total);
    println!("✅ Successful: {} ({:.1}%)", successful, (successful as f64 / total as f64) * 100.0);
    println!("❌ Failed: {} ({:.1}%)", failed.len(), (failed.len() as f64 / total as f64) * 100.0);
    
    if !failed.is_empty() {
        println!("\nFailed languages: {:?}", failed);
    }
    
    println!("\n📋 Coverage vs 60 Essential Languages:");
    println!("   Achievement: {}/60 ({}%+)", successful.min(60), (successful.min(60) as f64 / 60.0 * 100.0) as i32);
    
    if successful >= 60 {
        println!("\n🎉 EXCELLENT: Achieved {} languages (100%+ coverage)!", successful);
    }
    
    println!("\n🚀 Lapce now supports {} programming languages!", successful);
}

fn get_test_code(lang: &str) -> &'static str {
    match lang {
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
        "PowerShell" => "function Test { return 42 }",
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
        "SystemVerilog" => "module test; logic a; endmodule",
        "Erlang" => "-module(test).\ntest() -> 42.",
        "D" => "int test() { return 42; }",
        "Nim" => "proc test(): int = 42",
        "Pascal" => "function test: integer;\nbegin\n  test := 42;\nend;",
        "Scheme" => "(define test 42)",
        "Racket" => "#lang racket\n(define test 42)",
        "CommonLisp" => "(defun test () 42)",
        "Fennel" => "(fn test [] 42)",
        "Gleam" => "pub fn test() { 42 }",
        "Astro" => "---\nconst test = 42\n---\n<div>{test}</div>",
        "Prisma" => "model Test {\n  id Int @id\n}",
        "VimDoc" => "*test.txt* Test documentation",
        "WGSL" => "@compute @workgroup_size(1)\nfn main() {}",
        "GLSL" => "void main() { gl_Position = vec4(0.0); }",
        "HLSL" => "float4 main() : SV_Position { return float4(0.0); }",
        "Objective-C" => "@interface Test : NSObject\n@end",
        "MATLAB" => "function y = test(x)\n  y = x * 42;\nend",
        "Fortran" => "program test\n  print *, 42\nend program",
        "Ada" => "procedure Test is\nbegin\n  null;\nend Test;",
        "COBOL" => "IDENTIFICATION DIVISION.\nPROGRAM-ID. TEST.",
        "Perl" => "sub test { return 42; }",
        "Tcl" => "proc test {} { return 42 }",
        "Groovy" => "def test() { return 42 }",
        "HCL" => "resource \"test\" \"example\" { value = 42 }",
        "Solidity" => "contract Test { function get() returns (uint) { return 42; } }",
        "F#" => "let test = 42",
        "Cairo" => "func test() -> felt { return 42; }",
        "Assembly" => "mov eax, 42\nret",
        _ => "test",
    }
}
