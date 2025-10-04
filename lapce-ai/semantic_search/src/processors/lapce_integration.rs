// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Integration with lapce-tree-sitter CST generator

use crate::error::{Error, Result};
use crate::processors::cst_to_ast_pipeline::{CstToAstPipeline, PipelineOutput, CstNode, AstNode};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use dashmap::DashMap;
use tree_sitter::{Parser, Tree, Node};

/// Bridge between lapce-tree-sitter CST and our AST pipeline
pub struct LapceTreeSitterBridge {
    /// Reference to lapce-tree-sitter's MegaParser
    mega_parser_path: PathBuf,
    
    /// Our CST to AST pipeline
    pipeline: Arc<CstToAstPipeline>,
    
    /// Shared CST cache with lapce-tree-sitter
    cst_cache: Arc<DashMap<PathBuf, Arc<Tree>>>,
    
    /// Bidirectional mapping for deep code intelligence
    cst_ast_map: Arc<DashMap<PathBuf, (CstNode, AstNode)>>,
}

impl LapceTreeSitterBridge {
    /// Create new bridge to lapce-tree-sitter
    pub fn new(lapce_tree_sitter_path: PathBuf) -> Self {
        Self {
            mega_parser_path: lapce_tree_sitter_path,
            pipeline: Arc::new(CstToAstPipeline::new()),
            cst_cache: Arc::new(DashMap::new()),
            cst_ast_map: Arc::new(DashMap::new()),
        }
    }
    
    /// Process file using lapce-tree-sitter CST then transform to AST
    pub async fn process_with_mega_parser(&self, path: &Path) -> Result<CodeIntelligenceData> {
        // Step 1: Get CST from lapce-tree-sitter MegaParser
        let cst = self.get_cst_from_mega_parser(path).await?;
        
        // Step 2: Transform CST to AST through our pipeline
        let pipeline_output = self.pipeline.process_file(path).await?;
        
        // Step 3: Store bidirectional mapping
        self.cst_ast_map.insert(
            path.to_path_buf(),
            (pipeline_output.cst.clone(), pipeline_output.ast.clone())
        );
        
        // Step 4: Extract deep code intelligence
        let intelligence = self.extract_intelligence(&pipeline_output)?;
        
        Ok(intelligence)
    }
    
