// TypeScript transformer - similar to JavaScript with type annotations
use crate::processors::cst_to_ast_pipeline::{AstNode, AstNodeType};
use crate::processors::language_transformers::LanguageTransformer;
use crate::error::Result;

pub struct TypeScriptTransformer;

impl LanguageTransformer for TypeScriptTransformer {
    fn transform(&self, node: &tree_sitter::Node, source: &str) -> Result<AstNode> {
        // TypeScript uses same structure as JavaScript
        let js_transformer = super::javascript_transformer::JavaScriptTransformer;
        js_transformer.transform(node, source)
    }
    
    fn language_name(&self) -> &'static str {
        "typescript"
    }
}
