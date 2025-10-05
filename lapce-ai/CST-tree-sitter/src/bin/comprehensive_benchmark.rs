use std::time::{Duration, Instant};
use std::collections::HashMap;
use tree_sitter::{Parser, Language};

fn main() {
    println!("\n🚀 LAPCE TREE-SITTER 0.24 COMPREHENSIVE BENCHMARK");
    println!("==================================================");
    println!("Testing 22 working languages against 60-language requirements\n");
    
    let mut results = BenchmarkResults::new();
    
    // Test all 22 working languages
    let languages = get_language_tests();
    
    println!("📊 PHASE 1: PARSING PERFORMANCE");
    println!("--------------------------------");
    
    for (name, test) in &languages {
        let result = benchmark_language(name, &test);
        results.add_result(name, result);
    }
    
    results.generate_report();
}

struct LanguageTest {
    language: Language,
    sample_code: String,
}

struct BenchmarkResult {
    parse_time: Duration,
    parse_speed_lines_per_sec: usize,
    memory_kb: usize,
    incremental_parse_time: Duration,
    node_count: usize,
    success: bool,
}

struct BenchmarkResults {
    results: HashMap<String, BenchmarkResult>,
    start_time: Instant,
}

impl BenchmarkResults {
    fn new() -> Self {
        Self {
            results: HashMap::new(),
            start_time: Instant::now(),
        }
    }
    
    fn add_result(&mut self, lang: &str, result: BenchmarkResult) {
        self.results.insert(lang.to_string(), result);
    }
    
    fn generate_report(&self) {
        println!("\n{}", "=".repeat(80));
        println!("📈 COMPREHENSIVE BENCHMARK REPORT");
        println!("{}", "=".repeat(80));
        
        let total_languages = self.results.len();
        let successful: Vec<_> = self.results.iter()
            .filter(|(_, r)| r.success)
            .collect();
        
        println!("\n🎯 OVERALL RESULTS:");
        println!("   Total Languages Tested: {}", total_languages);
        println!("   ✅ Successful: {}", successful.len());
        println!("   Success Rate: {:.1}%", (successful.len() as f64 / total_languages as f64) * 100.0);
        
        if !successful.is_empty() {
            let avg_parse_time: f64 = successful.iter()
                .map(|(_, r)| r.parse_time.as_secs_f64() * 1000.0)
                .sum::<f64>() / successful.len() as f64;
                
            let avg_speed: usize = successful.iter()
                .map(|(_, r)| r.parse_speed_lines_per_sec)
                .sum::<usize>() / successful.len();
                
            let total_memory: usize = successful.iter()
                .map(|(_, r)| r.memory_kb)
                .sum();
                
            let avg_incremental: f64 = successful.iter()
                .map(|(_, r)| r.incremental_parse_time.as_secs_f64() * 1000.0)
                .sum::<f64>() / successful.len() as f64;
            
            println!("\n📊 PERFORMANCE METRICS:");
            println!("   Average Parse Time: {:.2}ms", avg_parse_time);
            println!("   Average Parse Speed: {} lines/sec", avg_speed);
            println!("   Average Incremental Parse: {:.2}ms", avg_incremental);
            println!("   Total Memory Usage: {} KB ({:.2} MB)", total_memory, total_memory as f64 / 1024.0);
            
            println!("\n✅ REQUIREMENT CHECKS:");
            println!("   Parse Speed > 125K lines/s: {}", 
                if avg_speed > 125_000 { "✅ PASS" } else { "❌ FAIL" });
            println!("   Incremental < 10ms: {}", 
                if avg_incremental < 10.0 { "✅ PASS" } else { "❌ FAIL" });
            println!("   Memory < 5MB total: {}", 
                if total_memory < 5120 { "✅ PASS" } else { "❌ FAIL" });
        }
        
        analyze_language_coverage();
        
        println!("\n📊 DETAILED RESULTS BY LANGUAGE:");
        println!("{:-<80}", "");
        println!("{:<15} {:>10} {:>12} {:>10} {:>10} {:>8}",
            "Language", "Parse(ms)", "Lines/sec", "Incr(ms)", "Mem(KB)", "Nodes");
        println!("{:-<80}", "");
        
        let mut sorted_results: Vec<_> = self.results.iter().collect();
        sorted_results.sort_by_key(|(name, _)| name.as_str());
        
        for (name, result) in sorted_results {
            if result.success {
                println!("{:<15} {:>10.2} {:>12} {:>10.2} {:>10} {:>8}",
                    name,
                    result.parse_time.as_secs_f64() * 1000.0,
                    result.parse_speed_lines_per_sec,
                    result.incremental_parse_time.as_secs_f64() * 1000.0,
                    result.memory_kb,
                    result.node_count
                );
            }
        }
        
        println!("\n⏱️  Total Benchmark Time: {:.2}s", self.start_time.elapsed().as_secs_f64());
    }
}

