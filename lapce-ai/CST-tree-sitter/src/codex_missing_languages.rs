//! Add support for missing languages from Codex (38 total languages)

use tree_sitter::{Parser, Language};

/// Get tree-sitter language for the missing Codex languages
pub fn get_missing_language(extension: &str) -> Option<Language> {
    unsafe {
        match extension {
            // TLA+ 
            // "tla" => Some(tree_sitter_tlaplus::language()), // Not available
            
            // Vue
            // "vue" => Some(tree_sitter_vue::language()), // Not available
            
            // Solidity
            "sol" => Some(tree_sitter_solidity::language()),
            
            // Kotlin
            "kt" | "kts" => Some(tree_sitter_kotlin::language()),
            
            // OCaml
            "ml" | "mli" => Some(tree_sitter_ocaml::LANGUAGE_OCAML.into()),
            
            // SystemRDL
            // "rdl" => Some(tree_sitter_systemrdl::language()), // Not available
            
            // Zig
            "zig" => Some(tree_sitter_zig::language()),
            
            // Embedded Template (EJS/ERB)
            "ejs" | "erb" => Some(Language::from(tree_sitter_embedded_template::LANGUAGE)),
            
            // Elisp
            // "el" => Some(tree_sitter_elisp::language()), // Not available
            
            // HTML
            "html" | "htm" => Some(tree_sitter_html::LANGUAGE.into()),
            
            // Visual Basic .NET
            // "vb" => Some(tree_sitter_vb::language()), // Not available
            
            _ => None,
        }
    }
}

/// Check if we already support this language
pub fn is_already_supported(extension: &str) -> bool {
    matches!(extension, 
        "js" | "jsx" | "json" |  // JavaScript
        "ts" | "tsx" |           // TypeScript
        "py" |                   // Python
        "rs" |                   // Rust
        "go" |                   // Go
        "c" | "h" |              // C
        "cpp" | "hpp" |          // C++
        "cs" |                   // C#
        "rb" |                   // Ruby
        "java" |                 // Java
        "php" |                  // PHP
        "swift" |                // Swift
        "lua" |                  // Lua
        "ex" | "exs" |           // Elixir
        "scala" |                // Scala
        "css" |                  // CSS
        "toml" |                 // TOML
        "sh" | "bash" |          // Bash
        "elm" |                  // Elm
        "md" | "markdown"        // Markdown
    )
}

/// Get all Codex supported extensions (38 languages)
pub fn get_all_codex_extensions() -> Vec<&'static str> {
    vec![
        // Already supported (22)
        "js", "jsx", "json", "ts", "tsx", "py", "rs", "go",
        "c", "h", "cpp", "hpp", "cs", "rb", "java", "php",
        "swift", "lua", "ex", "exs", "scala", "css", "toml",
        "sh", "bash", "elm", "md", "markdown",
        
        // Missing (16)
        "tla",           // TLA+
        "vue",           // Vue
        "sol",           // Solidity
        "kt", "kts",     // Kotlin
        "ml", "mli",     // OCaml
        "rdl",           // SystemRDL
        "zig",           // Zig
        "ejs", "erb",    // Embedded Template
        "el",            // Elisp
        "html", "htm",   // HTML
        "vb",            // Visual Basic .NET
    ]
}
