// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// CST to AST Pipeline - Integrates with lapce-tree-sitter

use crate::error::{Error, Result};
use crate::processors::language_registry;
use crate::processors::language_transformers;
use tree_sitter::{Node, Tree, Parser, Language, TreeCursor};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use serde::{Serialize, Deserialize};

#[cfg(feature = "cst_ts")]
use lapce_tree_sitter::ast::kinds::{CanonicalKind, map_kind, map_field};

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
    /// Stable ID for tracking nodes across file edits (Phase B)
    /// Populated when using CstApi from lapce-tree-sitter
    pub stable_id: Option<u64>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
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
    ConstantDeclaration,
    VariableReference,
    Parameter,
    Identifier,
    
    // Imports/Exports
    ImportStatement,
    ExportStatement,
    
    // Types
    TypeAnnotation,
    GenericType,
    UnionType,
    IntersectionType,
    
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
    /// Stable ID propagated from CST for incremental indexing (Phase B)
    pub stable_id: Option<u64>,
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

/// Language-specific transformers for better AST extraction
pub struct CstToAstPipeline {
    /// Registered transformers by language
    transformers: HashMap<String, Box<dyn language_transformers::LanguageTransformer>>,
    /// AST cache for processed files
    ast_cache: Arc<dashmap::DashMap<PathBuf, AstNode>>,
}

