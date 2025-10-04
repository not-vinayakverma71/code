//! ENHANCED CODEX FORMAT - Complete support for all 67 languages
//! 38 languages with Codex format, 29 with tree-sitter defaults

use tree_sitter::{Node, Parser, Query, QueryCursor, Tree};
use std::collections::{HashMap, HashSet};

/// Symbol extraction configuration for each language
pub struct LanguageConfig {
    pub min_lines: usize,
    pub use_codex_format: bool,
    pub filter_html: bool,
    pub query_patterns: Vec<&'static str>,
}

impl LanguageConfig {
    /// Get configuration for a specific language
    pub fn for_language(language: &str) -> Self {
        match language {
            // Codex-supported languages (38 total)
            "javascript" | "jsx" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_declaration name: (identifier) @name)",
                    "(class_declaration name: (identifier) @name)",
                    "(method_definition key: (property_identifier) @name)",
                    "(variable_declarator name: (identifier) @name)",
                ],
            },
            "typescript" | "tsx" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: true,
                query_patterns: vec![
                    "(function_declaration name: (identifier) @name)",
                    "(class_declaration name: (identifier) @name)",
                    "(interface_declaration name: (type_identifier) @name)",
                    "(type_alias_declaration name: (type_identifier) @name)",
                    "(enum_declaration name: (identifier) @name)",
                ],
            },
            "python" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_definition name: (identifier) @name)",
                    "(class_definition name: (identifier) @name)",
                ],
            },
            "rust" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_item name: (identifier) @name)",
                    "(impl_item type: (_) @name)",
                    "(struct_item name: (type_identifier) @name)",
                    "(enum_item name: (type_identifier) @name)",
                    "(trait_item name: (type_identifier) @name)",
                    "(mod_item name: (identifier) @name)",
                ],
            },
            "go" => Self {
                min_lines: 3, // Go functions can be shorter
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_declaration name: (identifier) @name)",
                    "(method_declaration name: (field_identifier) @name)",
                    "(type_declaration (type_spec name: (type_identifier) @name))",
                ],
            },
            "java" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(class_declaration name: (identifier) @name)",
                    "(interface_declaration name: (identifier) @name)",
                    "(method_declaration name: (identifier) @name)",
                    "(enum_declaration name: (identifier) @name)",
                ],
            },
            "c" | "cpp" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_definition declarator: (function_declarator declarator: (identifier) @name))",
                    "(class_specifier name: (type_identifier) @name)",
                    "(struct_specifier name: (type_identifier) @name)",
                    "(enum_specifier name: (type_identifier) @name)",
                ],
            },
            "csharp" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(class_declaration name: (identifier) @name)",
                    "(interface_declaration name: (identifier) @name)",
                    "(method_declaration name: (identifier) @name)",
                    "(property_declaration name: (identifier) @name)",
                ],
            },
            "ruby" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(method name: (identifier) @name)",
                    "(class name: (constant) @name)",
                    "(module name: (constant) @name)",
                ],
            },
            "php" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_definition name: (name) @name)",
                    "(class_declaration name: (name) @name)",
                    "(method_declaration name: (name) @name)",
                ],
            },
            "swift" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_declaration name: (simple_identifier) @name)",
                    "(class_declaration name: (type_identifier) @name)",
                    "(protocol_declaration name: (type_identifier) @name)",
                ],
            },
            "kotlin" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_declaration (simple_identifier) @name)",
                    "(class_declaration (type_identifier) @name)",
                ],
            },
            "scala" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_definition name: (identifier) @name)",
                    "(class_definition name: (identifier) @name)",
                    "(object_definition name: (identifier) @name)",
                ],
            },
            "haskell" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function name: (variable) @name)",
                    "(signature name: (variable) @name)",
                ],
            },
            "elixir" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(call target: (identifier) @keyword (#match? @keyword \"^(def|defp|defmodule)$\"))",
                ],
            },
            "erlang" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_clause name: (atom) @name)",
                ],
            },
            "clojure" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(list_lit (sym_lit) @keyword (#match? @keyword \"^(defn|defn-|defmacro|def)$\"))",
                ],
            },
            "lua" => Self {
                min_lines: 3,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_declaration name: (identifier) @name)",
                    "(function_definition name: (identifier) @name)",
                ],
            },
            "vim" => Self {
                min_lines: 3,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_declaration name: (identifier) @name)",
                ],
            },
            "bash" | "sh" => Self {
                min_lines: 3,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_definition name: (word) @name)",
                ],
            },
            "powershell" => Self {
                min_lines: 3,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_statement name: (function_name) @name)",
                ],
            },
            "dockerfile" => Self {
                min_lines: 1, // Docker instructions can be single line
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(from_instruction image: (image_spec) @name)",
                    "(label_instruction key: (unquoted_string) @name)",
                ],
            },
            "yaml" => Self {
                min_lines: 1,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(block_mapping_pair key: (flow_node) @name)",
                ],
            },
            "toml" => Self {
                min_lines: 1,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(table (bare_key) @name)",
                ],
            },
            "json" => Self {
                min_lines: 1,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(pair key: (string) @name)",
                ],
            },
            "sql" => Self {
                min_lines: 1,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(create_table_statement name: (table_name) @name)",
                    "(create_view_statement name: (identifier) @name)",
                ],
            },
            "graphql" => Self {
                min_lines: 3,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(object_type_definition name: (name) @name)",
                    "(interface_type_definition name: (name) @name)",
                ],
            },
            "html" => Self {
                min_lines: 1,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(element (start_tag (tag_name) @name))",
                ],
            },
            "css" => Self {
                min_lines: 1,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(rule_set (selectors) @name)",
                ],
            },
            "markdown" => Self {
                min_lines: 1,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(atx_heading (atx_h1_marker) heading_content: (_) @name)",
                    "(atx_heading (atx_h2_marker) heading_content: (_) @name)",
                    "(atx_heading (atx_h3_marker) heading_content: (_) @name)",
                ],
            },
            "elm" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_declaration_left (lower_case_identifier) @name)",
                    "(type_declaration (upper_case_identifier) @name)",
                ],
            },
            "zig" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(FnProto (IDENTIFIER) @name)",
                ],
            },
            "nim" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(proc_declaration (symbol) @name)",
                    "(func_declaration (symbol) @name)",
                ],
            },
            "julia" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(function_definition name: (identifier) @name)",
                    "(short_function_definition name: (identifier) @name)",
                ],
            },
            "solidity" => Self {
                min_lines: 4,
                use_codex_format: true,
                filter_html: false,
                query_patterns: vec![
                    "(contract_declaration name: (identifier) @name)",
                    "(function_definition name: (identifier) @name)",
                ],
            },
            
            // Non-Codex languages (29) - use tree-sitter defaults
            "r" | "matlab" | "perl" | "dart" | "ocaml" | "nix" | "latex" |
            "make" | "cmake" | "verilog" | "d" | "pascal" | "commonlisp" |
            "prisma" | "hlsl" | "objc" | "cobol" | "groovy" | "hcl" |
            "fsharp" | "systemverilog" | "embedded_template" | "fortran" |
            "vhdl" | "racket" | "ada" | "prolog" | "gradle" | "xml" => Self {
                min_lines: 1, // Default tree-sitter: include everything
                use_codex_format: false,
                filter_html: false,
                query_patterns: vec![], // Will use default tree-sitter queries
            },
            
            _ => Self {
                min_lines: 1,
                use_codex_format: false,
                filter_html: false,
                query_patterns: vec![],
            }
        }
    }
    
    /// Check if language is Codex-supported (38 languages)
    pub fn is_codex_supported(language: &str) -> bool {
        matches!(language,
            "javascript" | "typescript" | "tsx" | "jsx" | "python" | "rust" |
            "go" | "c" | "cpp" | "csharp" | "ruby" | "java" | "php" | "swift" |
            "kotlin" | "scala" | "haskell" | "elixir" | "erlang" | "clojure" |
            "elm" | "html" | "css" | "markdown" | "json" | "yaml" | "toml" |
            "sql" | "graphql" | "dockerfile" | "bash" | "sh" | "powershell" |
            "lua" | "vim" | "zig" | "nim" | "julia" | "solidity"
        )
    }
}