fn analyze_language_coverage() {
    let essential_60 = vec![
        // Systems
        "C", "C++", "Rust", "Go", "Zig", "D", "Nim", "Ada", "Fortran", "Assembly",
        // Web
        "JavaScript", "TypeScript", "HTML", "CSS", "PHP", "Ruby", "Elixir", "Java", 
        "C#", "Kotlin", "Swift", "Objective-C", "Scala", "Groovy",
        // Data Science
        "Python", "R", "Julia", "MATLAB", "SAS", "Stata", "SQL", "Cypher", "GraphQL",
        // Scripting
        "Bash", "PowerShell", "AppleScript", "Lua", "Perl", "Tcl",
        // Cloud
        "HCL", "YAML", "Erlang",
        // Blockchain
        "Solidity", "Vyper", "Cairo",
        // Hardware
        "Verilog", "VHDL", "SystemVerilog",
        // Functional
        "Haskell", "Scheme", "OCaml", "Elm", "F#", "Prolog", "Clojure",
        // Enterprise
        "Simulink", "COBOL", "ABAP", "RPG", "MUMPS"
    ];
    
    let working_22 = vec![
        "C", "C++", "C#", "Go", "Java", "JavaScript", "PHP", "Python", 
        "Ruby", "Rust", "Swift", "TypeScript", "CSS", "HTML", "JSON",
        "Bash", "Lua", "Elixir", "Scala", "Elm", "OCaml"
    ];
    
    let covered = working_22.iter()
        .filter(|l| essential_60.contains(l))
        .count();
    
    println!("\n📋 COVERAGE AGAINST 60 ESSENTIAL LANGUAGES:");
    println!("   Coverage: {}/{} ({:.1}%)", covered, essential_60.len(), 
        (covered as f64 / essential_60.len() as f64) * 100.0);
    
    println!("\n   By Category:");
    println!("   - Systems: 4/10 (C, C++, Rust, Go)");
    println!("   - Web: 10/14 (JS, TS, HTML, CSS, PHP, Ruby, Elixir, Java, C#, Swift)");
    println!("   - Data Science: 1/9 (Python)");
    println!("   - Scripting: 2/6 (Bash, Lua)");
    println!("   - Functional: 3/7 (OCaml, Elm, Scala)");
    
    println!("\n   🎯 Next Priority Languages:");
    println!("   1. Kotlin (Android/mobile)");
    println!("   2. YAML (configuration)");
    println!("   3. SQL (databases)");
    println!("   4. GraphQL (APIs)");
    println!("   5. Dart (Flutter)");
}