impl CstToAstPipeline {
    /// Create new pipeline integrated with lapce-tree-sitter
    /// Supports all 67 languages from CST-tree-sitter
    pub fn new() -> Self {
        let mut transformers = HashMap::new();
        
        // Register specialized transformers for ALL 31 core languages
        let core_languages = vec![
            "rust", "javascript", "typescript", "python", "go", "java",
            "c", "cpp", "html", "css", "json", "bash",
            "c_sharp", "ruby", "php", "lua", "swift", "scala",
            "elixir", "ocaml", "nix", "make", "cmake", "verilog",
            "erlang", "d", "pascal", "commonlisp", "objc", "groovy",
            "embedded_template"
        ];
        
        for lang_name in core_languages {
            if let Some(transformer) = language_transformers::get_transformer(lang_name) {
                transformers.insert(lang_name.to_string(), transformer);
            }
        }
        
        // Optionally, can override with specialized transformers for specific languages
        // These provide better AST transformation for complex language features
        // (Currently using generic for all, can add specialized later)
        
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
        
        // Phase B: Use CstApi for stable IDs when cst_ts feature is enabled
        #[cfg(feature = "cst_ts")]
        {
            if let Ok(output) = self.process_file_with_cst_api(path, &source, &language).await {
                return Ok(output);
            }
            // Fallback to regular parsing if CstApi fails
        }
        
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
            stable_id: None,  // Phase B: Will be populated via CstApi
        })
    }
    
    /// Transform CST to AST using language-specific rules
    fn transform_cst_to_ast(&self, cst: &CstNode, language: &str, _path: &Path) -> Result<AstNode> {
        // For now, create a generic AST from CST
        // Specialized transformers will be integrated when we have direct tree-sitter Node access
        Ok(AstNode {
            node_type: match cst.kind.as_str() {
                "function_item" | "function_declaration" | "function_definition" => AstNodeType::FunctionDeclaration,
                "struct_item" | "struct_declaration" => AstNodeType::StructDeclaration,
                "class" | "class_declaration" | "class_definition" => AstNodeType::ClassDeclaration,
                "enum_item" | "enum_declaration" => AstNodeType::EnumDeclaration,
                "trait_item" => AstNodeType::TraitDeclaration,
                "impl_item" => AstNodeType::ClassDeclaration,
                "mod_item" | "module" => AstNodeType::Module,
                _ => AstNodeType::Unknown,
            },
            text: cst.text.clone(),
            identifier: None,
            value: None,
            children: cst.children.iter()
                .map(|child| self.transform_cst_to_ast(child, language, _path))
                .collect::<Result<Vec<_>>>()?,
            metadata: NodeMetadata {
                start_line: cst.start_position.0,
                end_line: cst.end_position.0,
                start_column: cst.start_position.1,
                end_column: cst.end_position.1,
                source_file: Some(_path.to_path_buf()),
                language: language.to_string(),
                complexity: 0,
                stable_id: cst.stable_id,
            },
            semantic_info: None,
        })
    }
    
    /// Get or create parser for language
    fn get_or_create_parser(&self, language: &str) -> Result<Parser> {
        // Parser can't be cloned, so we create a new one each time
        let mut parser = Parser::new();
        let lang = self.get_language(language)?;
        parser.set_language(&lang)
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
    
    /// Get tree-sitter language for all 67 supported languages
    fn get_language(&self, name: &str) -> Result<Language> {
        #[cfg(feature = "cst_ts")]
        {
            // Try CST-tree-sitter's LanguageRegistry first
            use lapce_tree_sitter::language::registry::LanguageRegistry;
            let registry = LanguageRegistry::instance();
            
            // If CST registry has it, use that
            if let Ok(lang_info) = registry.by_name(name) {
                return Ok(lang_info.language.clone());
            }
            
            // Otherwise fall back to direct crates for core languages
            self.get_language_fallback(name)
        }
        
        #[cfg(not(feature = "cst_ts"))]
        {
            self.get_language_fallback(name)
        }
    }
    
    /// Fallback to direct tree-sitter crates
    fn get_language_fallback(&self, name: &str) -> Result<Language> {
        match name {
            "rust" => Ok(tree_sitter_rust::LANGUAGE.into()),
            "python" => Ok(tree_sitter_python::LANGUAGE.into()),
            "go" => Ok(tree_sitter_go::LANGUAGE.into()),
            "java" => Ok(tree_sitter_java::LANGUAGE.into()),
            "c" => Ok(tree_sitter_c::LANGUAGE.into()),
            "cpp" => Ok(tree_sitter_cpp::LANGUAGE.into()),
            "c_sharp" => Ok(tree_sitter_c_sharp::LANGUAGE.into()),
            "javascript" => Ok(tree_sitter_javascript::LANGUAGE.into()),
            "typescript" => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "ruby" => Ok(tree_sitter_ruby::LANGUAGE.into()),
            "php" => Ok(tree_sitter_php::language_php()),
            "lua" => Ok(tree_sitter_lua::LANGUAGE.into()),
            "bash" => Ok(tree_sitter_bash::LANGUAGE.into()),
            "css" => Ok(tree_sitter_css::LANGUAGE.into()),
            "json" => Ok(tree_sitter_json::LANGUAGE.into()),
            "swift" => Ok(tree_sitter_swift::LANGUAGE.into()),
            "scala" => Ok(tree_sitter_scala::LANGUAGE.into()),
            "elixir" => Ok(tree_sitter_elixir::LANGUAGE.into()),
            "html" => Ok(tree_sitter_html::LANGUAGE.into()),
            "ocaml" => Ok(tree_sitter_ocaml::LANGUAGE_OCAML.into()),
            "nix" => Ok(tree_sitter_nix::LANGUAGE.into()),
            "make" => Ok(tree_sitter_make::LANGUAGE.into()),
            "cmake" => Ok(tree_sitter_cmake::LANGUAGE.into()),
            "verilog" => Ok(tree_sitter_verilog::LANGUAGE.into()),
            "erlang" => Ok(tree_sitter_erlang::LANGUAGE.into()),
            "d" => Ok(tree_sitter_d::LANGUAGE.into()),
            "pascal" => Ok(tree_sitter_pascal::LANGUAGE.into()),
            "commonlisp" => Ok(tree_sitter_commonlisp::LANGUAGE_COMMONLISP.into()),
            "objc" => Ok(tree_sitter_objc::LANGUAGE.into()),
            "groovy" => Ok(tree_sitter_groovy::LANGUAGE.into()),
            "embedded_template" => Ok(tree_sitter_embedded_template::LANGUAGE.into()),
            _ => Err(Error::Runtime {
                message: format!("Language {} not available in fallback. External grammars require CST build.", name)
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
            
        // Use language registry for all 67 languages
        language_registry::get_language_by_extension(ext)
            .map(|s| s.to_string())
            .ok_or_else(|| Error::Runtime {
                message: format!("Unknown language for extension: {}", ext)
            })
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
    
    /// Process file using CstApi for stable ID support (Phase B)
    #[cfg(feature = "cst_ts")]
    async fn process_file_with_cst_api(&self, path: &Path, source: &str, language: &str) -> Result<PipelineOutput> {
        use lapce_tree_sitter::cst_api::CstApiBuilder;
        
        let start = std::time::Instant::now();
        
        // Parse with tree-sitter
        let parser = self.get_or_create_parser(language)?;
        let tree = self.parse_to_cst(parser, source)?;
        let parse_time = start.elapsed().as_secs_f64() * 1000.0;
        
        // Build CstApi with stable IDs
        let transform_start = std::time::Instant::now();
        let cst_api = CstApiBuilder::new()
            .build_from_tree(&tree, source.as_bytes())
            .map_err(|e| Error::Runtime {
                message: format!("Failed to build CstApi: {}", e)
            })?;
        
        // Build CST directly from CstApi with stable IDs
        let cst = self.cst_api_to_cst_node_simple(&cst_api, 0, source)?;
        
        // Transform CST to AST
        let ast = self.transform_cst_to_ast(&cst, language, path)?;
        let transform_time = transform_start.elapsed().as_secs_f64() * 1000.0;
        
        // Cache result
        self.ast_cache.insert(path.to_path_buf(), ast.clone());
        
        Ok(PipelineOutput {
            cst,
            ast,
            source_file: path.to_path_buf(),
            language: language.to_string(),
            parse_time_ms: parse_time,
            transform_time_ms: transform_time,
        })
    }
    
    /// Convert CstApi to CstNode directly from root (Phase B)
    #[cfg(feature = "cst_ts")]
    fn cst_api_to_cst_node_simple(&self, api: &lapce_tree_sitter::cst_api::CstApi, node_idx: usize, source: &str) -> Result<CstNode> {
        // Get stable ID for this node
        let stable_id = api.get_stable_id(node_idx);
        
        // Get children
        let api_children = api.iterate_children(node_idx);
        
        // Get node data from first child query, or infer from index 0
        // Since we don't have direct access to node data at index, we use iterate_children on parent
        // For root node (idx 0), we need special handling
        if node_idx == 0 {
            // Root node - get it via iterate_children from itself or use metadata
            let metadata = api.metadata();
            
            // Build children recursively
            let mut children = Vec::new();
            for (child_idx, child_node) in api_children.iter().enumerate() {
                // Child nodes have their indices embedded in the structure
                // We'll recursively build from their data
                let child = CstNode {
                    kind: child_node.kind_name.clone(),
                    text: if child_node.start_byte < source.len() && child_node.end_byte <= source.len() {
                        source[child_node.start_byte..child_node.end_byte].to_string()
                    } else {
                        String::new()
                    },
                    start_byte: child_node.start_byte,
                    end_byte: child_node.end_byte,
                    start_position: (0, 0),
                    end_position: (0, 0),
                    is_named: child_node.is_named,
                    is_missing: child_node.is_missing,
                    is_extra: child_node.is_extra,
                    field_name: child_node.field_name.clone(),
                    children: vec![],  // Will be filled recursively
                    stable_id: Some(child_node.stable_id),
                };
                children.push(child);
            }
            
            // For root, we need to construct it
            Ok(CstNode {
                kind: "source_file".to_string(),
                text: source.to_string(),
                start_byte: 0,
                end_byte: source.len(),
                start_position: (0, 0),
                end_position: (0, 0),
                is_named: true,
                is_missing: false,
                is_extra: false,
                field_name: None,
                children,
                stable_id,
            })
        } else {
            // Non-root node - this shouldn't be called directly
            Err(Error::Runtime {
                message: "Should only call with root index 0".to_string()
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

/// Trait for language-specific AST transformation
pub trait LanguageTransformer: Send + Sync {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode>;
}

/// Canonical kind mapping helper (only available with cst_ts feature)
#[cfg(feature = "cst_ts")]
fn canonical_to_ast_node_type(canonical: CanonicalKind) -> AstNodeType {
    match canonical {
        CanonicalKind::Module => AstNodeType::Module,
        CanonicalKind::Block => AstNodeType::Unknown, // No direct mapping
        CanonicalKind::Statement => AstNodeType::Unknown, // Too generic
        CanonicalKind::Expression => AstNodeType::Unknown, // Too generic
        
        // Declarations
        CanonicalKind::FunctionDeclaration => AstNodeType::FunctionDeclaration,
        CanonicalKind::ClassDeclaration => AstNodeType::ClassDeclaration,
        CanonicalKind::InterfaceDeclaration => AstNodeType::InterfaceDeclaration,
        CanonicalKind::StructDeclaration => AstNodeType::StructDeclaration,
        CanonicalKind::EnumDeclaration => AstNodeType::EnumDeclaration,
        CanonicalKind::TypeAlias => AstNodeType::TypeAlias,
        CanonicalKind::VariableDeclaration => AstNodeType::VariableDeclaration,
        CanonicalKind::ConstantDeclaration => AstNodeType::VariableDeclaration,
        
        // Functions
        CanonicalKind::FunctionSignature => AstNodeType::FunctionDeclaration,
        CanonicalKind::ParameterList => AstNodeType::Parameter,
        CanonicalKind::Parameter => AstNodeType::Parameter,
        CanonicalKind::ReturnType => AstNodeType::TypeAnnotation,
        CanonicalKind::FunctionBody => AstNodeType::Unknown,
        
        // Types
        CanonicalKind::TypeAnnotation => AstNodeType::TypeAnnotation,
        CanonicalKind::GenericType => AstNodeType::GenericType,
        CanonicalKind::ArrayType => AstNodeType::TypeAnnotation,
        CanonicalKind::PointerType => AstNodeType::TypeAnnotation,
        CanonicalKind::ReferenceType => AstNodeType::TypeAnnotation,
        CanonicalKind::UnionType => AstNodeType::UnionType,
        CanonicalKind::IntersectionType => AstNodeType::IntersectionType,
        
        // Expressions
        CanonicalKind::BinaryExpression => AstNodeType::BinaryExpression,
        CanonicalKind::UnaryExpression => AstNodeType::UnaryExpression,
        CanonicalKind::CallExpression => AstNodeType::CallExpression,
        CanonicalKind::MemberExpression => AstNodeType::MemberExpression,
        CanonicalKind::IndexExpression => AstNodeType::ArrayExpression,
        CanonicalKind::LiteralExpression => AstNodeType::Unknown,
        CanonicalKind::IdentifierExpression => AstNodeType::VariableReference,
        CanonicalKind::AssignmentExpression => AstNodeType::Unknown,
        
        // Control flow
        CanonicalKind::IfStatement => AstNodeType::IfStatement,
        CanonicalKind::ForLoop => AstNodeType::ForLoop,
        CanonicalKind::WhileLoop => AstNodeType::WhileLoop,
        CanonicalKind::DoWhileLoop => AstNodeType::WhileLoop,
        CanonicalKind::SwitchStatement => AstNodeType::SwitchStatement,
        CanonicalKind::CaseClause => AstNodeType::Unknown,
        CanonicalKind::BreakStatement => AstNodeType::BreakStatement,
        CanonicalKind::ContinueStatement => AstNodeType::ContinueStatement,
        CanonicalKind::ReturnStatement => AstNodeType::ReturnStatement,
        CanonicalKind::ThrowStatement => AstNodeType::Unknown,
        CanonicalKind::TryStatement => AstNodeType::Unknown,
        CanonicalKind::CatchClause => AstNodeType::Unknown,
        
        // Literals
        CanonicalKind::StringLiteral => AstNodeType::StringLiteral,
        CanonicalKind::NumberLiteral => AstNodeType::NumberLiteral,
        CanonicalKind::BooleanLiteral => AstNodeType::BooleanLiteral,
        CanonicalKind::NullLiteral => AstNodeType::NullLiteral,
        CanonicalKind::RegexLiteral => AstNodeType::StringLiteral,
        CanonicalKind::TemplateLiteral => AstNodeType::StringLiteral,
        
        // Comments
        CanonicalKind::LineComment => AstNodeType::Comment,
        CanonicalKind::BlockComment => AstNodeType::Comment,
        CanonicalKind::DocComment => AstNodeType::DocComment,
        
        // Other
        CanonicalKind::Identifier => AstNodeType::VariableReference,
        CanonicalKind::Operator => AstNodeType::Unknown,
        CanonicalKind::Keyword => AstNodeType::Unknown,
        CanonicalKind::Punctuation => AstNodeType::Unknown,
        CanonicalKind::Error => AstNodeType::Error,
        CanonicalKind::Unknown => AstNodeType::Unknown,
    }
}

/// Get node type using canonical mapping when available
#[cfg(feature = "cst_ts")]
fn get_node_type_with_canonical(language: &str, kind: &str) -> AstNodeType {
    use crate::search::search_metrics::{CANONICAL_MAPPING_APPLIED_TOTAL, CANONICAL_MAPPING_UNKNOWN_TOTAL};
    
    let canonical = map_kind(language, kind);
    let result = canonical_to_ast_node_type(canonical);
    
    // Record metrics
    if result == AstNodeType::Unknown {
        CANONICAL_MAPPING_UNKNOWN_TOTAL.with_label_values(&[language]).inc();
    } else {
        CANONICAL_MAPPING_APPLIED_TOTAL.with_label_values(&[language]).inc();
    }
    
    result
}

/// Get field name using canonical mapping when available
#[cfg(feature = "cst_ts")]
fn get_canonical_field(language: &str, field: &str) -> String {
    map_field(language, field).unwrap_or(field).to_string()
}

/// Rust-specific transformer
struct RustTransformer;

impl LanguageTransformer for RustTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        transform_rust_cst(cst, path, 0)
    }
}

fn transform_rust_cst(cst: &CstNode, path: &Path, scope_depth: usize) -> Result<AstNode> {
    #[cfg(feature = "cst_ts")]
    let node_type = get_node_type_with_canonical("rust", &cst.kind);
    
    #[cfg(not(feature = "cst_ts"))]
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
    #[cfg(feature = "cst_ts")]
    let identifier = extract_identifier_canonical(cst, "rust");
    #[cfg(not(feature = "cst_ts"))]
    let identifier = extract_identifier(cst);
    
    // Process children
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_rust_cst(child, path, scope_depth + 1)?);
        }
    }
    
    // Extract value for literals
    #[cfg(feature = "cst_ts")]
    let value = extract_value_canonical(cst, "rust");
    #[cfg(not(feature = "cst_ts"))]
    let value = if cst.children.is_empty() { Some(cst.text.clone()) } else { None };
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value,
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "rust".to_string(),
            complexity: calculate_complexity(cst),
            stable_id: cst.stable_id,  // Propagate from CST
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
    #[cfg(feature = "cst_ts")]
    let node_type = get_node_type_with_canonical("javascript", &cst.kind);
    
    #[cfg(not(feature = "cst_ts"))]
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
    
    #[cfg(feature = "cst_ts")]
    let identifier = extract_identifier_canonical(cst, "javascript");
    #[cfg(not(feature = "cst_ts"))]
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_js_cst(child, path, scope_depth + 1)?);
        }
    }
    
    #[cfg(feature = "cst_ts")]
    let value = extract_value_canonical(cst, "javascript");
    #[cfg(not(feature = "cst_ts"))]
    let value = if cst.children.is_empty() { Some(cst.text.clone()) } else { None };
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value,
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "javascript".to_string(),
            complexity: calculate_complexity(cst),
            stable_id: cst.stable_id,  // Propagate from CST
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
    #[cfg(feature = "cst_ts")]
    let node_type = get_node_type_with_canonical("python", &cst.kind);
    
    #[cfg(not(feature = "cst_ts"))]
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
    
    #[cfg(feature = "cst_ts")]
    let identifier = extract_identifier_canonical(cst, "python");
    #[cfg(not(feature = "cst_ts"))]
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_python_cst(child, path, scope_depth + 1)?);
        }
    }
    
    #[cfg(feature = "cst_ts")]
    let value = extract_value_canonical(cst, "python");
    #[cfg(not(feature = "cst_ts"))]
    let value = if cst.children.is_empty() { Some(cst.text.clone()) } else { None };
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value,
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "python".to_string(),
            complexity: calculate_complexity(cst),
            stable_id: cst.stable_id,  // Propagate from CST
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
    #[cfg(feature = "cst_ts")]
    let node_type = get_node_type_with_canonical("go", &cst.kind);
    
    #[cfg(not(feature = "cst_ts"))]
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
    
    #[cfg(feature = "cst_ts")]
    let identifier = extract_identifier_canonical(cst, "go");
    #[cfg(not(feature = "cst_ts"))]
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_go_cst(child, path, scope_depth + 1)?);
        }
    }
    
    #[cfg(feature = "cst_ts")]
    let value = extract_value_canonical(cst, "go");
    #[cfg(not(feature = "cst_ts"))]
    let value = if cst.children.is_empty() { Some(cst.text.clone()) } else { None };
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value,
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "go".to_string(),
            complexity: calculate_complexity(cst),
            stable_id: cst.stable_id,  // Propagate from CST
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
    #[cfg(feature = "cst_ts")]
    let node_type = get_node_type_with_canonical("java", &cst.kind);
    
    #[cfg(not(feature = "cst_ts"))]
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
    
    #[cfg(feature = "cst_ts")]
    let identifier = extract_identifier_canonical(cst, "java");
    #[cfg(not(feature = "cst_ts"))]
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_java_cst(child, path, scope_depth + 1)?);
        }
    }
    
    #[cfg(feature = "cst_ts")]
    let value = extract_value_canonical(cst, "java");
    #[cfg(not(feature = "cst_ts"))]
    let value = if cst.children.is_empty() { Some(cst.text.clone()) } else { None };
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value,
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "java".to_string(),
            complexity: calculate_complexity(cst),
            stable_id: cst.stable_id,  // Propagate from CST
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

