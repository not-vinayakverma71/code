use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser, Query, QueryCursor};

/// Test syntax highlighting for multiple languages using Tree-sitter
struct SyntaxHighlightTester {
    grammars_dir: PathBuf,
    queries_dir: PathBuf,
    parser: Parser,
}

impl SyntaxHighlightTester {
    fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/verma".to_string());
        let grammars_dir = PathBuf::from(format!("{}/.local/share/lapce-nightly/grammars", home));
        let queries_dir = PathBuf::from(format!("{}/.config/lapce-nightly/queries", home));

        let mut parser = Parser::new();
        // We'll set the language when testing each file

        Self {
            grammars_dir,
            queries_dir,
            parser,
        }
    }

    fn load_language(&self, name: &str) -> Option<Language> {
        let lib_path = self.grammars_dir.join(format!("{}.so", name));
        if !lib_path.exists() {
            return None;
        }

        unsafe {
            let library = libloading::Library::new(lib_path).ok()?;
            let language_fn: libloading::Symbol<unsafe extern "C" fn() -> Language> =
                library.get(format!("tree_sitter_{}", name).as_bytes()).ok()?;

            Some(language_fn())
        }
    }

    fn load_queries(&self, language: &str) -> Option<String> {
        let query_file = self.queries_dir.join(language).join("highlights.scm");
        fs::read_to_string(query_file).ok()
    }

    fn test_language(&mut self, language: &str, source_code: &str) -> TestResult {
        // Load the language grammar
        let lang = match self.load_language(language) {
            Some(lang) => lang,
            None => return TestResult::Failed(format!("Grammar not found for {}", language)),
        };

        // Set the language for the parser
        self.parser.set_language(&lang).unwrap();

        // Parse the source code
        let tree = match self.parser.parse(source_code, None) {
            Some(tree) => tree,
            None => return TestResult::Failed(format!("Failed to parse {}", language)),
        };

        // Load the highlighting queries
        let query_source = match self.load_queries(language) {
            Some(queries) => queries,
            None => return TestResult::Failed(format!("Queries not found for {}", language)),
        };

        // Create the query
        let query = match Query::new(&lang, &query_source) {
            Ok(query) => query,
            Err(e) => return TestResult::Failed(format!("Invalid query for {}: {:?}", language, e)),
        };

        // Run the query to check if it matches
        let mut query_cursor = QueryCursor::new();
        let matches = query_cursor.matches(&query, tree.root_node(), source_code.as_bytes());

        let match_count = matches.count();

        if match_count > 0 {
            TestResult::Success(format!("{}: Found {} highlight matches", language, match_count))
        } else {
            TestResult::Warning(format!("{}: Parsed successfully but no highlights found", language))
        }
    }

    fn run_comprehensive_test(&mut self) {
        let test_cases = vec![
            ("rust", r#"
fn main() {
    let message = "Hello, world!";
    println!("{}", message);
}
"#),
            ("python", r#"
def greet(name):
    print(f"Hello, {name}!")

if __name__ == "__main__":
    greet("World")
"#),
            ("javascript", r#"
function greet(name) {
    console.log(`Hello, ${name}!`);
}

greet("World");
"#),
            ("typescript", r#"
interface Person {
    name: string;
    age: number;
}

function greet(person: Person): string {
    return `Hello, ${person.name}!`;
}
"#),
            ("go", r#"
package main

import "fmt"

func main() {
    fmt.Println("Hello, World!")
}
"#),
            ("cpp", r#"
#include <iostream>

int main() {
    std::cout << "Hello, World!" << std::endl;
    return 0;
}
"#),
            ("java", r#"
public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#),
            ("html", r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Hello World</title>
</head>
<body>
    <h1>Hello, World!</h1>
</body>
</html>
"#),
            ("css", r#"
body {
    font-family: Arial, sans-serif;
    background-color: #f0f0f0;
}

h1 {
    color: #333;
}
"#),
            ("json", r#"
{
  "name": "test",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}
"#),
            ("yaml", r#"
name: test
version: 1.0.0
dependencies:
  lodash: ^4.17.21
"#),
            ("toml", r#"
[package]
name = "test"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#),
            ("bash", r#"
#!/bin/bash

echo "Hello, World!"

if [ "$1" = "test" ]; then
    echo "Running tests..."
fi
"#),
            ("sql", r#"
SELECT id, name, email
FROM users
WHERE active = 1
ORDER BY name;
"#),
        ];

        println!("🔍 Running comprehensive syntax highlighting test...\n");

        let mut results = Vec::new();

        for (language, code) in test_cases {
            println!("Testing {}...", language);
            let result = self.test_language(language, code);
            match &result {
                TestResult::Success(msg) => println!("✅ {}", msg),
                TestResult::Warning(msg) => println!("⚠️  {}", msg),
                TestResult::Failed(msg) => println!("❌ {}", msg),
            }
            results.push((language, result));
            println!();
        }

        self.print_summary(&results);
    }

    fn print_summary(&self, results: &[(&str, TestResult)]) {
        let total = results.len();
        let successful = results.iter().filter(|(_, r)| matches!(r, TestResult::Success(_))).count();
        let warnings = results.iter().filter(|(_, r)| matches!(r, TestResult::Warning(_))).count();
        let failed = results.iter().filter(|(_, r)| matches!(r, TestResult::Failed(_))).count();

        println!("📊 Test Summary:");
        println!("Total languages tested: {}", total);
        println!("✅ Successful: {}", successful);
        println!("⚠️  Warnings: {}", warnings);
        println!("❌ Failed: {}", failed);

        if failed == 0 {
            println!("\n🎉 All syntax highlighting tests passed!");
            println!("Tree-sitter grammars and queries are working correctly.");
        } else {
            println!("\n⚠️  Some tests failed. Check that:");
            println!("  - Grammar libraries (.so files) are present in ~/.local/share/lapce-nightly/grammars/");
            println!("  - Query files (.scm files) are present in ~/.config/lapce-nightly/queries/");
            println!("  - The grammar names match the expected format");
        }
    }
}

#[derive(Debug)]
enum TestResult {
    Success(String),
    Warning(String),
    Failed(String),
}

fn main() {
    let mut tester = SyntaxHighlightTester::new();
    tester.run_comprehensive_test();
}