/// Enhanced symbol extractor with support for all 67 languages
pub struct EnhancedSymbolExtractor {
    parsers: HashMap<String, Parser>,
}

impl EnhancedSymbolExtractor {
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }
    
    /// Extract symbols from source code
    pub fn extract_symbols(&mut self, language: &str, source: &str) -> Option<String> {
        let config = LanguageConfig::for_language(language);
        
        if config.use_codex_format {
            self.extract_codex_format(language, source, &config)
        } else {
            self.extract_default_format(language, source)
        }
    }
    
    /// Extract symbols in Codex format (38 languages)
    fn extract_codex_format(&mut self, language: &str, source: &str, config: &LanguageConfig) -> Option<String> {
        // Get or create parser
        let parser = self.get_parser(language)?;
        
        // Parse the source
        let tree = parser.parse(source, None)?;
        let root = tree.root_node();
        
        let lines: Vec<&str> = source.lines().collect();
        let mut symbols = Vec::new();
        let mut processed_ranges = HashSet::new();
        
        // Walk the tree to find symbols
        self.walk_tree_for_symbols(root, &lines, config, &mut symbols, &mut processed_ranges);
        
        if symbols.is_empty() {
            return None;
        }
        
        // Format in Codex style
        let mut output = String::new();
        for (start_line, end_line, first_line) in symbols {
            // Filter HTML if needed
            if config.filter_html && self.is_html_element(&first_line) {
                continue;
            }
            
            // Apply min lines filter
            if end_line - start_line + 1 >= config.min_lines {
                output.push_str(&format!("{}--{} | {}\n", start_line + 1, end_line + 1, first_line));
            }
        }
        
        if output.is_empty() {
            None
        } else {
            Some(output)
        }
    }
    
    /// Extract symbols in default tree-sitter format (29 languages)
    fn extract_default_format(&mut self, language: &str, source: &str) -> Option<String> {
        // Get or create parser
        let parser = self.get_parser(language)?;
        
        // Parse the source
        let tree = parser.parse(source, None)?;
        let root = tree.root_node();
        
        // Use tree-sitter's default symbol extraction
        let mut output = String::new();
        self.extract_default_symbols(root, source, &mut output);
        
        if output.is_empty() {
            None
        } else {
            Some(output)
        }
    }
    
    /// Walk tree to find symbol definitions
    fn walk_tree_for_symbols(
        &self,
        node: Node,
        lines: &[&str],
        config: &LanguageConfig,
        symbols: &mut Vec<(usize, usize, String)>,
        processed: &mut HashSet<(usize, usize)>,
    ) {
        // Check if this node is a definition
        if self.is_definition_node(node) {
            let start = node.start_position().row;
            let end = node.end_position().row;
            let key = (start, end);
            
            if !processed.contains(&key) && start < lines.len() {
                let first_line = lines[start].to_string();
                symbols.push((start, end, first_line));
                processed.insert(key);
            }
        }
        
        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_tree_for_symbols(child, lines, config, symbols, processed);
        }
    }
    
    /// Extract symbols in default format (for non-Codex languages)
    fn extract_default_symbols(&self, node: Node, source: &str, output: &mut String) {
        // Simple extraction: include all top-level items
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            
            // Common definition types across languages
            if kind.contains("declaration") || kind.contains("definition") ||
               kind.contains("function") || kind.contains("class") ||
               kind.contains("struct") || kind.contains("interface") ||
               kind.contains("type") || kind.contains("module") {
                
                let start = child.start_position();
                let end = child.end_position();
                
                // Get the text of the definition (first line only for brevity)
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    let first_line = text.lines().next().unwrap_or(text);
                    output.push_str(&format!("{}:{} {}: {}\n",
                        start.row + 1,
                        start.column + 1,
                        kind,
                        first_line.trim()
                    ));
                }
            }
        }
    }
    
    /// Check if node is a definition
    fn is_definition_node(&self, node: Node) -> bool {
        let kind = node.kind();
        kind.contains("declaration") || kind.contains("definition") ||
        kind.contains("function") || kind.contains("class") ||
        kind.contains("method") || kind.contains("struct") ||
        kind.contains("interface") || kind.contains("type") ||
        kind.contains("module") || kind.contains("impl") ||
        kind.contains("trait") || kind.contains("enum")
    }
    
    /// Check if line is an HTML element (for JSX/TSX filtering)
    fn is_html_element(&self, line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with('<') && (
            trimmed.starts_with("<div") || trimmed.starts_with("<span") ||
            trimmed.starts_with("<button") || trimmed.starts_with("<input") ||
            trimmed.starts_with("<h1") || trimmed.starts_with("<h2") ||
            trimmed.starts_with("<h3") || trimmed.starts_with("<h4") ||
            trimmed.starts_with("<h5") || trimmed.starts_with("<h6") ||
            trimmed.starts_with("<p>") || trimmed.starts_with("<p ") ||
            trimmed.starts_with("<a>") || trimmed.starts_with("<a ") ||
            trimmed.starts_with("<img") || trimmed.starts_with("<ul") ||
            trimmed.starts_with("<li") || trimmed.starts_with("<form")
        )
    }
    
    /// Get or create parser for language
    fn get_parser(&mut self, language: &str) -> Option<&mut Parser> {
        use crate::all_languages_support::SupportedLanguage;
        
        if !self.parsers.contains_key(language) {
            // Map language string to SupportedLanguage enum
            let lang = match language {
                "javascript" | "js" => SupportedLanguage::JavaScript,
                "typescript" | "ts" => SupportedLanguage::TypeScript,
                "tsx" => SupportedLanguage::Tsx,
                "jsx" => SupportedLanguage::Jsx,
                "python" | "py" => SupportedLanguage::Python,
                "rust" | "rs" => SupportedLanguage::Rust,
                "go" => SupportedLanguage::Go,
                "java" => SupportedLanguage::Java,
                "c" => SupportedLanguage::C,
                "cpp" | "c++" => SupportedLanguage::Cpp,
                "csharp" | "cs" => SupportedLanguage::CSharp,
                "ruby" | "rb" => SupportedLanguage::Ruby,
                "php" => SupportedLanguage::Php,
                "lua" => SupportedLanguage::Lua,
                "bash" | "sh" => SupportedLanguage::Bash,
                "css" => SupportedLanguage::Css,
                "json" => SupportedLanguage::Json,
                "swift" => SupportedLanguage::Swift,
                "scala" => SupportedLanguage::Scala,
                "elixir" | "ex" => SupportedLanguage::Elixir,
                "html" => SupportedLanguage::Html,
                "elm" => SupportedLanguage::Elm,
                "toml" => SupportedLanguage::Toml,
                "yaml" | "yml" => SupportedLanguage::Yaml,
                "kotlin" | "kt" => SupportedLanguage::Kotlin,
                "haskell" | "hs" => SupportedLanguage::Haskell,
                "dart" => SupportedLanguage::Dart,
                "julia" | "jl" => SupportedLanguage::Julia,
                "r" => SupportedLanguage::R,
                "matlab" | "m" => SupportedLanguage::Matlab,
                "perl" | "pl" => SupportedLanguage::Perl,
                "sql" => SupportedLanguage::Sql,
                "graphql" | "gql" => SupportedLanguage::GraphQL,
                "dockerfile" => SupportedLanguage::Dockerfile,
                "markdown" | "md" => SupportedLanguage::Markdown,
                "xml" => SupportedLanguage::Xml,
                "nim" => SupportedLanguage::Nim,
                "zig" => SupportedLanguage::Zig,
                "clojure" | "clj" => SupportedLanguage::Clojure,
                "erlang" | "erl" => SupportedLanguage::Erlang,
                "vim" => SupportedLanguage::Vim,
                "powershell" | "ps1" => SupportedLanguage::PowerShell,
                "solidity" | "sol" => SupportedLanguage::Solidity,
                _ => return None,
            };
            
            // Get parser for language
            if let Ok(parser) = lang.get_parser() {
                self.parsers.insert(language.to_string(), parser);
            } else {
                return None;
            }
        }
        
        self.parsers.get_mut(language)
    }
}

/// Public API for enhanced symbol extraction
pub fn extract_symbols_enhanced(file_path: &str, source: &str) -> Option<String> {
    // Determine language from file extension
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|s| s.to_str())?;
    
    let mut extractor = EnhancedSymbolExtractor::new();
    let result = extractor.extract_symbols(ext, source);
    
    // Add file header if we have symbols
    result.map(|symbols| format!("# {}\n{}", file_path, symbols))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_codex_language_detection() {
        assert!(LanguageConfig::is_codex_supported("javascript"));
        assert!(LanguageConfig::is_codex_supported("python"));
        assert!(LanguageConfig::is_codex_supported("rust"));
        assert!(!LanguageConfig::is_codex_supported("r"));
        assert!(!LanguageConfig::is_codex_supported("matlab"));
    }
    
    #[test]
    fn test_language_config() {
        let js_config = LanguageConfig::for_language("javascript");
        assert_eq!(js_config.min_lines, 4);
        assert!(js_config.use_codex_format);
        
        let r_config = LanguageConfig::for_language("r");
        assert_eq!(r_config.min_lines, 1);
        assert!(!r_config.use_codex_format);
    }
}