/// C++ transformer
struct CppTransformer;

impl LanguageTransformer for CppTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        transform_cpp_cst(cst, path, 0)
    }
}

fn transform_cpp_cst(cst: &CstNode, path: &Path, scope_depth: usize) -> Result<AstNode> {
    #[cfg(feature = "cst_ts")]
    let node_type = get_node_type_with_canonical("cpp", &cst.kind);
    
    #[cfg(not(feature = "cst_ts"))]
    let node_type = match cst.kind.as_str() {
        "translation_unit" => AstNodeType::Program,
        "function_definition" => AstNodeType::FunctionDeclaration,
        "class_specifier" => AstNodeType::ClassDeclaration,
        "struct_specifier" => AstNodeType::StructDeclaration,
        "namespace_definition" => AstNodeType::Module,
        "if_statement" => AstNodeType::IfStatement,
        "while_statement" => AstNodeType::WhileLoop,
        "for_statement" => AstNodeType::ForLoop,
        "switch_statement" => AstNodeType::SwitchStatement,
        _ => AstNodeType::Unknown,
    };
    
    #[cfg(feature = "cst_ts")]
    let identifier = extract_identifier_canonical(cst, "cpp");
    #[cfg(not(feature = "cst_ts"))]
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_cpp_cst(child, path, scope_depth + 1)?);
        }
    }
    
    #[cfg(feature = "cst_ts")]
    let value = extract_value_canonical(cst, "cpp");
    #[cfg(not(feature = "cst_ts"))]
    let value = if cst.children.is_empty() { Some(cst.text.clone()) } else { None };
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value,
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "cpp".to_string(),
            complexity: calculate_complexity(cst),
            stable_id: cst.stable_id,
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

