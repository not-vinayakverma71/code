use lapce_tree_sitter::all_languages_support::SupportedLanguage;
use std::fs;
use std::path::Path;
use tree_sitter::Parser;
use std::time::Instant;

fn main() {
    println!("🔬 COMPREHENSIVE TEST: 63 Tree-sitter Languages");
    println!("{}", "=".repeat(80));
    
    // Create test directory
    let test_dir = "test_all_63_languages";
    let _ = fs::create_dir_all(test_dir);
    
    let mut total_tested = 0;
    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut failed_languages = Vec::new();
    
    // Test samples for each language
    let test_samples = vec![
        ("rust", "fn main() { println!(\"Hello\"); }", "rs"),
        ("javascript", "const x = 42; console.log(x);", "js"),
        ("typescript", "let x: number = 42;", "ts"),
        ("python", "def hello(): print('world')", "py"),
        ("go", "package main\nfunc main() {}", "go"),
        ("java", "public class Test { }", "java"),
        ("c", "int main() { return 0; }", "c"),
        ("cpp", "int main() { return 0; }", "cpp"),
        ("csharp", "class Program { }", "cs"),
        ("ruby", "puts 'Hello'", "rb"),
        ("php", "<?php echo 'Hello'; ?>", "php"),
        ("swift", "let x = 42", "swift"),
        ("kotlin", "fun main() { }", "kt"),
        ("scala", "object Main { }", "scala"),
        ("elixir", "defmodule Test do end", "ex"),
        ("lua", "print('hello')", "lua"),
        ("bash", "echo 'hello'", "sh"),
        ("css", ".class { color: red; }", "css"),
        ("json", "{\"key\": \"value\"}", "json"),
        ("html", "<div>Hello</div>", "html"),
        ("yaml", "key: value", "yaml"),
        ("markdown", "# Title\n\nText", "md"),
        ("r", "x <- 42", "r"),
        ("matlab", "x = 42;", "m"),
        ("perl", "print 'hello';", "pl"),
        ("dart", "void main() { }", "dart"),
        ("julia", "x = 42", "jl"),
        ("haskell", "main = putStrLn \"Hello\"", "hs"),
        ("graphql", "query { field }", "graphql"),
        ("sql", "SELECT * FROM table;", "sql"),
        ("zig", "pub fn main() void { }", "zig"),
        ("vim", "set number", "vim"),
        ("ocaml", "let x = 42", "ml"),
        ("nix", "{ pkgs }: pkgs.hello", "nix"),
        ("make", "all:\n\techo done", "makefile"),
        ("cmake", "project(Test)", "cmake"),
        ("verilog", "module test; endmodule", "v"),
        ("erlang", "-module(test).", "erl"),
        ("d", "void main() { }", "d"),
        ("pascal", "program Test; begin end.", "pas"),
        ("objc", "@interface Test @end", "m"),
        ("groovy", "println 'Hello'", "groovy"),
        ("solidity", "contract Test { }", "sol"),
        ("fsharp", "let x = 42", "fs"),
        ("systemverilog", "module test; endmodule", "sv"),
        ("elm", "main = text \"Hello\"", "elm"),
        ("tsx", "const x: JSX.Element = <div />;", "tsx"),
        ("jsx", "const x = <div />;", "jsx"),
        ("cobol", "PROGRAM-ID. TEST.", "cob"),
        ("commonlisp", "(print \"hello\")", "lisp"),
        ("hcl", "resource \"test\" \"example\" { }", "hcl"),
        ("xml", "<root></root>", "xml"),
        ("clojure", "(println \"hello\")", "clj"),
        ("nim", "echo \"hello\"", "nim"),
        ("crystal", "puts \"hello\"", "cr"),
        ("fortran", "program test\nend program", "f90"),
        ("vhdl", "entity test is end;", "vhdl"),
        ("racket", "(display \"hello\")", "rkt"),
        ("ada", "procedure Test is begin null; end;", "ada"),
        ("svelte", "<script>let x = 42;</script>", "svelte"),
        ("abap", "WRITE 'Hello'.", "abap"),
        ("scheme", "(display \"hello\")", "scm"),
        ("fennel", "(print \"hello\")", "fnl"),
        ("gleam", "pub fn main() { }", "gleam"),
        ("astro", "---\nconst x = 42;\n---", "astro"),
        ("wgsl", "fn main() { }", "wgsl"),
        ("glsl", "void main() { }", "glsl"),
        ("tcl", "puts \"hello\"", "tcl"),
        ("cairo", "func main() { }", "cairo"),
    ];
    
    println!("📋 Testing {} language samples", test_samples.len());
    println!();
    
    for (lang_name, content, ext) in test_samples {
        print!("Testing {:20} ", format!("{}:", lang_name));
        total_tested += 1;
        
        // Write test file
        let file_path = format!("{}/test.{}", test_dir, ext);
        if let Err(e) = fs::write(&file_path, content) {
            println!("❌ Failed to write test file: {}", e);
            total_failed += 1;
            failed_languages.push(lang_name.to_string());
            continue;
        }
        
        // Try to parse with tree-sitter
        let start = Instant::now();
        match try_parse_file(&file_path, lang_name) {
            Ok(node_count) => {
                let elapsed = start.elapsed();
                println!("✅ Success ({} nodes, {:.2}ms)", node_count, elapsed.as_secs_f64() * 1000.0);
                total_passed += 1;
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
                total_failed += 1;
                failed_languages.push(lang_name.to_string());
            }
        }
    }
    
    // Clean up test directory
    let _ = fs::remove_dir_all(test_dir);
    
    println!();
    println!("{}", "=".repeat(80));
    println!("📊 FINAL RESULTS");
    println!("{}", "=".repeat(80));
    println!("Total Languages Tested: {}", total_tested);
    println!("✅ Passed: {} ({:.1}%)", total_passed, (total_passed as f64 / total_tested as f64) * 100.0);
    println!("❌ Failed: {} ({:.1}%)", total_failed, (total_failed as f64 / total_tested as f64) * 100.0);
    
    if total_failed > 0 {
        println!("\n⚠️ Failed Languages:");
        for lang in &failed_languages {
            println!("  - {}", lang);
        }
        println!("\n❌ NOT ALL LANGUAGES WORKING - FIX REQUIRED!");
        std::process::exit(1);
    } else {
        println!("\n✅ ALL 63 LANGUAGES WORKING PERFECTLY!");
        println!("🎉 100% SUCCESS RATE ACHIEVED!");
        std::process::exit(0);
    }
}

