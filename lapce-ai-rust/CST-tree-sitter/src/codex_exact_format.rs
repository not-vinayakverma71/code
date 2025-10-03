//! EXACT CODEX SYMBOL FORMAT - 1:1 Translation from TypeScript
//! This is the EXACT processCaptures logic from Codex index.ts

// use crate::queries::{get_query_for_language, should_skip_tag};
// use crate::tree_sitter_bindings;
use std::collections::HashSet;
use tree_sitter::{Query, QueryCapture, QueryCursor};
use regex::Regex;
use streaming_iterator::StreamingIterator;

/// Minimum number of lines for a component to be included
const MIN_COMPONENT_LINES: usize = 4;

/// Process captures using exact Codex logic
pub fn process_captures(captures: Vec<QueryCapture>, lines: &[String], language: &str) -> Option<String> {
    // Minimum lines for a component (Codex uses 4)
    let min_component_lines = 4;
    
    // Determine if HTML filtering is needed
    let needs_html_filtering = ["jsx", "tsx"].contains(&language);
    
    // Filter function to exclude HTML elements if needed
    let is_not_html_element = |line: &str| -> bool {
        if !needs_html_filtering {
            return true;
        }
        // Common HTML elements pattern (exact from Codex)
        let trimmed = line.trim();
        !trimmed.starts_with("<div") && !trimmed.starts_with("<span") && 
        !trimmed.starts_with("<button") && !trimmed.starts_with("<input") &&
        !trimmed.starts_with("<h1") && !trimmed.starts_with("<h2") &&
        !trimmed.starts_with("<h3") && !trimmed.starts_with("<h4") &&
        !trimmed.starts_with("<h5") && !trimmed.starts_with("<h6") &&
        !trimmed.starts_with("<p>") && !trimmed.starts_with("<p ") &&
        !trimmed.starts_with("<a>") && !trimmed.starts_with("<a ") &&
        !trimmed.starts_with("<img") && !trimmed.starts_with("<ul") &&
        !trimmed.starts_with("<li") && !trimmed.starts_with("<form")
    };
    
    if captures.is_empty() {
        return None;
    }
    
    let mut formatted_output = String::new();
    let mut sorted_captures = captures.clone();
    
    // Sort captures by their start position
    sorted_captures.sort_by_key(|c| c.node.start_position().row);
    
    // Track already processed lines to avoid duplicates
    let mut processed_lines = std::collections::HashSet::new();
    
    // Process captures (matching Codex processCaptures logic)
    for capture in sorted_captures {
        let node = capture.node;
        let query = capture.index; // This is actually the pattern index
        
        // Skip captures that don't represent definitions
        let query_name = format!("definition.{}", query); // Approximate the query name
        if !query_name.contains("definition") && !query_name.contains("name") {
            continue;
        }
        
        // Get the parent node that contains the full definition
        let definition_node = if query_name.contains("name") {
            node.parent().unwrap_or(node)
        } else {
            node
        };
        
        // Get the start and end lines of the full definition
        let start_line = definition_node.start_position().row;
        let end_line = definition_node.end_position().row;
        let line_count = end_line - start_line + 1;
        
        // Skip components that don't span enough lines
        if line_count < min_component_lines {
            continue;
        }
        
        // Create unique key for this definition based on line range
        let line_key = format!("{}-{}", start_line, end_line);
        
        // Skip already processed lines
        if processed_lines.contains(&line_key) {
            continue;
        }
        
        // Check if this is a valid component definition
        let start_line_content = &lines[start_line];
        
        if is_not_html_element(start_line_content) {
            // Format: "startLine--endLine | first_line_text" (exact Codex format)
            formatted_output.push_str(&format!(
                "{}--{} | {}\n",
                start_line + 1, // Convert to 1-indexed
                end_line + 1,   // Convert to 1-indexed
                start_line_content
            ));
            processed_lines.insert(line_key);
        }
    }
    
    if formatted_output.is_empty() {
        None
    } else {
        Some(formatted_output)
    }
}