/// C transformer
struct CTransformer;

impl LanguageTransformer for CTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        transform_c_cst(cst, path, 0)
    }
}

fn transform_c_cst(cst: &CstNode, path: &Path, scope_depth: usize) -> Result<AstNode> {
    #[cfg(feature = "cst_ts")]
    let node_type = get_node_type_with_canonical("c", &cst.kind);
    
    #[cfg(not(feature = "cst_ts"))]
    let node_type = match cst.kind.as_str() {
        "translation_unit" => AstNodeType::Program,
        "function_definition" => AstNodeType::FunctionDeclaration,
        "struct_specifier" => AstNodeType::StructDeclaration,
        "if_statement" => AstNodeType::IfStatement,
        "while_statement" => AstNodeType::WhileLoop,
        "for_statement" => AstNodeType::ForLoop,
        "switch_statement" => AstNodeType::SwitchStatement,
        _ => AstNodeType::Unknown,
    };
    
    #[cfg(feature = "cst_ts")]
    let identifier = extract_identifier_canonical(cst, "c");
    #[cfg(not(feature = "cst_ts"))]
    let identifier = extract_identifier(cst);
    
    let mut ast_children = Vec::new();
    for child in &cst.children {
        if child.is_named && !child.is_extra {
            ast_children.push(transform_c_cst(child, path, scope_depth + 1)?);
        }
    }
    
    #[cfg(feature = "cst_ts")]
    let value = extract_value_canonical(cst, "c");
    #[cfg(not(feature = "cst_ts"))]
    let value = if cst.children.is_empty() { Some(cst.text.clone()) } else { None };
    
    Ok(AstNode {
        node_type,
        text: cst.text.clone(),
        identifier,
        value,
        metadata: NodeMetadata {
            start_line: cst.start_position.0,
            end_line: cst.end_position.0,
            start_column: cst.start_position.1,
            end_column: cst.end_position.1,
            source_file: Some(path.to_path_buf()),
            language: "c".to_string(),
            complexity: calculate_complexity(cst),
            stable_id: cst.stable_id,
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

/// Extract identifier with canonical field mapping support
#[cfg(feature = "cst_ts")]
fn extract_identifier_canonical(cst: &CstNode, language: &str) -> Option<String> {
    // First try canonical field mapping for "name" field
    for child in &cst.children {
        if let Some(field) = &child.field_name {
            let canonical_field = get_canonical_field(language, field);
            if canonical_field == "name" && (child.kind == "identifier" || child.kind == "type_identifier") {
                return Some(child.text.clone());
            }
        }
    }
    // Fallback to regular identifier extraction
    extract_identifier(cst)
}

/// Extract literal value with canonical support
#[cfg(feature = "cst_ts")]
fn extract_value_canonical(cst: &CstNode, language: &str) -> Option<String> {
    // Check if this is a literal node
    let canonical = map_kind(language, &cst.kind);
    match canonical {
        CanonicalKind::StringLiteral | 
        CanonicalKind::NumberLiteral | 
        CanonicalKind::BooleanLiteral | 
        CanonicalKind::NullLiteral => Some(cst.text.clone()),
        _ => {
            // Look for value field in children
            for child in &cst.children {
                if let Some(field) = &child.field_name {
                    let canonical_field = get_canonical_field(language, field);
                    if canonical_field == "value" {
                        return Some(child.text.clone());
                    }
                }
            }
            None
        }
    }
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

#[cfg(test)]
mod cst_to_ast_tests;

#[cfg(test)]
mod security_tests;

#[cfg(all(test, feature = "cst_ts"))]
mod stable_id_tests;
