// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// CST to AST Pipeline - Integrates with lapce-tree-sitter

use crate::error::{Error, Result};
use tree_sitter::{Node, Tree, Parser, Language, TreeCursor};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use serde::{Serialize, Deserialize};

/// CST Node - Raw concrete syntax tree from tree-sitter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CstNode {
    pub kind: String,
    pub text: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_position: (usize, usize),  // (row, column)
    pub end_position: (usize, usize),
    pub is_named: bool,
    pub is_missing: bool,
    pub is_extra: bool,
    pub field_name: Option<String>,
    pub children: Vec<CstNode>,
}

/// AST Node - Abstract syntax tree derived from CST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    pub node_type: AstNodeType,
    pub text: String,
    pub identifier: Option<String>,
    pub value: Option<String>,
    pub children: Vec<AstNode>,
    pub metadata: NodeMetadata,
    pub semantic_info: Option<SemanticInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AstNodeType {
    // Program structure
    Program,
    Module,
    Package,
    
    // Declarations
    FunctionDeclaration,
    ClassDeclaration,
    InterfaceDeclaration,
    StructDeclaration,
    EnumDeclaration,
    TraitDeclaration,
    TypeAlias,
    
    // Statements
    IfStatement,
    WhileLoop,
    ForLoop,
    SwitchStatement,
    ReturnStatement,
    BreakStatement,
    ContinueStatement,
    
    // Expressions
    BinaryExpression,
    UnaryExpression,
    CallExpression,
    MemberExpression,
    ArrayExpression,
    ObjectExpression,
    
    // Literals
    StringLiteral,
    NumberLiteral,
    BooleanLiteral,
    NullLiteral,
    
    // Variables
    VariableDeclaration,
    VariableReference,
    Parameter,
    
    // Types
    TypeAnnotation,
    GenericType,
    UnionType,
    IntersectionType,
    
    // Imports/Exports
    ImportStatement,
    ExportStatement,
    
    // Comments and Docs
    Comment,
    DocComment,
    
    // Error recovery
    Unknown,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub source_file: Option<PathBuf>,
    pub language: String,
    pub complexity: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticInfo {
    pub scope_depth: usize,
    pub symbol_table: HashMap<String, SymbolInfo>,
    pub type_info: Option<TypeInfo>,
    pub data_flow: Vec<DataFlowEdge>,
    pub control_flow: Vec<ControlFlowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub scope: String,
    pub is_exported: bool,
    pub references: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Variable,
    Class,
    Type,
    Constant,
    Module,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub type_name: String,
    pub is_generic: bool,
    pub type_parameters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowEdge {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub flow_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFlowEdge {
    pub from: (usize, usize),
    pub to: (usize, usize),
    pub condition: Option<String>,
}

/// Pipeline output containing both CST and AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
    pub cst: CstNode,
    pub ast: AstNode,
    pub source_file: PathBuf,
    pub language: String,
    pub parse_time_ms: f64,
    pub transform_time_ms: f64,
}

/// CST to AST transformation pipeline
pub struct CstToAstPipeline {
    /// Language-specific transformers
    transformers: HashMap<String, Box<dyn LanguageTransformer>>,
    
    /// AST cache for processed files
    ast_cache: Arc<dashmap::DashMap<PathBuf, AstNode>>,
}

impl CstToAstPipeline {
    /// Create new pipeline integrated with lapce-tree-sitter
    pub fn new() -> Self {
        let mut transformers = HashMap::new();
        
        // Register language-specific transformers
        transformers.insert("rust".to_string(), Box::new(RustTransformer) as Box<dyn LanguageTransformer>);
        transformers.insert("javascript".to_string(), Box::new(JavaScriptTransformer) as Box<dyn LanguageTransformer>);
        transformers.insert("typescript".to_string(), Box::new(TypeScriptTransformer) as Box<dyn LanguageTransformer>);
        transformers.insert("python".to_string(), Box::new(PythonTransformer) as Box<dyn LanguageTransformer>);
        transformers.insert("go".to_string(), Box::new(GoTransformer) as Box<dyn LanguageTransformer>);
        transformers.insert("java".to_string(), Box::new(JavaTransformer) as Box<dyn LanguageTransformer>);
        
        Self {
            transformers,
            ast_cache: Arc::new(dashmap::DashMap::new()),
        }
    }
    
    /// Process file through complete CST -> AST pipeline
    pub async fn process_file(&self, path: &Path) -> Result<PipelineOutput> {
        let start = std::time::Instant::now();
        
        // Read source code
        let source = std::fs::read_to_string(path)
            .map_err(|e| Error::Runtime { 
                message: format!("Failed to read file: {}", e) 
            })?;
        
        // Detect language
        let language = self.detect_language(path)?;
        
        // Get or create parser from lapce-tree-sitter integration
        let parser = self.get_or_create_parser(&language)?;
        
        // Parse to CST (tree-sitter Tree)
        let tree = self.parse_to_cst(parser, &source)?;
        let parse_time = start.elapsed().as_secs_f64() * 1000.0;
        
        // Convert tree-sitter Tree to our CstNode
        let cst = self.tree_to_cst_node(tree.root_node(), &source)?;
        
        let transform_start = std::time::Instant::now();
        
        // Transform CST to AST using language-specific transformer
        let ast = self.transform_cst_to_ast(&cst, &language, path)?;
        
        let transform_time = transform_start.elapsed().as_secs_f64() * 1000.0;
        
        // Cache the AST
        self.ast_cache.insert(path.to_path_buf(), ast.clone());
        
        Ok(PipelineOutput {
            cst,
            ast,
            source_file: path.to_path_buf(),
            language,
            parse_time_ms: parse_time,
            transform_time_ms: transform_time,
        })
    }
    
    /// Convert tree-sitter Node to CstNode
    fn tree_to_cst_node(&self, node: Node, source: &str) -> Result<CstNode> {
        let text = node.utf8_text(source.as_bytes())
            .map_err(|e| Error::Runtime { 
                message: format!("Failed to get node text: {}", e) 
            })?;
            
        let mut children = Vec::new();
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            children.push(self.tree_to_cst_node(child, source)?);
        }
        
        Ok(CstNode {
            kind: node.kind().to_string(),
            text: text.to_string(),
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_position: (node.start_position().row, node.start_position().column),
            end_position: (node.end_position().row, node.end_position().column),
            is_named: node.is_named(),
            is_missing: node.is_missing(),
            is_extra: node.is_extra(),
            field_name: None,  // Would need parent context
            children,
        })
    }
    
    /// Transform CST to AST using language-specific rules
    fn transform_cst_to_ast(&self, cst: &CstNode, language: &str, path: &Path) -> Result<AstNode> {
        let transformer = self.transformers.get(language)
            .ok_or_else(|| Error::Runtime {
                message: format!("No transformer for language: {}", language)
            })?;
            
        transformer.transform(cst, path)
    }
    
    /// Get or create parser for language
    fn get_or_create_parser(&self, language: &str) -> Result<Parser> {
        // Parser can't be cloned, so we create a new one each time
        let mut parser = Parser::new();
        let lang = self.get_language(language)?;
        parser.set_language(lang)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to set language: {}", e)
            })?;
            
        Ok(parser)
    }
    
    /// Parse source code to CST
    fn parse_to_cst(&self, mut parser: Parser, source: &str) -> Result<Tree> {
        parser.parse(source, None)
            .ok_or_else(|| Error::Runtime {
                message: "Failed to parse source code".to_string()
            })
    }
    
    /// Get tree-sitter language
    fn get_language(&self, name: &str) -> Result<Language> {
        match name {
            "rust" => Ok(unsafe { tree_sitter_rust::language() }),
            "javascript" => Ok(unsafe { tree_sitter_javascript::language() }),
            "typescript" => Ok(unsafe { tree_sitter_typescript::language_typescript() }),
            "python" => Ok(unsafe { tree_sitter_python::language() }),
            "go" => Ok(unsafe { tree_sitter_go::language() }),
            "java" => Ok(unsafe { tree_sitter_java::language() }),
            _ => Err(Error::Runtime {
                message: format!("Unsupported language: {}", name)
            })
        }
    }
    
    /// Detect language from file extension
    fn detect_language(&self, path: &Path) -> Result<String> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| Error::Runtime {
                message: "No file extension".to_string()
            })?;
            
        Ok(match ext {
            "rs" => "rust",
            "js" | "jsx" => "javascript",
            "ts" | "tsx" => "typescript",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            _ => return Err(Error::Runtime {
                message: format!("Unknown language for extension: {}", ext)
            })
        }.to_string())
    }
    
    /// Query both CST and AST for deep analysis
    pub fn query_both(&self, path: &Path, query: &str) -> Result<QueryResult> {
        // This provides both CST and AST views for agents
        if let Some(ast) = self.ast_cache.get(path) {
            Ok(QueryResult {
                ast_matches: self.query_ast(&ast, query)?,
                cst_matches: Vec::new(),  // Would query CST cache if available
            })
        } else {
            Err(Error::Runtime {
                message: "File not processed yet".to_string()
            })
        }
    }
    
    /// Query AST nodes
    fn query_ast(&self, ast: &AstNode, query: &str) -> Result<Vec<AstNode>> {
        let mut results = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(ast.clone());
        
        while let Some(node) = queue.pop_front() {
            // Simple query matching - can be extended
            if format!("{:?}", node.node_type).contains(query) ||
               node.identifier.as_ref().map_or(false, |id| id.contains(query)) {
                results.push(node.clone());
            }
            
            for child in &node.children {
                queue.push_back(child.clone());
            }
        }
        
        Ok(results)
    }
}

