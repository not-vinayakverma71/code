// Rust transformer matching Codex query format exactly
// Reference: /home/verma/lapce/Codex/src/services/tree-sitter/queries/rust.ts

use crate::processors::cst_to_ast_pipeline::{AstNode, AstNodeType, NodeMetadata};
use crate::processors::language_transformers::LanguageTransformer;
use crate::error::{Error, Result};
use tree_sitter::Node;

pub struct RustTransformer;

impl LanguageTransformer for RustTransformer {
    fn transform(&self, node: &Node, source: &str) -> Result<AstNode> {
        let node_type = match node.kind() {
            // Function definitions - Codex format: "fn myFunc"
            "function_item" => AstNodeType::FunctionDeclaration,
            
            // Struct definitions - Codex format: "struct MyStruct"
            "struct_item" => AstNodeType::StructDeclaration,
            
            // Enum definitions - Codex format: "enum MyEnum"
            "enum_item" => AstNodeType::EnumDeclaration,
            
            // Trait definitions - Codex format: "trait MyTrait"
            "trait_item" => AstNodeType::TraitDeclaration,
            
            // Impl blocks
            "impl_item" => AstNodeType::ClassDeclaration,
            
            // Module definitions
            "mod_item" => AstNodeType::Module,
            
            // Macro definitions
            "macro_definition" => AstNodeType::FunctionDeclaration,
            
            // Type aliases
            "type_item" => AstNodeType::TypeAlias,
            
            // Constants
            "const_item" => AstNodeType::ConstantDeclaration,
            
            // Static items
            "static_item" => AstNodeType::VariableDeclaration,
            
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
                language: "rust".to_string(),
                complexity: 0,
                stable_id: None,
            },
            semantic_info: None,
        })
    }
    
    fn language_name(&self) -> &'static str {
        "rust"
    }
}

fn extract_name(node: &Node, source: &str, field_name: &str) -> Result<String> {
    node.child_by_field_name(field_name)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Runtime {
            message: format!("Failed to extract {} name", field_name)
        })
}

fn extract_impl_name(node: &Node, source: &str) -> Result<String> {
    // Try to get type name from impl block
    if let Some(type_node) = node.child_by_field_name("type") {
        if let Ok(type_name) = type_node.utf8_text(source.as_bytes()) {
            // Check if it's a trait impl
            if let Some(trait_node) = node.child_by_field_name("trait") {
                if let Ok(trait_name) = trait_node.utf8_text(source.as_bytes()) {
                    return Ok(format!("{} for {}", trait_name, type_name));
                }
            }
            return Ok(type_name.to_string());
        }
    }
    Ok("_".to_string())
}

fn extract_identifier(node: &Node, source: &str) -> Option<String> {
    // Try common identifier field names
    for field in &["name", "identifier", "type_identifier"] {
        if let Some(child) = node.child_by_field_name(field) {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                return Some(text.to_string());
            }
        }
    }
    None
}
