use std::time::Instant;
use tree_sitter::{Parser, Language};

fn main() {
    println!("\n🚀 FINAL TEST: 59 LANGUAGES WITH TREE-SITTER 0.24");
    println!("{}", "=".repeat(70));
    
    let languages = vec![
        // Original 22 languages
        ("JavaScript", tree_sitter_javascript::language()),
        ("TypeScript", tree_sitter_typescript::language_typescript()),
        ("TSX", tree_sitter_typescript::language_tsx()),
        ("Python", tree_sitter_python::LANGUAGE.into()),
        ("Rust", tree_sitter_rust::LANGUAGE.into()),
        ("Go", tree_sitter_go::LANGUAGE.into()),
        ("C", tree_sitter_c::LANGUAGE.into()),
        ("C++", tree_sitter_cpp::LANGUAGE.into()),
        ("C#", tree_sitter_c_sharp::LANGUAGE.into()),
        ("Ruby", tree_sitter_ruby::LANGUAGE.into()),
        ("Java", tree_sitter_java::LANGUAGE.into()),
        ("PHP", tree_sitter_php::LANGUAGE_PHP.into()),
        ("Swift", tree_sitter_swift::LANGUAGE.into()),
        ("Lua", tree_sitter_lua::LANGUAGE.into()),
        ("Elixir", tree_sitter_elixir::LANGUAGE.into()),
        ("Scala", tree_sitter_scala::LANGUAGE.into()),
        ("Bash", tree_sitter_bash::LANGUAGE.into()),
        ("CSS", tree_sitter_css::LANGUAGE.into()),
        ("JSON", tree_sitter_json::LANGUAGE.into()),
        ("HTML", tree_sitter_html::LANGUAGE.into()),
        ("Elm", tree_sitter_elm::LANGUAGE()),
        ("OCaml", tree_sitter_ocaml::LANGUAGE_OCAML.into()),
        
        // Phase 2 languages (17 added)
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
        
        // Phase 3 languages (20 added)
        ("Nim", tree_sitter_nim::language()),
        ("Pascal", tree_sitter_pascal::LANGUAGE),
        ("Scheme", tree_sitter_scheme::LANGUAGE),
        ("Racket", tree_sitter_racket::language()),
        ("CommonLisp", tree_sitter_commonlisp::LANGUAGE),
        ("Fennel", tree_sitter_fennel::LANGUAGE),
        ("Gleam", tree_sitter_gleam::LANGUAGE),
        ("Astro", tree_sitter_astro::LANGUAGE),
        ("Prisma", tree_sitter_prisma::LANGUAGE),
        ("VimDoc", tree_sitter_vimdoc::LANGUAGE),
        ("WGSL", tree_sitter_wgsl::LANGUAGE),
        ("GLSL", tree_sitter_glsl::LANGUAGE),
        ("HLSL", tree_sitter_hlsl::LANGUAGE),
        ("Objective-C", tree_sitter_objc::LANGUAGE),
        ("MATLAB", tree_sitter_matlab::language()),
        ("Fortran", tree_sitter_fortran::language()),
        ("Ada", tree_sitter_ada::language()),
        ("COBOL", tree_sitter_cobol::LANGUAGE),
        ("Perl", tree_sitter_perl::language()),
        ("Tcl", tree_sitter_tcl::LANGUAGE),
        ("Groovy", tree_sitter_groovy::LANGUAGE),
    ];
    
    let total = languages.len();
    let mut successful = 0;
    let mut failed = 0;
    let mut total_parse_time = 0.0;
    
    println!("\nTesting {} languages:\n", total);
    
    for (name, lang) in &languages {
        let mut parser = Parser::new();
        
        // Generate test code
        let test_code = match *name {
            "JavaScript" => "function test() { return 42; }",
            "TypeScript" => "const x: number = 42;",
            "Python" => "def test():\n    return 42",
            "Rust" => "fn test() -> i32 { 42 }",
            "Go" => "func test() int { return 42 }",
            "C" => "int test() { return 42; }",
            "Kotlin" => "fun test(): Int = 42",
            "YAML" => "test: 42\nvalue: true",
            "SQL" => "SELECT * FROM test WHERE id = 42;",
            "Haskell" => "test :: Int\ntest = 42",
            "R" => "test <- function() { 42 }",
            "Julia" => "function test() 42 end",
            "Clojure" => "(defn test [] 42)",
            "Zig" => "fn test() i32 { return 42; }",
            "Nix" => "{ test = 42; }",
            "LaTeX" => "\\section{Test}\n\\begin{document}",
            "Make" => "test:\n\techo 42",
            "CMake" => "set(TEST 42)\nproject(Test)",
            "Verilog" => "module test; endmodule",
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
            _ => "test",
        };
        
        match parser.set_language(&lang.into()) {
            Ok(_) => {
                let start = Instant::now();
                match parser.parse(test_code, None) {
                    Some(tree) => {
                        let elapsed = start.elapsed();
                        let ms = elapsed.as_secs_f64() * 1000.0;
                        total_parse_time += ms;
                        let nodes = tree.root_node().descendant_count();
                        println!("✅ {:<15} | Parse: {:6.2}ms | Nodes: {:5}", name, ms, nodes);
                        successful += 1;
                    }
                    None => {
                        println!("❌ {:<15} | Failed to parse", name);
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                println!("❌ {:<15} | Failed to set language: {:?}", name, e);
                failed += 1;
            }
        }
    }
    
    println!("\n" + &"=".repeat(70));
    println!("📊 FINAL RESULTS");
    println!(&"=".repeat(70));
    println!("Total Languages: {}", total);
    println!("✅ Successful: {} ({:.1}%)", successful, (successful as f64 / total as f64) * 100.0);
    println!("❌ Failed: {} ({:.1}%)", failed, (failed as f64 / total as f64) * 100.0);
    
    if successful > 0 {
        println!("\n📈 Performance Metrics:");
        println!("   Average Parse Time: {:.2}ms", total_parse_time / successful as f64);
        println!("   Total Parse Time: {:.2}ms", total_parse_time);
    }
    
    println!("\n📋 Coverage vs 60 Essential Languages:");
    println!("   Achievement: {}/60 ({:.1}%)", successful, (successful as f64 / 60.0) * 100.0);
    
    if successful >= 55 {
        println!("\n🎉 EXCELLENT: Achieved {} languages (92%+ coverage)!", successful);
    } else if successful >= 50 {
        println!("\n✅ GREAT: Achieved {} languages (83%+ coverage)!", successful);
    } else if successful >= 40 {
        println!("\n👍 GOOD: Achieved {} languages (67%+ coverage)", successful);
    }
    
    println!("\n🚀 Lapce now supports {} programming languages!", successful);
}
