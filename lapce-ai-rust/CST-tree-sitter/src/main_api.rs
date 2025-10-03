//! MAIN API MODULE - Single entry point for all tree-sitter functionality
//! This is THE module that Lapce IDE and other systems should use

use crate::codex_integration::CodexSymbolExtractor;
use std::path::Path;

/// Main Tree-Sitter API for Lapce IDE
/// Provides all parsing, symbol extraction, and code intelligence features
pub struct LapceTreeSitterAPI {
    symbol_extractor: CodexSymbolExtractor,
}

impl LapceTreeSitterAPI {
    /// Create new API instance with default settings
    pub fn new() -> Self {
        Self {
            symbol_extractor: CodexSymbolExtractor::new(),
        }
    }
    
    /// Extract symbols from source code (PRIMARY API)
    /// 
    /// # Arguments
    /// * `file_path` - Path to the file (used for extension detection)
    /// * `source_code` - The source code content
    /// 
    /// # Returns
    /// String in exact Codex format: "# filename\n1--10 | function myFunc()\n..."
    /// Returns None if language is not supported
    /// 
    /// # Example
    /// ```
    /// let api = LapceTreeSitterAPI::new();
    /// let code = "fn main() { println!(\"Hello\"); }";
    /// if let Some(symbols) = api.extract_symbols("main.rs", code) {
    ///     println!("{}", symbols);
    /// }
    /// ```
    pub fn extract_symbols(&self, file_path: &str, source_code: &str) -> Option<String> {
        self.symbol_extractor.extract_from_file(file_path, source_code)
    }
    
    /// Extract symbols from a file on disk
    pub fn extract_symbols_from_path(&self, file_path: &str) -> Option<String> {
        self.symbol_extractor.extract_from_file_path(file_path)
    }
    
    /// Extract symbols from all files in a directory
    /// 
    /// # Arguments
    /// * `dir_path` - Path to the directory
    /// 
    /// # Returns
    /// Combined string with all symbols from all files (max 50 files)
    /// Respects .gitignore by default
    pub fn extract_symbols_from_directory(&self, dir_path: &str) -> String {
        self.symbol_extractor.extract_from_directory(dir_path)
    }
    
    /// Check if a file is supported
    pub fn is_file_supported(&self, file_path: &str) -> bool {
        self.symbol_extractor.is_supported_file(file_path)
    }
    
    /// Get list of supported languages (23 total)
    pub fn get_supported_languages(&self) -> Vec<&'static str> {
        CodexSymbolExtractor::supported_languages()
    }
    
    /// Get list of supported file extensions
    pub fn get_supported_extensions(&self) -> Vec<&'static str> {
        CodexSymbolExtractor::supported_extensions()
    }
    
    /// Get detailed language support status
    pub fn get_language_status(&self) -> LanguageStatus {
        LanguageStatus {
            working: vec![
                "JavaScript", "TypeScript", "TSX", "Python", "Rust",
                "Go", "C", "C++", "C#", "Ruby", "Java", "PHP",
                "Swift", "Lua", "Elixir", "Scala", "CSS", "JSON",
                "TOML", "Bash", "Elm", "Dockerfile", "Markdown"
            ],
            unavailable: vec![
                "Vue", "Solidity", "Kotlin", "Elisp", "HTML",
                "SystemRDL", "OCaml", "Zig", "TLA+", "Embedded Template",
                "Visual Basic", "YAML", "Haskell", "Clojure", "Dart"
            ],
            total_supported: 23,
            total_from_codex: 38,
        }
    }
}

impl Default for LapceTreeSitterAPI {
    fn default() -> Self {
        Self::new()
    }
}

/// Language support status information
#[derive(Debug, Clone)]
pub struct LanguageStatus {
    pub working: Vec<&'static str>,
    pub unavailable: Vec<&'static str>,
    pub total_supported: usize,
    pub total_from_codex: usize,
}

impl LanguageStatus {
    pub fn coverage_percentage(&self) -> f64 {
        (self.total_supported as f64 / self.total_from_codex as f64) * 100.0
    }
}

/// Quick extraction function for convenience
pub fn extract(file_path: &str, source_code: &str) -> Option<String> {
    LapceTreeSitterAPI::new().extract_symbols(file_path, source_code)
}

/// Quick directory extraction function
pub fn extract_from_directory(dir_path: &str) -> String {
    LapceTreeSitterAPI::new().extract_symbols_from_directory(dir_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_creation() {
        let api = LapceTreeSitterAPI::new();
        assert_eq!(api.get_supported_languages().len(), 23);
    }
    
    #[test]
    fn test_language_status() {
        let api = LapceTreeSitterAPI::new();
        let status = api.get_language_status();
        assert_eq!(status.total_supported, 23);
        assert_eq!(status.total_from_codex, 38);
        assert!(status.coverage_percentage() > 60.0);
    }
    
    #[test]
    fn test_extract_rust() {
        let code = "fn main() {\n    println!(\"test\");\n}\n\nstruct Person {\n    name: String\n}";
        let result = extract("test.rs", code);
        assert!(result.is_some());
        if let Some(output) = result {
            assert!(output.contains("# test.rs"));
            assert!(output.contains("--"));
        }
    }
}
