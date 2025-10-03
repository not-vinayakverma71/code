/// Code Parser - Consolidated implementation with AST-aware chunking
/// Merged from code_parser.rs and code_parser_impl.rs

use tree_sitter::{Parser, Query, QueryCursor, Node, Language};
use std::sync::Arc;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use walkdir::WalkDir;

// Consolidated CodeChunk structure
#[derive(Debug, Clone)]
pub struct CodeChunk {
    pub file_path: String,
    pub content: String,
    pub language: String,
    pub start_line: u32,
    pub end_line: u32,
    pub chunk_type: Option<ChunkType>,
}

#[derive(Debug, Clone)]
pub enum ChunkType {
    Function,
    Class,
    Method,
    Module,
    Block,
}

pub struct CodeParser {
    parsers: HashMap<String, Parser>,
    chunk_size: usize,
    chunk_overlap: usize,
    max_chunk_size: usize,
}

impl CodeParser {
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();
        
        // Initialize parsers for each language
        parsers.insert("rust".to_string(), Self::create_parser(tree_sitter_rust::language())?);
        parsers.insert("python".to_string(), Self::create_parser(tree_sitter_python::language())?);
        parsers.insert("javascript".to_string(), Self::create_parser(tree_sitter_javascript::language())?);
        parsers.insert("typescript".to_string(), Self::create_parser(tree_sitter_typescript::language_typescript())?);
        parsers.insert("go".to_string(), Self::create_parser(tree_sitter_go::language())?);
        parsers.insert("java".to_string(), Self::create_parser(tree_sitter_java::language())?);
        parsers.insert("c".to_string(), Self::create_parser(tree_sitter_c::language())?);
        parsers.insert("cpp".to_string(), Self::create_parser(tree_sitter_cpp::language())?);
        
        Ok(Self {
            parsers,
            chunk_size: 15,      // 15 lines per chunk
            chunk_overlap: 5,    // 5 lines overlap
            max_chunk_size: 512, // Max 512 tokens for embedding
        })
    }
    
    fn create_parser(language: Language) -> Result<Parser> {
        let mut parser = Parser::new();
        parser.set_language(language)?;
        Ok(parser)
    }
    
    pub async fn parse_file(&self, file_path: &Path) -> Result<Vec<CodeChunk>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let language = Self::detect_language(file_path);
        
        // Use AST-aware chunking for supported languages
        if let Some(parser) = self.parsers.get(&language) {
            self.ast_aware_chunking(parser, &content, &language)
        } else {
            // Fallback to line-based chunking
            Ok(self.line_based_chunking(&content, &language))
        }
    }
    
    fn ast_aware_chunking(&self, parser: &Parser, content: &str, language: &str) -> Result<Vec<CodeChunk>> {
        let mut chunks = Vec::new();
        
        // Parse the content
        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse"))?;
        
        let root = tree.root_node();
        
        // Extract semantic units (functions, classes, etc.)
        self.extract_semantic_units(root, content, language, &mut chunks);
        
        // If no semantic units found or file is small, use line-based
        if chunks.is_empty() {
            return Ok(self.line_based_chunking(content, language));
        }
        
        Ok(chunks)
    }
    
    fn extract_semantic_units(&self, node: Node, content: &str, language: &str, chunks: &mut Vec<CodeChunk>) {
        // Define what constitutes a semantic unit per language
        let semantic_kinds = match language {
            "rust" => vec!["function_item", "impl_item", "struct_item", "trait_item", "mod_item"],
            "python" => vec!["function_definition", "class_definition"],
            "javascript" | "typescript" => vec!["function_declaration", "class_declaration", "arrow_function", "method_definition"],
            "go" => vec!["function_declaration", "method_declaration", "type_declaration"],
            "java" => vec!["method_declaration", "class_declaration", "interface_declaration"],
            "c" | "cpp" => vec!["function_definition", "class_specifier", "struct_specifier"],
            _ => vec![],
        };
        
        // Check if current node is a semantic unit
        if semantic_kinds.contains(&node.kind()) {
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            let chunk_content = &content[start_byte..end_byte];
            
            // Get line numbers
            let start_line = content[..start_byte].lines().count();
            let end_line = start_line + chunk_content.lines().count();
            
            // Include docstring/comments if present
            let chunk_with_context = self.include_context(node, content, start_byte, end_byte);
            
            chunks.push(CodeChunk {
                path: String::new(), // Path will be set by caller
                content: chunk_with_context,
                language: language.to_string(),
                start_line,
                end_line,
            });
        }
        
        // Recursively process children
        for child in node.children(&mut node.walk()) {
            self.extract_semantic_units(child, content, language, chunks);
        }
    }
    
    fn include_context(&self, node: Node, content: &str, start: usize, end: usize) -> String {
        let mut context_start = start;
        
        // Look for preceding comments
        if let Some(prev) = node.prev_sibling() {
            if prev.kind().contains("comment") {
                context_start = prev.start_byte();
            }
        }
        
        // Look for docstrings (Python)
        if node.kind() == "function_definition" || node.kind() == "class_definition" {
            if let Some(body) = node.child_by_field_name("body") {
                if let Some(first_stmt) = body.child(0) {
                    if first_stmt.kind() == "expression_statement" {
                        if let Some(string) = first_stmt.child(0) {
                            if string.kind() == "string" {
                                // Include docstring in chunk
                                return content[context_start..string.end_byte()].to_string();
                            }
                        }
                    }
                }
            }
        }
        
        content[context_start..end].to_string()
    }
    
    fn line_based_chunking(&self, content: &str, language: &str) -> Vec<CodeChunk> {
        let lines: Vec<&str> = content.lines().collect();
        let mut chunks = Vec::new();
        
        let stride = self.chunk_size - self.chunk_overlap;
        
        for i in (0..lines.len()).step_by(stride) {
            let end = (i + self.chunk_size).min(lines.len());
            
            // Skip very small chunks
            if end - i < 3 {
                continue;
            }
            
            let chunk_lines = &lines[i..end];
            let chunk_content = chunk_lines.join("\n");
            
            // Skip empty or whitespace-only chunks
            if chunk_content.trim().is_empty() {
                continue;
            }
            
            chunks.push(CodeChunk {
                path: String::new(), // Path will be set by caller
                content: chunk_content,
                language: language.to_string(),
                start_line: i + 1,
                end_line: end,
            });
        }
        
        chunks
    }
    
    fn detect_language(file_path: &Path) -> String {
        match file_path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => "rust",
            Some("py") => "python",
            Some("js") | Some("mjs") => "javascript",
            Some("ts") | Some("tsx") => "typescript",
            Some("go") => "go",
            Some("java") => "java",
            Some("c") | Some("h") => "c",
            Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") => "cpp",
            Some("rb") => "ruby",
            Some("php") => "php",
            Some("swift") => "swift",
            Some("kt") => "kotlin",
            Some("scala") => "scala",
            Some("sh") | Some("bash") => "shell",
            Some("sql") => "sql",
            Some("json") => "json",
            Some("yaml") | Some("yml") => "yaml",
            Some("toml") => "toml",
            Some("xml") => "xml",
            Some("md") => "markdown",
            _ => "text",
        }.to_string()
    }
    
    pub async fn collect_repository_files(&self, repo_path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for entry in WalkDir::new(repo_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Skip directories
            if path.is_dir() {
                continue;
            }
            
            // Skip common non-code directories
            let path_str = path.to_string_lossy();
            if path_str.contains("target/") ||
               path_str.contains("node_modules/") ||
               path_str.contains(".git/") ||
               path_str.contains("dist/") ||
               path_str.contains("build/") ||
               path_str.contains("__pycache__/") ||
               path_str.contains(".venv/") {
                continue;
            }
            
            // Check if it's a code file
            if Self::is_code_file(path) {
                files.push(path.to_path_buf());
            }
        }
        
        Ok(files)
    }
    
    pub fn is_code_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            matches!(ext_str.as_ref(),
                "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java" |
                "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" | "cs" | "rb" |
                "php" | "swift" | "kt" | "scala" | "m" | "mm" | "sh" | "bash" |
                "sql" | "r" | "R" | "jl" | "lua" | "pl" | "pm" | "tcl" |
                "vim" | "el" | "clj" | "cljs" | "ex" | "exs" | "erl" | "hrl" |
                "ml" | "mli" | "fs" | "fsi" | "fsx" | "hs" | "lhs" | "elm" |
                "purs" | "nim" | "cr" | "d" | "dart" | "zig" | "v" | "vala" |
                "json" | "yaml" | "yml" | "toml" | "xml" | "md" | "rst" | "tex"
            )
        } else {
            // Check for extensionless files that might be code
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                matches!(name_str.as_ref(),
                    "Makefile" | "Dockerfile" | "Jenkinsfile" | "Rakefile" | 
                    "Gemfile" | "Guardfile" | "Procfile" | "Vagrantfile"
                )
            } else {
                false
            }
        }
    }
}

