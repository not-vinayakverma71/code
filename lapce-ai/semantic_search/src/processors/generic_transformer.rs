// Generic transformer that works for all CST languages
// This provides basic transformation for any language without language-specific logic

use crate::error::{Error, Result};
use crate::processors::cst_to_ast_pipeline::{
    AstNode, AstNodeType, CstNode, LanguageTransformer, NodeMetadata, SemanticInfo,
};
use std::collections::HashMap;
use std::path::Path;

/// Generic transformer for all CST-supported languages
pub struct GenericTransformer {
    language: String,
}

impl GenericTransformer {
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
        }
    }

    /// Map CST node kind to AST node type generically
    fn map_node_type(kind: &str) -> AstNodeType {
        match kind {
            // Program/Module level
            k if k.contains("source") || k.contains("program") || k.contains("translation_unit") => {
                AstNodeType::Program
            }
            k if k.contains("module") || k.contains("package") || k.contains("namespace") => {
                AstNodeType::Module
            }

            // Declarations
            k if k.contains("function") && k.contains("decl") => AstNodeType::FunctionDeclaration,
            k if k.contains("method") && k.contains("decl") => AstNodeType::FunctionDeclaration,
            k if k.contains("class") && k.contains("decl") => AstNodeType::ClassDeclaration,
            k if k.contains("interface") && k.contains("decl") => AstNodeType::InterfaceDeclaration,
            k if k.contains("struct") && (k.contains("decl") || k.contains("spec")) => {
                AstNodeType::StructDeclaration
            }
            k if k.contains("enum") && k.contains("decl") => AstNodeType::EnumDeclaration,
            k if k.contains("trait") && k.contains("decl") => AstNodeType::TraitDeclaration,
            k if k.contains("type") && k.contains("alias") => AstNodeType::TypeAlias,
            k if k.contains("variable") && k.contains("decl") => AstNodeType::VariableDeclaration,
            k if k.contains("const") && k.contains("decl") => AstNodeType::ConstantDeclaration,

            // Statements
            k if k.contains("if") && k.contains("statement") => AstNodeType::IfStatement,
            k if k.contains("while") && (k.contains("statement") || k.contains("loop")) => {
                AstNodeType::WhileLoop
            }
            k if k.contains("for") && (k.contains("statement") || k.contains("loop")) => {
                AstNodeType::ForLoop
            }
            k if k.contains("switch") || k.contains("match") => AstNodeType::SwitchStatement,
            k if k.contains("return") => AstNodeType::ReturnStatement,
            k if k.contains("break") => AstNodeType::BreakStatement,
            k if k.contains("continue") => AstNodeType::ContinueStatement,
            k if k.contains("import") || k.contains("use") || k.contains("include") => {
                AstNodeType::ImportStatement
            }
            k if k.contains("export") => AstNodeType::ExportStatement,

            // Expressions
            k if k.contains("binary") && k.contains("expr") => AstNodeType::BinaryExpression,
            k if k.contains("unary") && k.contains("expr") => AstNodeType::UnaryExpression,
            k if k.contains("call") && k.contains("expr") => AstNodeType::CallExpression,
            k if k.contains("member") || k.contains("field") => AstNodeType::MemberExpression,
            k if k.contains("array") && k.contains("expr") => AstNodeType::ArrayExpression,
            k if k.contains("object") || k.contains("struct_expr") => {
                AstNodeType::ObjectExpression
            }

            // Literals
            k if k.contains("string") && k.contains("literal") => AstNodeType::StringLiteral,
            k if k.contains("number") || k.contains("integer") || k.contains("float") => {
                AstNodeType::NumberLiteral
            }
            k if k.contains("boolean") || k == "true" || k == "false" => {
                AstNodeType::BooleanLiteral
            }
            k if k.contains("null") || k.contains("nil") => AstNodeType::NullLiteral,

            // Identifiers
            k if k == "identifier" || k == "type_identifier" || k == "field_identifier" => {
                AstNodeType::Identifier
            }

            // Default
            _ => AstNodeType::Unknown,
        }
    }

    fn transform_node(&self, cst: &CstNode, path: &Path, scope_depth: usize) -> Result<AstNode> {
        // Map node type
        let node_type = Self::map_node_type(&cst.kind);

        // Extract identifier
        let identifier = self.extract_identifier(cst);

        // Process children
        let mut ast_children = Vec::new();
        for child in &cst.children {
            if child.is_named && !child.is_extra {
                ast_children.push(self.transform_node(child, path, scope_depth + 1)?);
            }
        }

        // Extract value for leaf nodes
        let value = if cst.children.is_empty() {
            Some(cst.text.clone())
        } else {
            None
        };

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
                language: self.language.clone(),
                complexity: self.calculate_complexity(cst),
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

    fn extract_identifier(&self, cst: &CstNode) -> Option<String> {
        // Look for identifier nodes in children
        for child in &cst.children {
            if child.kind == "identifier"
                || child.kind == "type_identifier"
                || child.kind == "field_identifier"
                || child.kind == "name"
            {
                return Some(child.text.clone());
            }
        }
        None
    }

    fn calculate_complexity(&self, cst: &CstNode) -> usize {
        let mut complexity = 1;

        // Count decision points
        let kind = cst.kind.as_str();
        if kind.contains("if")
            || kind.contains("while")
            || kind.contains("for")
            || kind.contains("switch")
            || kind.contains("match")
            || kind.contains("case")
        {
            complexity += 1;
        }

        // Add complexity from children
        for child in &cst.children {
            complexity += self.calculate_complexity(child);
        }

        complexity
    }
}

impl LanguageTransformer for GenericTransformer {
    fn transform(&self, cst: &CstNode, path: &Path) -> Result<AstNode> {
        self.transform_node(cst, path, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_transformer_node_type_mapping() {
        assert_eq!(
            GenericTransformer::map_node_type("source_file"),
            AstNodeType::Program
        );
        assert_eq!(
            GenericTransformer::map_node_type("function_declaration"),
            AstNodeType::FunctionDeclaration
        );
        assert_eq!(
            GenericTransformer::map_node_type("if_statement"),
            AstNodeType::IfStatement
        );
    }
}