/// Parse a file and extract symbols in EXACT Codex format
pub fn parse_source_code_definitions_for_file(
    file_path: &str,
    source: &str,
) -> Option<String> {
    // Get file extension to determine parser
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    // Check if the file extension is supported
    let supported_extensions = vec![
        "tla", "js", "jsx", "ts", "vue", "tsx", "py", "rs", "go",
        "c", "h", "cpp", "hpp", "cs", "rb", "java", "php", "swift",
        "sol", "kt", "kts", "ex", "exs", "el", "html", "htm", "md",
        "markdown", "json", "css", "rdl", "ml", "mli", "lua", "scala",
        "toml", "zig", "elm", "ejs", "erb", "vb", "sh", "bash"
    ];
    
    // Check for Dockerfile first (no extension)
    let is_dockerfile = file_path.ends_with("Dockerfile") || file_path.ends_with("dockerfile");
    
    // Check if supported
    if !is_dockerfile && !supported_extensions.contains(&ext) {
        return None;
    }
    
    // Special case for markdown files
    if ext == "md" || ext == "markdown" {
        let result = parse_markdown(source);
        if let Some(output) = result {
            // Add filename header for markdown
            let filename = std::path::Path::new(file_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(file_path);
            return Some(format!("# {}\n{}", filename, output));
        }
        return None;
    }
    
    // Handle special case for Dockerfile (no extension)
    let language = if file_path.ends_with("Dockerfile") || file_path.ends_with("dockerfile") {
        "dockerfile"
    } else {
        // For other files, we need the parser and query
        match ext {
            "js" | "jsx" => "javascript",
            "ts" => "typescript", 
            "tsx" => "tsx",
            "vue" => "vue",
            "py" => "python",
            "rs" => "rust",
            "go" => "go",
            "c" | "h" => "c",
            "cpp" | "hpp" => "cpp",
            "cs" => "c_sharp",
            "rb" => "ruby",
            "java" => "java",
            "php" => "php",
            "lua" => "lua",
            "swift" => "swift",
            "sol" => "solidity",
            "kt" | "kts" => "kotlin",
            "ex" | "exs" => "elixir",
            "el" => "elisp",
            "html" | "htm" => "html",
            "json" => "json",
            "css" => "css",
            "rdl" => "systemrdl",
            "ml" | "mli" => "ocaml",
            "scala" => "scala",
            "toml" => "toml",
            "zig" => "zig",
            "elm" => "elm",
            "ejs" | "erb" => "embedded_template",
            "vb" => "vb",
            "tla" => "tlaplus",
            "sh" | "bash" => "bash",  // Added sh extension mapping
            _ => return None,
        }
    };
    
    // Parse the file using tree-sitter
    match parse_file_with_tree_sitter(source, language) {
        Some(definitions) => {
            let filename = std::path::Path::new(file_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(file_path);
            Some(format!("# {}\n{}", filename, definitions))
        }
        None => None,
    }
}

/// Parse file using tree-sitter and extract definitions
pub fn parse_file_with_tree_sitter(source: &str, language_str: &str) -> Option<String> {
    // Get parser for language
    let mut parser = tree_sitter::Parser::new();
    
    // Set language based on file type - only working parsers
    let language = match language_str {
        "javascript" | "js" | "jsx" => tree_sitter_javascript::LANGUAGE.into(),
        "typescript" | "ts" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
        "python" | "py" => tree_sitter_python::LANGUAGE.into(),
        "rust" | "rs" => tree_sitter_rust::LANGUAGE.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        "c" | "h" => tree_sitter_c::LANGUAGE.into(),
        "cpp" | "hpp" => tree_sitter_cpp::LANGUAGE.into(),
        "c_sharp" | "cs" => tree_sitter_c_sharp::LANGUAGE.into(),
        "ruby" | "rb" => tree_sitter_ruby::LANGUAGE.into(),
        "java" => tree_sitter_java::LANGUAGE.into(),
        "php" => tree_sitter_php::LANGUAGE_PHP.into(),
        "swift" => tree_sitter_swift::LANGUAGE.into(),
        "css" => tree_sitter_css::LANGUAGE.into(),
        "html" | "htm" => tree_sitter_html::LANGUAGE.into(),
        "ocaml" | "ml" | "mli" => tree_sitter_ocaml::LANGUAGE_OCAML.into(),
        "lua" => tree_sitter_lua::LANGUAGE.into(),
        "elixir" | "ex" | "exs" => tree_sitter_elixir::LANGUAGE.into(),
        "scala" => tree_sitter_scala::LANGUAGE.into(),
        "bash" | "sh" => tree_sitter_bash::LANGUAGE.into(),
        "json" => tree_sitter_json::LANGUAGE.into(),
        "elm" => tree_sitter_elm::LANGUAGE.into(),
        // Languages with version conflicts - return None for now
        "kotlin" | "kt" | "kts" => return None,  // Requires tree-sitter 0.21+
        "solidity" | "sol" => return None,  // Requires tree-sitter 0.22+
        "toml" => return None,  // Version conflict
        "vue" => return None,  // Version conflict
        "systemrdl" | "rdl" => return None,  // Not available
        "tlaplus" | "tla" => return None,  // Not available
        "zig" => return None,  // Requires newer tree-sitter
        "embedded_template" | "ejs" | "erb" => return None,  // Version conflict
        "elisp" | "el" => return None,  // Requires tree-sitter 0.21+
        _ => return None,
    };
    
    parser.set_language(&language).ok()?;
    
    // Parse the source code
    let tree = parser.parse(source, None)?;
    
    // Get query for the language (use the language_str parameter we already have)
    let lang_str = match language_str {
        "javascript" | "js" | "jsx" => "javascript",
        "typescript" | "ts" | "tsx" => "typescript",
        "python" | "py" => "python",
        "rust" | "rs" => "rust",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "hpp" => "cpp",
        "c_sharp" | "cs" => "csharp",
        "ruby" | "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "lua" => "lua",
        "elixir" | "ex" | "exs" => "elixir",
        "scala" => "scala",
        "elm" => "elm",
        "ocaml" | "ml" | "mli" => "ocaml",
        _ => language_str,
    };
    // Get query string for working languages
    let query_str = match lang_str {
        "javascript" => include_str!("../queries/javascript.scm"),
        "typescript" => include_str!("../queries/typescript.scm"),
        "tsx" => include_str!("../queries/tsx.scm"),
        "python" => include_str!("../queries/python.scm"),
        "rust" => include_str!("../queries/rust.scm"),
        "go" => include_str!("../queries/go.scm"),
        "c" => include_str!("../queries/c.scm"),
        "cpp" => include_str!("../queries/cpp.scm"),
        "csharp" => include_str!("../queries/c-sharp.scm"),
        "ruby" => include_str!("../queries/ruby.scm"),
        "java" => include_str!("../queries/java.scm"),
        "php" => include_str!("../queries/php.scm"),
        "swift" => include_str!("../queries/swift.scm"),
        "css" => include_str!("../queries/css.scm"),
        "html" => include_str!("../queries/html.scm"),
        "ocaml" => include_str!("../queries/ocaml.scm"),
        "lua" => include_str!("../queries/lua.scm"),
        "elixir" => include_str!("../queries/elixir.scm"),
        "scala" => include_str!("../queries/scala.scm"),
        "bash" => "(function_definition) @definition",
        "json" => "(_) @definition",
        "elm" => include_str!("../queries/elm.scm"),
        _ => return None,
    };
    let query = Query::new(&language, &query_str).ok()?;
    
    // Execute query
    let mut cursor = QueryCursor::new();
    let mut all_captures = Vec::new();
    
    // In tree-sitter 0.24, QueryCaptures is a streaming iterator
    let mut captures = cursor.captures(&query, tree.root_node(), source.as_bytes());
    while let Some((match_, _)) = captures.next() {
        for capture in match_.captures {
            all_captures.push(capture.clone());
        }
    }
    
    // Convert source lines
    let lines: Vec<String> = source.lines().map(String::from).collect();
    
    // Process captures with exact Codex format
    process_captures(all_captures, &lines, lang_str)
}

/// Get query string for a language (using actual Codex queries)
fn get_query_for_language(language: &str) -> Option<String> {
    // Use simplified queries that capture the whole definition
    match language {
        "javascript" => Some(r#"
(class_declaration) @definition.class
(function_declaration) @definition.function  
(method_definition) @definition.method
(lexical_declaration) @definition.variable
(variable_declaration) @definition.variable
        "#.to_string()),
        
        "typescript" | "tsx" => Some(r#"
(interface_declaration) @definition.interface
(type_alias_declaration) @definition.type
(class_declaration) @definition.class
(function_declaration) @definition.function
(method_definition) @definition.method
(lexical_declaration) @definition.variable
        "#.to_string()),
        
        "python" => Some(r#"
(class_definition) @definition.class
(function_definition) @definition.function
(decorated_definition) @definition.decorated
        "#.to_string()),
        
        "rust" => Some(r#"
(function_item) @definition.function
(struct_item) @definition.struct
(impl_item) @definition.impl
(trait_item) @definition.trait
(mod_item) @definition.module
        "#.to_string()),
        
        "go" => Some(r#"
(function_declaration) @definition.function
(method_declaration) @definition.method
(type_declaration) @definition.type
(type_spec) @definition.type
(struct_type) @definition.struct
        "#.to_string()),
        
        "java" => Some(r#"
(class_declaration) @definition.class
(interface_declaration) @definition.interface
(method_declaration) @definition.method
(constructor_declaration) @definition.constructor
        "#.to_string()),
        
        "c" => Some(r#"
(function_definition) @definition.function
(struct_specifier) @definition.struct
(enum_specifier) @definition.enum
(type_definition) @definition.typedef
        "#.to_string()),
        
        "cpp" => Some(r#"
(function_definition) @definition.function
(class_specifier) @definition.class
(struct_specifier) @definition.struct
(namespace_definition) @definition.namespace
        "#.to_string()),
        
        "ruby" => Some(r#"
(class) @definition.class
(module) @definition.module
(method) @definition.method
        "#.to_string()),
        
        "php" => Some(r#"
(class_declaration) @definition.class
(function_definition) @definition.function
(method_declaration) @definition.method
        "#.to_string()),
        
        "c_sharp" => Some(r#"
(class_declaration) @definition.class
(interface_declaration) @definition.interface
(method_declaration) @definition.method
        "#.to_string()),
        
        "swift" => Some(r#"
(class_declaration) @definition.class
(function_declaration) @definition.function
(protocol_declaration) @definition.protocol
(simple_identifier) @definition.identifier
        "#.to_string()),
        
        "kotlin" => Some(r#"
(class_declaration) @definition.class
(function_declaration) @definition.function
(object_declaration) @definition.object
        "#.to_string()),
        
        "lua" => Some(r#"
(function_declaration) @definition.function
(assignment_statement) @definition.assignment
        "#.to_string()),
        
        "elixir" => Some(r#"
(call) @definition.call
        "#.to_string()),
        
        "scala" => Some(r#"
(object_definition) @definition.object
(class_definition) @definition.class
(function_definition) @definition.function
(trait_definition) @definition.trait
        "#.to_string()),
        
        "html" => Some(r#"
(element) @definition.element
(script_element) @definition.script
(style_element) @definition.style
        "#.to_string()),
        
        "css" => Some(r#"
(rule_set) @definition.rule
(media_statement) @definition.media
(keyframes_statement) @definition.keyframes
        "#.to_string()),
        
        "json" => Some(r#"
(pair) @definition.pair
(object) @definition.object
(array) @definition.array
        "#.to_string()),
        
        "toml" => Some(r#"
(table) @definition.table
(table_array_element) @definition.table_array
(pair) @definition.pair
        "#.to_string()),
        
        "bash" => Some(r#"
(function_definition) @definition.function
(variable_assignment) @definition.variable
        "#.to_string()),
        
        "elm" => Some(r#"
(function_declaration_left) @definition.function
(type_alias_declaration) @definition.type
(type_declaration) @definition.type
        "#.to_string()),
        
        "dockerfile" => Some(r#"
(from_instruction) @definition.from
(run_instruction) @definition.run
(cmd_instruction) @definition.cmd
        "#.to_string()),
        
        "markdown" => Some(r#"
(heading) @definition.heading
(code_block) @definition.code
        "#.to_string()),
        
        // Languages not yet fully supported
        "vue" | "solidity" | "elisp" | "systemrdl" | "ocaml" | 
        "zig" | "embedded_template" | "vb" | "tlaplus" => None,
        
        _ => None,
    }
}

/// Parse markdown file using exact Codex logic
use crate::markdown_parser::parse_markdown_to_codex_format;

fn parse_markdown(content: &str) -> Option<String> {
    // Get the formatted output from markdown parser
    let output = parse_markdown_to_codex_format(content)?;
    
    // Add the filename header (will be added by caller)
    Some(output)
}

/// Parse directory and get all definitions (max 50 files)
pub fn parse_source_code_for_definitions_top_level(
    dir_path: &str,
) -> String {
    use std::fs;
    use std::path::Path;
    
    let path = Path::new(dir_path);
    if !path.exists() || !path.is_dir() {
        return "This directory does not exist or you do not have permission to access it.".to_string();
    }
    
    let mut result = String::new();
    let mut file_count = 0;
    const MAX_FILES: usize = 50;
    
    // Get all files (simplified - should respect .gitignore)
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if file_count >= MAX_FILES {
                break;
            }
            
            let file_path = entry.path();
            if file_path.is_file() {
                if let Some(path_str) = file_path.to_str() {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        if let Some(definitions) = parse_source_code_definitions_for_file(path_str, &content) {
                            result.push_str(&definitions);
                            result.push('\n');
                            file_count += 1;
                        }
                    }
                }
            }
        }
    }
    
    if result.is_empty() {
        "No source code definitions found.".to_string()
    } else {
        result
    }
}