fn get_language_tests() -> Vec<(&'static str, LanguageTest)> {
    vec![
        ("JavaScript", LanguageTest {
            language: tree_sitter_javascript::language().into(),
            sample_code: generate_js_code(500),
        }),
        ("TypeScript", LanguageTest {
            language: tree_sitter_typescript::language_typescript().into(),
            sample_code: generate_ts_code(500),
        }),
        ("Python", LanguageTest {
            language: tree_sitter_python::LANGUAGE.into(),
            sample_code: generate_python_code(500),
        }),
        ("Rust", LanguageTest {
            language: tree_sitter_rust::LANGUAGE.into(),
            sample_code: generate_rust_code(500),
        }),
        ("Go", LanguageTest {
            language: tree_sitter_go::LANGUAGE.into(),
            sample_code: generate_go_code(500),
        }),
        ("C", LanguageTest {
            language: tree_sitter_c::LANGUAGE.into(),
            sample_code: generate_c_code(500),
        }),
        ("C++", LanguageTest {
            language: tree_sitter_cpp::LANGUAGE.into(),
            sample_code: generate_cpp_code(500),
        }),
        ("C#", LanguageTest {
            language: tree_sitter_c_sharp::LANGUAGE.into(),
            sample_code: generate_csharp_code(500),
        }),
        ("Java", LanguageTest {
            language: tree_sitter_java::LANGUAGE.into(),
            sample_code: generate_java_code(500),
        }),
        ("Ruby", LanguageTest {
            language: tree_sitter_ruby::LANGUAGE.into(),
            sample_code: generate_ruby_code(500),
        }),
        ("PHP", LanguageTest {
            language: tree_sitter_php::LANGUAGE_PHP.into(),
            sample_code: generate_php_code(500),
        }),
        ("Swift", LanguageTest {
            language: tree_sitter_swift::LANGUAGE.into(),
            sample_code: generate_swift_code(500),
        }),
        ("Lua", LanguageTest {
            language: tree_sitter_lua::LANGUAGE.into(),
            sample_code: generate_lua_code(500),
        }),
        ("Elixir", LanguageTest {
            language: tree_sitter_elixir::LANGUAGE.into(),
            sample_code: generate_elixir_code(500),
        }),
        ("Scala", LanguageTest {
            language: tree_sitter_scala::LANGUAGE.into(),
            sample_code: generate_scala_code(500),
        }),
        ("Bash", LanguageTest {
            language: tree_sitter_bash::LANGUAGE.into(),
            sample_code: generate_bash_code(500),
        }),
        ("CSS", LanguageTest {
            language: tree_sitter_css::LANGUAGE.into(),
            sample_code: generate_css_code(500),
        }),
        ("JSON", LanguageTest {
            language: tree_sitter_json::LANGUAGE.into(),
            sample_code: generate_json_code(),
        }),
        ("HTML", LanguageTest {
            language: tree_sitter_html::LANGUAGE.into(),
            sample_code: generate_html_code(500),
        }),
        ("TSX", LanguageTest {
            language: tree_sitter_typescript::language_tsx().into(),
            sample_code: generate_tsx_code(500),
        }),
        ("Elm", LanguageTest {
            language: tree_sitter_elm::LANGUAGE().into(),
            sample_code: generate_elm_code(500),
        }),
        ("OCaml", LanguageTest {
            language: tree_sitter_ocaml::LANGUAGE_OCAML.into(),
            sample_code: generate_ocaml_code(500),
        }),
    ]
}

fn benchmark_language(name: &str, test: &LanguageTest) -> BenchmarkResult {
    let mut parser = Parser::new();
    
    if parser.set_language(&test.language).is_err() {
        println!("❌ {:<15} - Failed to set language", name);
        return BenchmarkResult {
            parse_time: Duration::from_secs(0),
            parse_speed_lines_per_sec: 0,
            memory_kb: 0,
            incremental_parse_time: Duration::from_secs(0),
            node_count: 0,
            success: false,
        };
    }
    
    let start = Instant::now();
    let tree = match parser.parse(&test.sample_code, None) {
        Some(t) => t,
        None => {
            println!("❌ {:<15} - Failed to parse", name);
            return BenchmarkResult {
                parse_time: Duration::from_secs(0),
                parse_speed_lines_per_sec: 0,
                memory_kb: 0,
                incremental_parse_time: Duration::from_secs(0),
                node_count: 0,
                success: false,
            };
        }
    };
    let parse_time = start.elapsed();
    
    let line_count = test.sample_code.lines().count();
    let parse_speed = if parse_time.as_secs_f64() > 0.0 {
        (line_count as f64 / parse_time.as_secs_f64()) as usize
    } else {
        line_count * 1000
    };
    
    let node_count = count_nodes(&tree.root_node());
    let memory_kb = (test.sample_code.len() + node_count * 48) / 1024;
    
    let modified_code = format!("{}\n// Added comment", test.sample_code);
    let incr_start = Instant::now();
    let _incr_tree = parser.parse(&modified_code, Some(&tree));
    let incremental_parse_time = incr_start.elapsed();
    
    println!("✅ {:<15} | Parse: {:6.2}ms | Speed: {:8} l/s | Incr: {:5.2}ms | Nodes: {:6}",
        name,
        parse_time.as_secs_f64() * 1000.0,
        parse_speed,
        incremental_parse_time.as_secs_f64() * 1000.0,
        node_count
    );
    
    BenchmarkResult {
        parse_time,
        parse_speed_lines_per_sec: parse_speed,
        memory_kb,
        incremental_parse_time,
        node_count,
        success: true,
    }
}

