//! Directory traversal with gitignore support and file limits
//! Exact implementation from Codex

use std::fs;
use std::path::{Path, PathBuf};
use ignore::WalkBuilder;

const MAX_FILES: usize = 50;

/// Parse all source files in a directory (max 50 files)
/// Respects .gitignore and separates markdown from other files
pub fn parse_directory_for_definitions(
    dir_path: &str,
    respect_gitignore: bool,
) -> String {
    let path = Path::new(dir_path);
    
    if !path.exists() || !path.is_dir() {
        return "This directory does not exist or you do not have permission to access it.".to_string();
    }
    
    let mut result = String::new();
    let mut file_count = 0;
    let mut markdown_files = Vec::new();
    let mut source_files = Vec::new();
    
    // Use ignore crate for gitignore support
    let walker = WalkBuilder::new(dir_path)
        .git_ignore(respect_gitignore)
        .git_global(respect_gitignore)
        .git_exclude(respect_gitignore)
        .max_depth(Some(10)) // Reasonable depth limit
        .build();
    
    // Collect files
    for entry in walker.flatten() {
        if file_count >= MAX_FILES {
            break;
        }
        
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        
        // Get extension
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_str().unwrap_or("");
            
            // Check if it's a supported extension
            if is_supported_extension(ext_str) {
                file_count += 1;
                
                // Separate markdown from other files (like Codex does)
                if ext_str == "md" || ext_str == "markdown" {
                    markdown_files.push(path.to_path_buf());
                } else {
                    source_files.push(path.to_path_buf());
                }
            }
        }
    }
    
    // Process non-markdown files first
    for file_path in source_files {
        process_file(&file_path, &mut result);
    }
    
    // Then process markdown files
    for file_path in markdown_files {
        process_file(&file_path, &mut result);
    }
    
    if result.is_empty() {
        "No source code definitions found.".to_string()
    } else {
        result
    }
}

fn process_file(file_path: &Path, result: &mut String) {
    if let Some(path_str) = file_path.to_str() {
        if let Ok(content) = fs::read_to_string(file_path) {
            // Use the exact Codex format parser
            if let Some(definitions) = crate::codex_exact_format::parse_source_code_definitions_for_file(
                path_str, 
                &content
            ) {
                result.push_str(&definitions);
                if !definitions.ends_with('\n') {
                    result.push('\n');
                }
            }
        }
    }
}

fn is_supported_extension(ext: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_directory_traversal() {
        // Test with current directory
        let result = parse_directory_for_definitions(".", true);
        assert!(!result.contains("does not exist"));
        
        // Test with non-existent directory
        let result = parse_directory_for_definitions("/nonexistent", true);
        assert!(result.contains("does not exist"));
    }
    
    #[test]
    fn test_extension_filtering() {
        assert!(is_supported_extension("rs"));
        assert!(is_supported_extension("ts"));
        assert!(is_supported_extension("py"));
        assert!(!is_supported_extension("txt"));
        assert!(!is_supported_extension("pdf"));
    }
}