#[derive(Debug)]
pub struct QueryResult {
    pub ast_matches: Vec<AstNode>,
    pub cst_matches: Vec<CstNode>,
}

/// Trait for language-specific CST to AST transformation
trait LanguageTransformer: Send + Sync {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode>;
}

/// Rust-specific transformer
struct RustTransformer;

impl LanguageTransformer for RustTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        transform_rust_cst(cst, path, 0)
    }
}

fn transform_rust_cst(cst: &CstNode, path: &Path, scope_depth: usize) -> Result<AstNode> {
    let node_type = match cst.kind.as_str() {
        "source_file" => AstNodeType::Program,
        "function_item" => AstNodeType::FunctionDeclaration,
        "struct_item" => AstNodeType::StructDeclaration,
        "impl_item" => AstNodeType::ClassDeclaration,  // Treat impl as class-like
        "trait_item" => AstNodeType::TraitDeclaration,
        "enum_item" => AstNodeType::EnumDeclaration,
        "if_expression" => AstNodeType::IfStatement,
        "while_expression" => AstNodeType::WhileLoop,
        "for_expression" => AstNodeType::ForLoop,
        "let_declaration" => AstNodeType::VariableDeclaration,
        "use_declaration" => AstNodeType::ImportStatement,
        _ => AstNodeType::Unknown,
    };
    
    // Extract identifier if present
    let identifier = extract_identifier(cst);
    
    // Process children
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_rust_cst(child, path, scope_depth + 1)?);
        }
    }
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value: if cst.children.is_empty() { Some(cst.text.clone()) } else { None },
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "rust".to_string(),
            complexity: calculate_complexity(cst),
        },
        children: ast_children,
        semantic_info: Some(SemanticInfo {
            scope_depth,
            symbol_table: HashMap::new(),
            type_info: None,
            data_flow: Vec::new(),
            control_flow: Vec::new(),
        }),
    })
}

