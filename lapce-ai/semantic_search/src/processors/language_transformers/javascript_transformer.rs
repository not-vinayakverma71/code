// JavaScript transformer matching Codex query format exactly
// Reference: /home/verma/lapce/Codex/src/services/tree-sitter/queries/javascript.ts
// Symbol format: "class MyClass", "function myFunc()", "const myVar"

use crate::processors::cst_to_ast_pipeline::{AstNode, AstNodeType, NodeMetadata};
use crate::processors::language_transformers::LanguageTransformer;
use crate::error::{Error, Result};
use tree_sitter::Node;

pub struct JavaScriptTransformer;

impl LanguageTransformer for JavaScriptTransformer {
    fn transform(&self, node: &Node, source: &str) -> Result<AstNode> {
        let node_type = match node.kind() {
            // Class definitions - Codex format: "class MyClass"
            "class" | "class_declaration" => AstNodeType::ClassDeclaration,
            
            // Function declarations - Codex format: "function myFunc()"
            "function_declaration" | "generator_function_declaration" => AstNodeType::FunctionDeclaration,
            
            // Method definitions - Codex format: "MyClass.method()"
            "method_definition" => AstNodeType::FunctionDeclaration,
            
            // Arrow functions and function expressions - Codex format: "const myFunc"
            "lexical_declaration" | "variable_declaration" => {
                if is_function_variable(node, source) {
                    AstNodeType::FunctionDeclaration
                } else {
                    AstNodeType::VariableDeclaration
                }
            }
            
            // JSON object definitions
            "object" => AstNodeType::ObjectExpression,
            
            // JSON array definitions
            "array" => AstNodeType::ArrayExpression,
            
            // Property definitions in objects
            "pair" => AstNodeType::Identifier,
            
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
                language: "javascript".to_string(),
                complexity: 0,
                stable_id: None,
            },
            semantic_info: None,
        })
    }
    
    fn language_name(&self) -> &'static str {
        "javascript"
    }
}

fn extract_name(node: &Node, source: &str) -> Result<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Runtime {
            message: "Failed to extract name".to_string()
        })
}

fn extract_method_name(node: &Node, source: &str) -> Result<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Runtime {
            message: "Failed to extract method name".to_string()
        })
}

fn is_function_variable(node: &Node, source: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(value_node) = child.child_by_field_name("value") {
                return matches!(
                    value_node.kind(),
                    "arrow_function" | "function_expression"
                );
            }
        }
    }
    false
}

fn extract_identifier(node: &Node, source: &str) -> Option<String> {
    for field in &["name", "identifier", "property_identifier"] {
        if let Some(child) = node.child_by_field_name(field) {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                return Some(text.to_string());
            }
        }
    }
    None
}
