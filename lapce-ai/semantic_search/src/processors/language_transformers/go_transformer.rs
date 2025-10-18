// Go transformer
use crate::processors::cst_to_ast_pipeline::{AstNode, AstNodeType, NodeMetadata};
use crate::processors::language_transformers::LanguageTransformer;
use crate::error::Result;

pub struct GoTransformer;

impl LanguageTransformer for GoTransformer {
    fn transform(&self, node: &tree_sitter::Node, source: &str) -> Result<AstNode> {
        let node_type = match node.kind() {
            "function_declaration" => AstNodeType::FunctionDeclaration,
            "type_declaration" => AstNodeType::TypeAlias,
            _ => AstNodeType::Unknown,
        };
        Ok(AstNode {
            node_type,
            text: node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
            identifier: None,
            value: None,
            children: vec![],
            metadata: NodeMetadata {
                start_line: node.start_position().row,
                end_line: node.end_position().row,
                start_column: node.start_position().column,
                end_column: node.end_position().column,
                source_file: None,
                language: "go".to_string(),
                complexity: 0,
                stable_id: None,
            },
            semantic_info: None,
        })
    }
    fn language_name(&self) -> &'static str { "go" }
}