    /// Get CST from lapce-tree-sitter's MegaParser
    async fn get_cst_from_mega_parser(&self, path: &Path) -> Result<Arc<Tree>> {
        // Check cache first
        if let Some(cached) = self.cst_cache.get(path) {
            return Ok(Arc::clone(&*cached));
        }
        
        // In real implementation, this would call into lapce-tree-sitter
        // For now, we'll parse directly
        let source = std::fs::read_to_string(path)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to read file: {}", e)
            })?;
        
        let lang = self.detect_language(path)?;
        let mut parser = Parser::new();
        
        let language = match lang.as_str() {
            "rust" => unsafe { tree_sitter_rust::language() },
            "javascript" => unsafe { tree_sitter_javascript::language() },
            "typescript" => unsafe { tree_sitter_typescript::language_typescript() },
            "python" => unsafe { tree_sitter_python::language() },
            "go" => unsafe { tree_sitter_go::language() },
            "java" => unsafe { tree_sitter_java::language() },
            _ => return Err(Error::Runtime {
                message: format!("Unsupported language: {}", lang)
            })
        };
        
        parser.set_language(language)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to set language: {}", e)
            })?;
            
        let tree = parser.parse(&source, None)
            .ok_or_else(|| Error::Runtime {
                message: "Failed to parse".to_string()
            })?;
        
        let tree_arc = Arc::new(tree);
        self.cst_cache.insert(path.to_path_buf(), tree_arc.clone());
        
        Ok(tree_arc)
    }
    
    /// Extract deep code intelligence from both CST and AST
    fn extract_intelligence(&self, output: &PipelineOutput) -> Result<CodeIntelligenceData> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();
        let mut symbols = Vec::new();
        
        // Walk AST to extract structured information
        self.walk_ast_for_intelligence(&output.ast, &mut functions, &mut classes, &mut imports, &mut symbols);
        
        // Extract additional CST details for completeness
        let cst_details = self.extract_cst_details(&output.cst);
        
        Ok(CodeIntelligenceData {
            source_file: output.source_file.clone(),
            language: output.language.clone(),
            functions,
            classes,
            imports,
            symbols,
            cst_details,
            ast_summary: self.summarize_ast(&output.ast),
            complexity_score: self.calculate_overall_complexity(&output.ast),
        })
    }
    
    /// Walk AST to extract intelligence
    fn walk_ast_for_intelligence(
        &self,
        node: &AstNode,
        functions: &mut Vec<FunctionInfo>,
        classes: &mut Vec<ClassInfo>,
        imports: &mut Vec<ImportInfo>,
        symbols: &mut Vec<SymbolInfo>,
    ) {
        use crate::processors::cst_to_ast_pipeline::AstNodeType;
        
        match node.node_type {
            AstNodeType::FunctionDeclaration => {
                functions.push(FunctionInfo {
                    name: node.identifier.clone().unwrap_or_default(),
                    start_line: node.metadata.start_line,
                    end_line: node.metadata.end_line,
                    parameters: self.extract_parameters(node),
                    return_type: self.extract_return_type(node),
                    complexity: node.metadata.complexity,
                });
            }
            AstNodeType::ClassDeclaration | AstNodeType::StructDeclaration => {
                classes.push(ClassInfo {
                    name: node.identifier.clone().unwrap_or_default(),
                    start_line: node.metadata.start_line,
                    end_line: node.metadata.end_line,
                    methods: self.extract_methods(node),
                    fields: self.extract_fields(node),
                });
            }
            AstNodeType::ImportStatement => {
                imports.push(ImportInfo {
                    module: node.value.clone().unwrap_or_default(),
                    line: node.metadata.start_line,
                });
            }
            _ => {}
        }
        
        // Add to symbol table
        if let Some(id) = &node.identifier {
            symbols.push(SymbolInfo {
                name: id.clone(),
                kind: format!("{:?}", node.node_type),
                line: node.metadata.start_line,
                scope_depth: node.semantic_info.scope_depth,
            });
        }
        
        // Recurse through children
        for child in &node.children {
            self.walk_ast_for_intelligence(child, functions, classes, imports, symbols);
        }
    }
    
    /// Extract function parameters from AST
    fn extract_parameters(&self, node: &AstNode) -> Vec<String> {
        let mut params = Vec::new();
        for child in &node.children {
            if matches!(child.node_type, crate::processors::cst_to_ast_pipeline::AstNodeType::Parameter) {
                if let Some(name) = &child.identifier {
                    params.push(name.clone());
                }
            }
        }
        params
    }
    
    /// Extract return type from AST
    fn extract_return_type(&self, node: &AstNode) -> Option<String> {
        for child in &node.children {
            if matches!(child.node_type, crate::processors::cst_to_ast_pipeline::AstNodeType::TypeAnnotation) {
                return child.value.clone();
            }
        }
        None
    }
    
    /// Extract methods from class/struct
    fn extract_methods(&self, node: &AstNode) -> Vec<String> {
        let mut methods = Vec::new();
        for child in &node.children {
            if matches!(child.node_type, crate::processors::cst_to_ast_pipeline::AstNodeType::FunctionDeclaration) {
                if let Some(name) = &child.identifier {
                    methods.push(name.clone());
                }
            }
        }
        methods
    }
    
    /// Extract fields from class/struct
    fn extract_fields(&self, node: &AstNode) -> Vec<String> {
        let mut fields = Vec::new();
        for child in &node.children {
            if matches!(child.node_type, crate::processors::cst_to_ast_pipeline::AstNodeType::VariableDeclaration) {
                if let Some(name) = &child.identifier {
                    fields.push(name.clone());
                }
            }
        }
        fields
    }
    
    /// Extract detailed CST information
    fn extract_cst_details(&self, cst: &CstNode) -> CstDetails {
        CstDetails {
            total_nodes: self.count_cst_nodes(cst),
            named_nodes: self.count_named_nodes(cst),
            syntax_errors: self.count_errors(cst),
            max_depth: self.calculate_max_depth(cst, 0),
        }
    }
    
    /// Count total CST nodes
    fn count_cst_nodes(&self, cst: &CstNode) -> usize {
        1 + cst.children.iter().map(|c| self.count_cst_nodes(c)).sum::<usize>()
    }
    
    /// Count named CST nodes
    fn count_named_nodes(&self, cst: &CstNode) -> usize {
        let count = if cst.is_named { 1 } else { 0 };
        count + cst.children.iter().map(|c| self.count_named_nodes(c)).sum::<usize>()
    }
    
    /// Count syntax errors in CST
    fn count_errors(&self, cst: &CstNode) -> usize {
        let count = if cst.is_missing || cst.kind == "ERROR" { 1 } else { 0 };
        count + cst.children.iter().map(|c| self.count_errors(c)).sum::<usize>()
    }
    
    /// Calculate max depth of CST
    fn calculate_max_depth(&self, cst: &CstNode, current: usize) -> usize {
        if cst.children.is_empty() {
            current
        } else {
            cst.children.iter()
                .map(|c| self.calculate_max_depth(c, current + 1))
                .max()
                .unwrap_or(current)
        }
    }
    
    /// Summarize AST structure
    fn summarize_ast(&self, ast: &AstNode) -> AstSummary {
        use crate::processors::cst_to_ast_pipeline::AstNodeType;
        
        let mut summary = AstSummary {
            total_nodes: 0,
            functions: 0,
            classes: 0,
            variables: 0,
            imports: 0,
            max_depth: 0,
        };
        
        self.count_ast_nodes(ast, &mut summary, 0);
        summary
    }
    
    /// Count AST node types
    fn count_ast_nodes(&self, node: &AstNode, summary: &mut AstSummary, depth: usize) {
        use crate::processors::cst_to_ast_pipeline::AstNodeType;
        
        summary.total_nodes += 1;
        if depth > summary.max_depth {
            summary.max_depth = depth;
        }
        
        match node.node_type {
            AstNodeType::FunctionDeclaration => summary.functions += 1,
            AstNodeType::ClassDeclaration | AstNodeType::StructDeclaration => summary.classes += 1,
            AstNodeType::VariableDeclaration => summary.variables += 1,
            AstNodeType::ImportStatement => summary.imports += 1,
            _ => {}
        }
        
        for child in &node.children {
            self.count_ast_nodes(child, summary, depth + 1);
        }
    }
    
    /// Calculate overall code complexity
    fn calculate_overall_complexity(&self, ast: &AstNode) -> usize {
        ast.metadata.complexity + 
        ast.children.iter().map(|c| self.calculate_overall_complexity(c)).sum::<usize>()
    }
    
    /// Detect language from file
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
                message: format!("Unknown extension: {}", ext)
            })
        }.to_string())
    }
    
    /// Get both CST and AST for a file (for agents)
    pub fn get_both_trees(&self, path: &Path) -> Option<(CstNode, AstNode)> {
        self.cst_ast_map.get(path).map(|entry| entry.clone())
    }
    
    /// Query across both CST and AST
    pub fn deep_query(&self, path: &Path, query: &str) -> Result<DeepQueryResult> {
        if let Some((cst, ast)) = self.get_both_trees(path) {
            Ok(DeepQueryResult {
                cst_matches: self.query_cst(&cst, query),
                ast_matches: self.query_ast(&ast, query),
                cross_references: self.find_cross_references(&cst, &ast, query),
            })
        } else {
            Err(Error::Runtime {
                message: "File not processed".to_string()
            })
        }
    }
    
    /// Query CST nodes
    fn query_cst(&self, cst: &CstNode, query: &str) -> Vec<CstNode> {
        let mut results = Vec::new();
        if cst.kind.contains(query) || cst.text.contains(query) {
            results.push(cst.clone());
        }
        for child in &cst.children {
            results.extend(self.query_cst(child, query));
        }
        results
    }
    
    /// Query AST nodes
    fn query_ast(&self, ast: &AstNode, query: &str) -> Vec<AstNode> {
        let mut results = Vec::new();
        if format!("{:?}", ast.node_type).contains(query) ||
           ast.identifier.as_ref().map_or(false, |id| id.contains(query)) {
            results.push(ast.clone());
        }
        for child in &ast.children {
            results.extend(self.query_ast(child, query));
        }
        results
    }
    
    /// Find cross-references between CST and AST
    fn find_cross_references(&self, cst: &CstNode, ast: &AstNode, query: &str) -> Vec<CrossReference> {
        let mut refs = Vec::new();
        
        // Find matching positions between CST and AST
        if cst.start_position == (ast.metadata.start_line, ast.metadata.start_column) {
            if cst.text.contains(query) || 
               ast.identifier.as_ref().map_or(false, |id| id.contains(query)) {
                refs.push(CrossReference {
                    cst_kind: cst.kind.clone(),
                    ast_type: format!("{:?}", ast.node_type),
                    position: cst.start_position,
                    matched_text: cst.text.clone(),
                });
            }
        }
        
        refs
    }
}

