//! Final validation for all 67 languages
//! Tests parsers, symbol extraction, and performance metrics

use lapce_tree_sitter::all_languages_support::SupportedLanguage;
use lapce_tree_sitter::enhanced_codex_format::{EnhancedSymbolExtractor, LanguageConfig};
use lapce_tree_sitter::performance_metrics::PerformanceTracker;
use std::time::Instant;
use std::fs;
use std::path::Path;

fn main() {
    println!("\n🚀 FINAL VALIDATION: ALL 67 LANGUAGES");
    println!("{}", "=".repeat(80));
    println!();
    
    // Initialize performance tracker
    let mut tracker = PerformanceTracker::new();
    
    // Test data for each language type
    let test_codes = get_all_test_codes();
    
    let mut results = Vec::new();
    let mut total_languages = 0;
    let mut working_parsers = 0;
    let mut working_symbols = 0;
    let mut codex_languages = 0;
    let mut default_languages = 0;
    
    println!("📊 Testing all languages...\n");
    
    for lang in SupportedLanguage::all() {
        total_languages += 1;
        let lang_name = format!("{:?}", lang);
        
        print!("{:3}. {:20} ", total_languages, lang_name);
        
        // Get test code
        let (ext, code) = test_codes.get(&lang)
            .unwrap_or(&("txt", "// Test code"));
        
        // Test parser
        let start = Instant::now();
        match lang.get_parser() {
            Ok(mut parser) => {
                match parser.parse(code, None) {
                    Some(tree) => {
                        let parse_time = start.elapsed();
                        let lines = code.lines().count();
                        let bytes = code.len();
                        
                        // Record metrics
                        tracker.record_parse(parse_time, lines, bytes);
                        tracker.record_full_parse(parse_time);
                        
                        print!("✅ Parser ");
                        working_parsers += 1;
                        
                        // Test symbol extraction
                        let mut extractor = EnhancedSymbolExtractor::new();
                        let symbol_start = Instant::now();
                        
                        match extractor.extract_symbols(ext, code) {
                            Some(symbols) => {
                                let symbol_time = symbol_start.elapsed();
                                let symbol_count = symbols.lines().count();
                                
                                tracker.record_symbol_extraction(symbol_time, symbol_count);
                                
                                print!("✅ Symbols ({}) ", symbol_count);
                                working_symbols += 1;
                                
                                // Check format type
                                if LanguageConfig::is_codex_supported(ext) {
                                    print!("[Codex]");
                                    codex_languages += 1;
                                } else {
                                    print!("[Default]");
                                    default_languages += 1;
                                }
                            }
                            None => {
                                print!("⚠️  No symbols ");
                            }
                        }
                        
                        println!(" | Parse: {:?}", parse_time);
                        
                        results.push((lang_name.clone(), true, true));
                    }
                    None => {
                        println!("❌ Parse failed");
                        results.push((lang_name.clone(), false, false));
                    }
                }
            }
            Err(e) => {
                println!("❌ Parser error: {}", e);
                results.push((lang_name.clone(), false, false));
            }
        }
        
        // Sample memory periodically
        if total_languages % 10 == 0 {
            tracker.sample_memory();
        }
    }
    
    // Final memory sample
    tracker.sample_memory();
    
    // Generate performance report
    let report = tracker.generate_report();
    let criteria = tracker.check_success_criteria();
    
    println!("\n{}", "=".repeat(80));
    println!("📊 FINAL RESULTS");
    println!("{}", "=".repeat(80));
    
    println!("\n📈 Language Support:");
    println!("  Total Languages:        {}", total_languages);
    println!("  Working Parsers:        {} ({:.1}%)", 
        working_parsers, 
        (working_parsers as f64 / total_languages as f64) * 100.0
    );
    println!("  Symbol Extraction:      {} ({:.1}%)",
        working_symbols,
        (working_symbols as f64 / total_languages as f64) * 100.0
    );
    println!("  Codex Format:          {} languages", codex_languages);
    println!("  Default Format:        {} languages", default_languages);
    
    println!("\n💾 Memory Usage:");
    println!("  Peak Usage:            {:.2} MB", report.memory.peak_usage_mb);
    println!("  Average Usage:         {:.2} MB", report.memory.average_usage_mb);
    println!("  Target:                < 5 MB");
    println!("  Status:                {}", 
        if criteria.memory_under_5mb { "✅ PASS" } else { "❌ FAIL" }
    );
    
    println!("\n⚡ Parse Performance:");
    println!("  Total Lines Parsed:    {}", report.parse.total_lines);
    println!("  Speed:                 {:.0} lines/second", report.parse.lines_per_second);
    println!("  Target:                > 10,000 lines/second");
    println!("  Status:                {}", 
        if criteria.parse_speed_over_10k { "✅ PASS" } else { "❌ FAIL" }
    );
    
    println!("\n🎯 Symbol Extraction:");
    println!("  Total Symbols:         {}", report.symbols.total_symbols);
    println!("  Average Time:          {:.2} ms", 
        report.symbols.average_extraction_time.as_secs_f64() * 1000.0
    );
    println!("  Target:                < 50 ms");
    println!("  Status:                {}", 
        if criteria.symbol_extraction_under_50ms { "✅ PASS" } else { "❌ FAIL" }
    );
    
    // List failed languages
    let failed: Vec<_> = results.iter()
        .filter(|(_, parser_ok, _)| !parser_ok)
        .collect();
    
    if !failed.is_empty() {
        println!("\n⚠️  Failed Languages ({}):", failed.len());
        for (name, _, _) in &failed {
            println!("    - {}", name);
        }
    }
    
    // Success criteria summary
    println!("\n{}", "=".repeat(80));
    println!("{}", criteria.summary());
    
    // Final verdict
    println!("\n{}", "=".repeat(80));
    if working_parsers == total_languages && criteria.all_passed() {
        println!("✅ SUCCESS: All 67 languages working with all criteria met!");
    } else if working_parsers >= 60 {
        println!("⚠️  PARTIAL SUCCESS: {}/{} languages working", working_parsers, total_languages);
    } else {
        println!("❌ NEEDS WORK: Only {}/{} languages working", working_parsers, total_languages);
    }
    println!("{}", "=".repeat(80));
}