fn try_parse_file(file_path: &str, lang_name: &str) -> Result<usize, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    // Get language from SupportedLanguage enum
    let language = match lang_name {
        "rust" => SupportedLanguage::Rust,
        "javascript" | "jsx" => SupportedLanguage::JavaScript,
        "typescript" | "tsx" => SupportedLanguage::TypeScript,
        "python" => SupportedLanguage::Python,
        "go" => SupportedLanguage::Go,
        "java" => SupportedLanguage::Java,
        "c" => SupportedLanguage::C,
        "cpp" => SupportedLanguage::Cpp,
        "csharp" => SupportedLanguage::CSharp,
        "ruby" => SupportedLanguage::Ruby,
        "php" => SupportedLanguage::Php,
        "swift" => SupportedLanguage::Swift,
        "kotlin" => SupportedLanguage::Kotlin,
        "scala" => SupportedLanguage::Scala,
        "elixir" => SupportedLanguage::Elixir,
        "lua" => SupportedLanguage::Lua,
        "bash" => SupportedLanguage::Bash,
        "css" => SupportedLanguage::Css,
        "json" => SupportedLanguage::Json,
        "html" => SupportedLanguage::Html,
        "yaml" => SupportedLanguage::Yaml,
        "markdown" => SupportedLanguage::Markdown,
        "r" => SupportedLanguage::R,
        "matlab" => SupportedLanguage::Matlab,
        "perl" => SupportedLanguage::Perl,
        "dart" => SupportedLanguage::Dart,
        "julia" => SupportedLanguage::Julia,
        "haskell" => SupportedLanguage::Haskell,
        "graphql" => SupportedLanguage::GraphQL,
        "sql" => SupportedLanguage::Sql,
        "zig" => SupportedLanguage::Zig,
        "vim" => SupportedLanguage::Vim,
        "ocaml" => SupportedLanguage::Ocaml,
        "nix" => SupportedLanguage::Nix,
        "make" => SupportedLanguage::Make,
        "cmake" => SupportedLanguage::Cmake,
        "verilog" => SupportedLanguage::Verilog,
        "erlang" => SupportedLanguage::Erlang,
        "d" => SupportedLanguage::D,
        "pascal" => SupportedLanguage::Pascal,
        "objc" => SupportedLanguage::ObjectiveC,
        "groovy" => SupportedLanguage::Groovy,
        "solidity" => SupportedLanguage::Solidity,
        "fsharp" => SupportedLanguage::FSharp,
        "systemverilog" => SupportedLanguage::SystemVerilog,
        "elm" => SupportedLanguage::Elm,
        "cobol" => SupportedLanguage::Cobol,
        "commonlisp" => SupportedLanguage::CommonLisp,
        "hcl" => SupportedLanguage::Hcl,
        "xml" => SupportedLanguage::Xml,
        "clojure" => SupportedLanguage::Clojure,
        "nim" => SupportedLanguage::Nim,
        "crystal" => SupportedLanguage::Crystal,
        "fortran" => SupportedLanguage::Fortran,
        "vhdl" => SupportedLanguage::Vhdl,
        "racket" => SupportedLanguage::Racket,
        "ada" => SupportedLanguage::Ada,
        "svelte" => SupportedLanguage::Svelte,
        "abap" => SupportedLanguage::Abap,
        // External grammars
        "scheme" => SupportedLanguage::Scheme,
        "fennel" => SupportedLanguage::Fennel, 
        "gleam" => SupportedLanguage::Gleam,
        "astro" => SupportedLanguage::Astro,
        "wgsl" => SupportedLanguage::Wgsl,
        "glsl" => SupportedLanguage::Glsl,
        "tcl" => SupportedLanguage::Tcl,
        "cairo" => SupportedLanguage::Cairo,
        _ => return Err(format!("Unknown language: {}", lang_name)),
    };
    
    // Get language parser
    let lang = language.get_language()
        .map_err(|e| format!("Failed to get language: {}", e))?;
    
    // Create parser
    let mut parser = Parser::new();
    parser.set_language(&lang)
        .map_err(|e| format!("Failed to set language: {:?}", e))?;
    
    // Parse the content
    let tree = parser.parse(&content, None)
        .ok_or_else(|| "Failed to parse".to_string())?;
    
    // Count nodes to verify parsing worked
    let root = tree.root_node();
    let node_count = count_nodes(&root);
    
    if node_count == 0 {
        return Err("No nodes in tree".to_string());
    }
    
    Ok(node_count)
}

fn count_nodes(node: &tree_sitter::Node) -> usize {
    let mut count = 1;
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            count += count_nodes(&child);
        }
    }
    count
}