/// Deep code intelligence data structure
#[derive(Debug, Clone)]
pub struct CodeIntelligenceData {
    pub source_file: PathBuf,
    pub language: String,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<ImportInfo>,
    pub symbols: Vec<SymbolInfo>,
    pub cst_details: CstDetails,
    pub ast_summary: AstSummary,
    pub complexity_score: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub complexity: usize,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub methods: Vec<String>,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub module: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub scope_depth: usize,
}

#[derive(Debug, Clone)]
pub struct CstDetails {
    pub total_nodes: usize,
    pub named_nodes: usize,
    pub syntax_errors: usize,
    pub max_depth: usize,
}

#[derive(Debug, Clone)]
pub struct AstSummary {
    pub total_nodes: usize,
    pub functions: usize,
    pub classes: usize,
    pub variables: usize,
    pub imports: usize,
    pub max_depth: usize,
}

#[derive(Debug)]
pub struct DeepQueryResult {
    pub cst_matches: Vec<CstNode>,
    pub ast_matches: Vec<AstNode>,
    pub cross_references: Vec<CrossReference>,
}

#[derive(Debug)]
pub struct CrossReference {
    pub cst_kind: String,
    pub ast_type: String,
    pub position: (usize, usize),
    pub matched_text: String,
}