/// JavaScript-specific transformer
struct JavaScriptTransformer;

impl LanguageTransformer for JavaScriptTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        transform_js_cst(cst, path, 0)
    }
}

fn transform_js_cst(cst: &CstNode, path: &Path, scope_depth: usize) -> Result<AstNode> {
    let node_type = match cst.kind.as_str() {
        "program" => AstNodeType::Program,
        "function_declaration" => AstNodeType::FunctionDeclaration,
        "class_declaration" => AstNodeType::ClassDeclaration,
        "if_statement" => AstNodeType::IfStatement,
        "while_statement" => AstNodeType::WhileLoop,
        "for_statement" => AstNodeType::ForLoop,
        "variable_declaration" => AstNodeType::VariableDeclaration,
        "import_statement" => AstNodeType::ImportStatement,
        "export_statement" => AstNodeType::ExportStatement,
        _ => AstNodeType::Unknown,
    };
    
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_js_cst(child, path, scope_depth + 1)?);
        }
    }
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value: if cst.children.is_empty() { Some(cst.text.clone()) } else { None },
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "javascript".to_string(),
            complexity: calculate_complexity(cst),
        },
        children: ast_children,
        semantic_info: Some(SemanticInfo {
            scope_depth,
            symbol_table: HashMap::new(),
            type_info: None,
            data_flow: Vec::new(),
            control_flow: Vec::new(),
        }),
    })
}

/// TypeScript transformer
struct TypeScriptTransformer;

impl LanguageTransformer for TypeScriptTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        // Similar to JavaScript but with type annotations
        transform_js_cst(cst, path, 0)
    }
}

/// Python transformer
struct PythonTransformer;

impl LanguageTransformer for PythonTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        transform_python_cst(cst, path, 0)
    }
}

