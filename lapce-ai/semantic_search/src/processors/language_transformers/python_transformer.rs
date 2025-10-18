// Python transformer matching Codex format
use crate::processors::cst_to_ast_pipeline::{AstNode, AstNodeType, NodeMetadata};
use crate::processors::language_transformers::LanguageTransformer;
use crate::error::Result;

pub struct PythonTransformer;

impl LanguageTransformer for PythonTransformer {
    fn transform(&self, node: &tree_sitter::Node, source: &str) -> Result<AstNode> {
        let node_type = match node.kind() {
            "function_definition" => AstNodeType::FunctionDeclaration,
            "class_definition" => AstNodeType::ClassDeclaration,
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
                language: "python".to_string(),
                complexity: 0,
                stable_id: None,
            },
            semantic_info: None,
        })
    }
    
    fn language_name(&self) -> &'static str {
        "python"
    }
}

fn extract_name(node: &tree_sitter::Node, source: &str) -> Result<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
        .ok_or_else(|| crate::error::Error::Runtime {
            message: "Failed to extract name".to_string()
        })
}

fn extract_identifier(node: &tree_sitter::Node, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
}
