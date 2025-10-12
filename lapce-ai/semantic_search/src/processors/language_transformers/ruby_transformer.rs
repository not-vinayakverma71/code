// Ruby transformer matching Codex format
use crate::processors::cst_to_ast_pipeline::{AstNode, AstNodeType, NodeMetadata};
use crate::processors::language_transformers::LanguageTransformer;
use crate::error::Result;

pub struct RubyTransformer;

impl LanguageTransformer for RubyTransformer {
    fn transform(&self, node: &tree_sitter::Node, source: &str) -> Result<AstNode> {
        let node_type = match node.kind() {
            "method" | "singleton_method" => AstNodeType::FunctionDeclaration,
            "class" => AstNodeType::ClassDeclaration,
            "module" => AstNodeType::Module,
            _ => AstNodeType::Unknown,
        };
        
        Ok(AstNode {
            node_type,
            text: node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
            identifier: extract_identifier(node, source),
            value: None,
            children: vec![],
            metadata: NodeMetadata {
                start_line: node.start_position().row,
                end_line: node.end_position().row,
                start_column: node.start_position().column,
                end_column: node.end_position().column,
                source_file: None,
                language: "ruby".to_string(),
                complexity: 0,
                stable_id: None,
            },
            semantic_info: None,
        })
    }
    
    fn language_name(&self) -> &'static str { "ruby" }
}

fn extract_identifier(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for field in &["name", "constant", "identifier"] {
        if let Some(child) = node.child_by_field_name(field) {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                return Some(text.to_string());
            }
        }
    }
    None
}