fn transform_python_cst(cst: &CstNode, path: &Path, scope_depth: usize) -> Result<AstNode> {
    let node_type = match cst.kind.as_str() {
        "module" => AstNodeType::Module,
        "function_definition" => AstNodeType::FunctionDeclaration,
        "class_definition" => AstNodeType::ClassDeclaration,
        "if_statement" => AstNodeType::IfStatement,
        "while_statement" => AstNodeType::WhileLoop,
        "for_statement" => AstNodeType::ForLoop,
        "import_statement" | "import_from_statement" => AstNodeType::ImportStatement,
        _ => AstNodeType::Unknown,
    };
    
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_python_cst(child, path, scope_depth + 1)?);
        }
    }
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value: if cst.children.is_empty() { Some(cst.text.clone()) } else { None },
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "python".to_string(),
            complexity: calculate_complexity(cst),
        },
        children: ast_children,
        semantic_info: Some(SemanticInfo {
            scope_depth,
            symbol_table: HashMap::new(),
            type_info: None,
            data_flow: Vec::new(),
            control_flow: Vec::new(),
        }),
    })
}

/// Go transformer
struct GoTransformer;

impl LanguageTransformer for GoTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        transform_go_cst(cst, path, 0)
    }
}

fn transform_go_cst(cst: &CstNode, path: &Path, scope_depth: usize) -> Result<AstNode> {
    let node_type = match cst.kind.as_str() {
        "source_file" => AstNodeType::Program,
        "package_clause" => AstNodeType::Package,
        "function_declaration" => AstNodeType::FunctionDeclaration,
        "type_declaration" => AstNodeType::TypeAlias,
        "if_statement" => AstNodeType::IfStatement,
        "for_statement" => AstNodeType::ForLoop,
        "import_declaration" => AstNodeType::ImportStatement,
        _ => AstNodeType::Unknown,
    };
    
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_go_cst(child, path, scope_depth + 1)?);
        }
    }
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value: if cst.children.is_empty() { Some(cst.text.clone()) } else { None },
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "go".to_string(),
            complexity: calculate_complexity(cst),
        },
        children: ast_children,
        semantic_info: Some(SemanticInfo {
            scope_depth,
            symbol_table: HashMap::new(),
            type_info: None,
            data_flow: Vec::new(),
            control_flow: Vec::new(),
        }),
    })
}

/// Java transformer
struct JavaTransformer;

impl LanguageTransformer for JavaTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        transform_java_cst(cst, path, 0)
    }
}

fn transform_java_cst(cst: &CstNode, path: &Path, scope_depth: usize) -> Result<AstNode> {
    let node_type = match cst.kind.as_str() {
        "program" => AstNodeType::Program,
        "package_declaration" => AstNodeType::Package,
        "class_declaration" => AstNodeType::ClassDeclaration,
        "interface_declaration" => AstNodeType::InterfaceDeclaration,
        "method_declaration" => AstNodeType::FunctionDeclaration,
        "if_statement" => AstNodeType::IfStatement,
        "while_statement" => AstNodeType::WhileLoop,
        "for_statement" => AstNodeType::ForLoop,
        "import_declaration" => AstNodeType::ImportStatement,
        _ => AstNodeType::Unknown,
    };
    
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_java_cst(child, path, scope_depth + 1)?);
        }
    }
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value: if cst.children.is_empty() { Some(cst.text.clone()) } else { None },
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "java".to_string(),
            complexity: calculate_complexity(cst),
        },
        children: ast_children,
        semantic_info: Some(SemanticInfo {
            scope_depth,
            symbol_table: HashMap::new(),
            type_info: None,
            data_flow: Vec::new(),
            control_flow: Vec::new(),
        }),
    })
}

/// Helper functions
fn extract_identifier(cst: &CstNode) -> Option<String> {
    // Look for identifier nodes in children
    for child in &cst.children {
        if child.kind == "identifier" || child.kind == "type_identifier" {
            return Some(child.text.clone());
        }
    }
    None
}

fn calculate_complexity(cst: &CstNode) -> usize {
    let mut complexity = 1;
    
    // Count decision points
    match cst.kind.as_str() {
        "if_statement" | "if_expression" => complexity += 1,
        "while_statement" | "while_expression" => complexity += 1,
        "for_statement" | "for_expression" => complexity += 1,
        "switch_statement" | "match_expression" => complexity += 1,
        _ => {}
    }
    
    // Add complexity from children
    for child in &cst.children {
        complexity += calculate_complexity(child);
    }
    
    complexity
}
