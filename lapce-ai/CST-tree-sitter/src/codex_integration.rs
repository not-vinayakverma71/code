//! MAIN INTEGRATION MODULE - Primary API for Codex-compatible symbol extraction
//! This is the module that external systems should use

use crate::codex_exact_format;
use crate::directory_traversal;
use std::path::Path;

/// Main API for extracting symbols from source code
/// Returns symbols in EXACT Codex format: "startLine--endLine | definition_text"
pub struct CodexSymbolExtractor {
    respect_gitignore: bool,
    min_component_lines: usize,
}

impl CodexSymbolExtractor {
    /// Create new extractor with default settings
    pub fn new() -> Self {
        Self {
            respect_gitignore: true,
            min_component_lines: 4, // Codex default
        }
    }
    
    /// Create extractor with custom settings
    pub fn with_settings(respect_gitignore: bool, min_component_lines: usize) -> Self {
        Self {
            respect_gitignore,
            min_component_lines,
        }
    }
    
    /// Extract symbols from a single file
    /// Returns: String in format "# filename.ext\n1--10 | function myFunc()\n..."
    pub fn extract_from_file(&self, file_path: &str, source_code: &str) -> Option<String> {
        codex_exact_format::parse_source_code_definitions_for_file(file_path, source_code)
    }
    
    /// Extract symbols from a file by reading it from disk
    pub fn extract_from_file_path(&self, file_path: &str) -> Option<String> {
        let path = Path::new(file_path);
        if !path.exists() || !path.is_file() {
            return None;
        }
        
        match std::fs::read_to_string(path) {
            Ok(content) => self.extract_from_file(file_path, &content),
            Err(_) => None,
        }
    }
    
    /// Extract symbols from all files in a directory (max 50 files)
    /// Respects .gitignore if enabled
    pub fn extract_from_directory(&self, dir_path: &str) -> String {
        directory_traversal::parse_directory_for_definitions(dir_path, self.respect_gitignore)
    }
    
    /// Check if a file extension is supported
    pub fn is_supported_file(&self, file_path: &str) -> bool {
        let path = Path::new(file_path);
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            Self::is_supported_extension(ext_str)
        } else {
            false
        }
    }
    
    /// Check if an extension is supported
    pub fn is_supported_extension(ext: &str) -> bool {
        matches!(ext,
            "js" | "jsx" | "ts" | "tsx" | "vue" |
            "py" | "rs" | "go" | "c" | "h" |
            "cpp" | "hpp" | "cs" | "rb" | "java" |
            "php" | "swift" | "sol" | "kt" | "kts" |
            "ex" | "exs" | "el" | "html" | "htm" |
            "md" | "markdown" | "json" | "css" | "rdl" |
            "ml" | "mli" | "lua" | "scala" | "toml" |
            "zig" | "elm" | "ejs" | "erb" | "vb" |
            "tla" | "sh" | "bash"
        )
    }
    
    /// Get list of supported languages (23 working)
    pub fn supported_languages() -> Vec<&'static str> {
        vec![
            "JavaScript", "TypeScript", "TSX", "Python", "Rust",
            "Go", "C", "C++", "C#", "Ruby", "Java", "PHP",
            "Swift", "Lua", "Elixir", "Scala", "CSS", "JSON",
            "TOML", "Bash", "Elm", "Dockerfile", "Markdown"
        ]
    }
    
    /// Get list of supported file extensions
    pub fn supported_extensions() -> Vec<&'static str> {
        vec![
            "js", "jsx", "ts", "tsx", "py", "rs", "go",
            "c", "h", "cpp", "hpp", "cs", "rb", "java",
            "php", "swift", "lua", "ex", "exs", "scala",
            "css", "json", "toml", "sh", "bash", "elm",
            "md", "markdown"
        ]
    }
}

impl Default for CodexSymbolExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function for quick extraction from a file
pub fn extract_symbols(file_path: &str, source_code: &str) -> Option<String> {
    CodexSymbolExtractor::new().extract_from_file(file_path, source_code)
}

/// Convenience function for quick extraction from a directory
pub fn extract_symbols_from_directory(dir_path: &str) -> String {
    CodexSymbolExtractor::new().extract_from_directory(dir_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_supported_extensions() {
        let extractor = CodexSymbolExtractor::new();
        
        // Test supported extensions
        assert!(extractor.is_supported_file("test.rs"));
        assert!(extractor.is_supported_file("main.js"));
        assert!(extractor.is_supported_file("app.tsx"));
        assert!(extractor.is_supported_file("script.py"));
        
        // Test unsupported extensions
        assert!(!extractor.is_supported_file("doc.txt"));
        assert!(!extractor.is_supported_file("image.png"));
        assert!(!extractor.is_supported_file("data.csv"));
    }
    
    #[test]
    fn test_extract_from_rust_code() {
        let code = r#"
fn main() {
    println!("Hello, world!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
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
"#;
        
        let result = extract_symbols("test.rs", code);
        assert!(result.is_some());
        
        if let Some(output) = result {
            // Should contain filename header
            assert!(output.contains("# test.rs"));
            // Should have line ranges in correct format
            assert!(output.contains("--"));
            assert!(output.contains(" | "));
        }
    }
    
    #[test]
    fn test_supported_languages_count() {
        let languages = CodexSymbolExtractor::supported_languages();
        assert_eq!(languages.len(), 23); // We support 23 languages
    }
}