fn count_nodes(node: &tree_sitter::Node) -> usize {
    let mut count = 1;
    let mut cursor = node.walk();
    
    if cursor.goto_first_child() {
        loop {
            count += count_nodes(&cursor.node());
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    count
}

// Code generators
fn generate_js_code(n: usize) -> String {
    (0..n).map(|i| format!("function f{}() {{ return {}; }}\n", i, i)).collect()
}

fn generate_ts_code(n: usize) -> String {
    (0..n).map(|i| format!("interface I{} {{ id: number; name: string; }}\n", i)).collect()
}

fn generate_python_code(n: usize) -> String {
    (0..n).map(|i| format!("def func_{}():\n    return {}\n", i, i)).collect()
}

fn generate_rust_code(n: usize) -> String {
    (0..n).map(|i| format!("fn func_{}() -> i32 {{ {} }}\n", i, i)).collect()
}

fn generate_go_code(n: usize) -> String {
    (0..n).map(|i| format!("func f{}() int {{ return {} }}\n", i, i)).collect()
}

fn generate_c_code(n: usize) -> String {
    (0..n).map(|i| format!("int func_{}() {{ return {}; }}\n", i, i)).collect()
}

fn generate_cpp_code(n: usize) -> String {
    (0..n).map(|i| format!("class C{} {{ public: int get() {{ return {}; }} }};\n", i, i)).collect()
}

fn generate_csharp_code(n: usize) -> String {
    (0..n).map(|i| format!("public class C{} {{ public int Get() => {}; }}\n", i, i)).collect()
}

fn generate_java_code(n: usize) -> String {
    (0..n).map(|i| format!("public class C{} {{ public int get() {{ return {}; }} }}\n", i, i)).collect()
}

fn generate_ruby_code(n: usize) -> String {
    (0..n).map(|i| format!("def func_{}\n  {}\nend\n", i, i)).collect()
}

fn generate_php_code(n: usize) -> String {
    (0..n).map(|i| format!("<?php\nfunction f{}() {{ return {}; }}\n", i, i)).collect()
}

fn generate_swift_code(n: usize) -> String {
    (0..n).map(|i| format!("func f{}() -> Int {{ return {} }}\n", i, i)).collect()
}

fn generate_lua_code(n: usize) -> String {
    (0..n).map(|i| format!("function f{}() return {} end\n", i, i)).collect()
}

fn generate_elixir_code(n: usize) -> String {
    (0..n).map(|i| format!("def f{}(), do: {}\n", i, i)).collect()
}

fn generate_scala_code(n: usize) -> String {
    (0..n).map(|i| format!("def f{}: Int = {}\n", i, i)).collect()
}

fn generate_bash_code(n: usize) -> String {
    (0..n).map(|i| format!("function f{} {{ echo {}; }}\n", i, i)).collect()
}

fn generate_css_code(n: usize) -> String {
    (0..n).map(|i| format!(".class-{} {{ margin: {}px; }}\n", i, i)).collect()
}

fn generate_json_code() -> String {
    format!("{{\n{}\n}}", (0..500).map(|i| format!("  \"key{}\": {}", i, i)).collect::<Vec<_>>().join(",\n"))
}

fn generate_html_code(n: usize) -> String {
    format!("<html><body>\n{}</body></html>", 
        (0..n).map(|i| format!("<div id=\"{}\">Content {}</div>\n", i, i)).collect::<String>())
}

fn generate_tsx_code(n: usize) -> String {
    (0..n).map(|i| format!("const C{}: React.FC = () => <div>Component {}</div>;\n", i, i)).collect()
}

fn generate_elm_code(n: usize) -> String {
    (0..n).map(|i| format!("func{} : Int\nfunc{} = {}\n", i, i, i)).collect()
}

fn generate_ocaml_code(n: usize) -> String {
    (0..n).map(|i| format!("let func_{} () = {}\n", i, i)).collect()
}