/// Parallel file processor for faster indexing
pub struct ParallelProcessor {
    parser: Arc<CodeParser>,
    batch_size: usize,
}

impl ParallelProcessor {
    pub fn new(parser: CodeParser) -> Self {
        Self {
            parser: Arc::new(parser),
            batch_size: 100,
        }
    }
    
    pub async fn process_files(&self, files: Vec<PathBuf>) -> Result<HashMap<PathBuf, Vec<CodeChunk>>> {
        use futures::stream::{self, StreamExt};
        
        let results = stream::iter(files)
            .map(|path| {
                let parser = self.parser.clone();
                async move {
                    match parser.parse_file(&path).await {
                        Ok(chunks) => Some((path, chunks)),
                        Err(e) => {
                            eprintln!("Failed to parse {:?}: {}", path, e);
                            None
                        }
                    }
                }
            })
            .buffer_unordered(10) // Process 10 files concurrently
            .filter_map(|x| async { x })
            .collect::<HashMap<_, _>>()
            .await;
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_language_detection() {
        assert_eq!(CodeParser::detect_language(Path::new("test.rs")), "rust");
        assert_eq!(CodeParser::detect_language(Path::new("test.py")), "python");
        assert_eq!(CodeParser::detect_language(Path::new("test.js")), "javascript");
        assert_eq!(CodeParser::detect_language(Path::new("test.go")), "go");
        assert_eq!(CodeParser::detect_language(Path::new("unknown.xyz")), "text");
    }
    
    #[test]
    fn test_is_code_file() {
        assert!(CodeParser::is_code_file(Path::new("test.rs")));
        assert!(CodeParser::is_code_file(Path::new("test.py")));
        assert!(CodeParser::is_code_file(Path::new("Dockerfile")));
        assert!(!CodeParser::is_code_file(Path::new("image.jpg")));
        assert!(!CodeParser::is_code_file(Path::new("document.pdf")));
    }
}
