// pascal transformer
use crate::processors::cst_to_ast_pipeline::{AstNode, AstNodeType, NodeMetadata};
use crate::processors::language_transformers::LanguageTransformer;
use crate::error::Result;

pub struct PascalTransformer;

impl LanguageTransformer for PascalTransformer {
    fn transform(&self, node: &tree_sitter::Node, source: &str) -> Result<AstNode> {
        Ok(AstNode {
            node_type: AstNodeType::Unknown,
            text: node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
            identifier: node.child_by_field_name("name")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string()),
            value: None,
            children: vec![],
            metadata: NodeMetadata {
                start_line: node.start_position().row,
                end_line: node.end_position().row,
                start_column: node.start_position().column,
                end_column: node.end_position().column,
                source_file: None,
                language: "pascal".to_string(),
                complexity: 0,
                stable_id: None,
            },
            semantic_info: None,
        })
    }
    fn language_name(&self) -> &'static str { "pascal" }
}