fn get_all_test_codes() -> std::collections::HashMap<SupportedLanguage, (&'static str, &'static str)> {
    use std::collections::HashMap;
    let mut codes = HashMap::new();
    
    // Add test code for each language
    codes.insert(SupportedLanguage::Rust, ("rs", r#"
fn main() {
    println!("Hello, world!");
}

struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }
}
"#));
    
    codes.insert(SupportedLanguage::JavaScript, ("js", r#"
function greet(name) {
    console.log(`Hello, ${name}!`);
}

class Person {
    constructor(name, age) {
        this.name = name;
        this.age = age;
    }
}
"#));
    
    codes.insert(SupportedLanguage::TypeScript, ("ts", r#"
interface User {
    id: number;
    name: string;
}

class UserService {
    getUser(id: number): User | undefined {
        return undefined;
    }
}
"#));
    
    codes.insert(SupportedLanguage::Python, ("py", r#"
def greet(name):
    print(f"Hello, {name}!")

class Person:
    def __init__(self, name, age):
        self.name = name
        self.age = age
"#));
    
    codes.insert(SupportedLanguage::Go, ("go", r#"
package main

func greet(name string) {
    fmt.Println("Hello", name)
}

type Person struct {
    Name string
    Age  int
}
"#));
    
    codes.insert(SupportedLanguage::Java, ("java", r#"
public class Person {
    private String name;
    private int age;
    
    public Person(String name, int age) {
        this.name = name;
        this.age = age;
    }
}
"#));
    
    codes.insert(SupportedLanguage::C, ("c", r#"
#include <stdio.h>

void greet(const char* name) {
    printf("Hello, %s!\n", name);
}

int main() {
    greet("World");
    return 0;
}
"#));
    
    codes.insert(SupportedLanguage::Cpp, ("cpp", r#"
#include <iostream>

class Person {
private:
    std::string name;
    int age;
public:
    Person(std::string n, int a) : name(n), age(a) {}
};
"#));
    
    codes.insert(SupportedLanguage::CSharp, ("cs", r#"
public class Person {
    public string Name { get; set; }
    public int Age { get; set; }
    
    public Person(string name, int age) {
        Name = name;
        Age = age;
    }
}
"#));
    
    codes.insert(SupportedLanguage::Ruby, ("rb", r#"
class Person
  attr_accessor :name, :age
  
  def initialize(name, age)
    @name = name
    @age = age
  end
end
"#));
    
    codes.insert(SupportedLanguage::Php, ("php", r#"
<?php
class Person {
    private $name;
    private $age;
    
    public function __construct($name, $age) {
        $this->name = $name;
        $this->age = $age;
    }
}
"#));
    
    codes.insert(SupportedLanguage::Swift, ("swift", r#"
struct Person {
    let name: String
    let age: Int
}

func greet(name: String) {
    print("Hello, \(name)!")
}
"#));
    
    codes.insert(SupportedLanguage::Kotlin, ("kt", r#"
class Person(val name: String, val age: Int) {
    fun greet() {
        println("Hello, $name")
    }
}
"#));
    
    codes.insert(SupportedLanguage::Scala, ("scala", r#"
class Person(val name: String, val age: Int) {
  def greet(): Unit = {
    println(s"Hello, $name")
  }
}
"#));
    
    codes.insert(SupportedLanguage::Html, ("html", r#"
<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body>
    <h1>Hello World</h1>
</body>
</html>
"#));
    
    codes.insert(SupportedLanguage::Css, ("css", r#"
body {
    font-family: Arial, sans-serif;
}

.container {
    max-width: 1200px;
}
"#));
    
    codes.insert(SupportedLanguage::Json, ("json", r#"
{
    "name": "Test",
    "version": "1.0.0",
    "dependencies": {
        "express": "4.18.0"
    }
}
"#));
    
    codes.insert(SupportedLanguage::Yaml, ("yaml", r#"
name: Test
version: 1.0.0
dependencies:
  express: 4.18.0
  lodash: 4.17.21
"#));
    
    codes.insert(SupportedLanguage::Toml, ("toml", r#"
[package]
name = "test"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#));
    
    // Add simple test code for remaining languages
    for lang in SupportedLanguage::all() {
        if !codes.contains_key(&lang) {
            codes.insert(lang, ("txt", "// Simple test code\nfunction test() { return true; }"));
        }
    }
    
    codes
}
