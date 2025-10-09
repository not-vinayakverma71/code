//! Canonical kind and field mappings for cross-language AST compatibility
//! 
//! This module provides standardized mappings between tree-sitter node kinds
//! and canonical AST node types for semantic analysis.

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Canonical AST node kinds that are language-agnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanonicalKind {
    // Structural
    Module,
    Block,
    Statement,
    Expression,
    
    // Declarations
    FunctionDeclaration,
    ClassDeclaration,
    InterfaceDeclaration,
    StructDeclaration,
    EnumDeclaration,
    TypeAlias,
    VariableDeclaration,
    ConstantDeclaration,
    
    // Functions
    FunctionSignature,
    ParameterList,
    Parameter,
    ReturnType,
    FunctionBody,
    
    // Types
    TypeAnnotation,
    GenericType,
    ArrayType,
    PointerType,
    ReferenceType,
    UnionType,
    IntersectionType,
    
    // Expressions
    BinaryExpression,
    UnaryExpression,
    CallExpression,
    MemberExpression,
    IndexExpression,
    LiteralExpression,
    IdentifierExpression,
    AssignmentExpression,
    
    // Control flow
    IfStatement,
    ForLoop,
    WhileLoop,
    DoWhileLoop,
    SwitchStatement,
    CaseClause,
    BreakStatement,
    ContinueStatement,
    ReturnStatement,
    ThrowStatement,
    TryStatement,
    CatchClause,
    
    // Literals
    StringLiteral,
    NumberLiteral,
    BooleanLiteral,
    NullLiteral,
    RegexLiteral,
    TemplateLiteral,
    
    // Comments
    LineComment,
    BlockComment,
    DocComment,
    
    // Other
    Identifier,
    Operator,
    Keyword,
    Punctuation,
    Error,
    Unknown,
}

/// Language-specific kind mapping
pub struct LanguageMapping {
    /// Map from tree-sitter kind string to canonical kind
    pub kind_map: HashMap<&'static str, CanonicalKind>,
    /// Map from tree-sitter field name to canonical field name
    pub field_map: HashMap<&'static str, &'static str>,
    /// Grammar version for compatibility checking
    pub grammar_version: &'static str,
}

/// Rust language mapping
pub static RUST_MAPPING: Lazy<LanguageMapping> = Lazy::new(|| {
    let mut kind_map = HashMap::new();
    
    // Structural
    kind_map.insert("source_file", CanonicalKind::Module);
    kind_map.insert("block", CanonicalKind::Block);
    kind_map.insert("statement", CanonicalKind::Statement);
    kind_map.insert("expression_statement", CanonicalKind::Statement);
    
    // Declarations
    kind_map.insert("function_item", CanonicalKind::FunctionDeclaration);
    kind_map.insert("struct_item", CanonicalKind::StructDeclaration);
    kind_map.insert("enum_item", CanonicalKind::EnumDeclaration);
    kind_map.insert("trait_item", CanonicalKind::InterfaceDeclaration);
    kind_map.insert("type_item", CanonicalKind::TypeAlias);
    kind_map.insert("let_declaration", CanonicalKind::VariableDeclaration);
    kind_map.insert("const_item", CanonicalKind::ConstantDeclaration);
    kind_map.insert("static_item", CanonicalKind::ConstantDeclaration);
    
    // Functions
    kind_map.insert("parameters", CanonicalKind::ParameterList);
    kind_map.insert("parameter", CanonicalKind::Parameter);
    kind_map.insert("function_signature_item", CanonicalKind::FunctionSignature);
    
    // Types
    kind_map.insert("type_identifier", CanonicalKind::TypeAnnotation);
    kind_map.insert("generic_type", CanonicalKind::GenericType);
    kind_map.insert("array_type", CanonicalKind::ArrayType);
    kind_map.insert("pointer_type", CanonicalKind::PointerType);
    kind_map.insert("reference_type", CanonicalKind::ReferenceType);
    
    // Expressions
    kind_map.insert("binary_expression", CanonicalKind::BinaryExpression);
    kind_map.insert("unary_expression", CanonicalKind::UnaryExpression);
    kind_map.insert("call_expression", CanonicalKind::CallExpression);
    kind_map.insert("field_expression", CanonicalKind::MemberExpression);
    kind_map.insert("index_expression", CanonicalKind::IndexExpression);
    kind_map.insert("assignment_expression", CanonicalKind::AssignmentExpression);
    
    // Control flow
    kind_map.insert("if_expression", CanonicalKind::IfStatement);
    kind_map.insert("for_expression", CanonicalKind::ForLoop);
    kind_map.insert("while_expression", CanonicalKind::WhileLoop);
    kind_map.insert("loop_expression", CanonicalKind::WhileLoop);
    kind_map.insert("match_expression", CanonicalKind::SwitchStatement);
    kind_map.insert("match_arm", CanonicalKind::CaseClause);
    kind_map.insert("break_expression", CanonicalKind::BreakStatement);
    kind_map.insert("continue_expression", CanonicalKind::ContinueStatement);
    kind_map.insert("return_expression", CanonicalKind::ReturnStatement);
    
    // Literals
    kind_map.insert("string_literal", CanonicalKind::StringLiteral);
    kind_map.insert("integer_literal", CanonicalKind::NumberLiteral);
    kind_map.insert("float_literal", CanonicalKind::NumberLiteral);
    kind_map.insert("boolean_literal", CanonicalKind::BooleanLiteral);
    kind_map.insert("char_literal", CanonicalKind::StringLiteral);
    
    // Comments
    kind_map.insert("line_comment", CanonicalKind::LineComment);
    kind_map.insert("block_comment", CanonicalKind::BlockComment);
    
    // Other
    kind_map.insert("identifier", CanonicalKind::Identifier);
    kind_map.insert("ERROR", CanonicalKind::Error);
    
    let mut field_map = HashMap::new();
    field_map.insert("name", "name");
    field_map.insert("body", "body");
    field_map.insert("parameters", "params");
    field_map.insert("return_type", "returnType");
    field_map.insert("type", "type");
    field_map.insert("value", "value");
    field_map.insert("condition", "condition");
    field_map.insert("consequence", "then");
    field_map.insert("alternative", "else");
    field_map.insert("left", "left");
    field_map.insert("right", "right");
    field_map.insert("operator", "operator");
    field_map.insert("argument", "argument");
    field_map.insert("arguments", "arguments");
    
    LanguageMapping {
        kind_map,
        field_map,
        grammar_version: "0.23.0",
    }
});

/// Python language mapping
pub static PYTHON_MAPPING: Lazy<LanguageMapping> = Lazy::new(|| {
    let mut kind_map = HashMap::new();
    
    // Structural
    kind_map.insert("module", CanonicalKind::Module);
    kind_map.insert("block", CanonicalKind::Block);
    kind_map.insert("statement", CanonicalKind::Statement);
    kind_map.insert("simple_statement", CanonicalKind::Statement);
    
    // Declarations
    kind_map.insert("function_definition", CanonicalKind::FunctionDeclaration);
    kind_map.insert("class_definition", CanonicalKind::ClassDeclaration);
    kind_map.insert("assignment", CanonicalKind::VariableDeclaration);
    
    // Functions
    kind_map.insert("parameters", CanonicalKind::ParameterList);
    kind_map.insert("parameter", CanonicalKind::Parameter);
    kind_map.insert("lambda", CanonicalKind::FunctionDeclaration);
    
    // Expressions
    kind_map.insert("binary_operator", CanonicalKind::BinaryExpression);
    kind_map.insert("unary_operator", CanonicalKind::UnaryExpression);
    kind_map.insert("call", CanonicalKind::CallExpression);
    kind_map.insert("attribute", CanonicalKind::MemberExpression);
    kind_map.insert("subscript", CanonicalKind::IndexExpression);
    
    // Control flow
    kind_map.insert("if_statement", CanonicalKind::IfStatement);
    kind_map.insert("for_statement", CanonicalKind::ForLoop);
    kind_map.insert("while_statement", CanonicalKind::WhileLoop);
    kind_map.insert("match_statement", CanonicalKind::SwitchStatement);
    kind_map.insert("case_clause", CanonicalKind::CaseClause);
    kind_map.insert("break_statement", CanonicalKind::BreakStatement);
    kind_map.insert("continue_statement", CanonicalKind::ContinueStatement);
    kind_map.insert("return_statement", CanonicalKind::ReturnStatement);
    kind_map.insert("raise_statement", CanonicalKind::ThrowStatement);
    kind_map.insert("try_statement", CanonicalKind::TryStatement);
    kind_map.insert("except_clause", CanonicalKind::CatchClause);
    
    // Literals
    kind_map.insert("string", CanonicalKind::StringLiteral);
    kind_map.insert("integer", CanonicalKind::NumberLiteral);
    kind_map.insert("float", CanonicalKind::NumberLiteral);
    kind_map.insert("true", CanonicalKind::BooleanLiteral);
    kind_map.insert("false", CanonicalKind::BooleanLiteral);
    kind_map.insert("none", CanonicalKind::NullLiteral);
    
    // Comments
    kind_map.insert("comment", CanonicalKind::LineComment);
    
    // Other
    kind_map.insert("identifier", CanonicalKind::Identifier);
    kind_map.insert("ERROR", CanonicalKind::Error);
    
    let mut field_map = HashMap::new();
    field_map.insert("name", "name");
    field_map.insert("body", "body");
    field_map.insert("parameters", "params");
    field_map.insert("return_type", "returnType");
    field_map.insert("value", "value");
    field_map.insert("condition", "condition");
    field_map.insert("consequence", "then");
    field_map.insert("alternative", "else");
    field_map.insert("left", "left");
    field_map.insert("right", "right");
    field_map.insert("operator", "operator");
    field_map.insert("argument", "argument");
    field_map.insert("arguments", "arguments");
    
    LanguageMapping {
        kind_map,
        field_map,
        grammar_version: "0.23.0",
    }
});

/// JavaScript/TypeScript language mapping
pub static JAVASCRIPT_MAPPING: Lazy<LanguageMapping> = Lazy::new(|| {
    let mut kind_map = HashMap::new();
    
    // Structural
    kind_map.insert("program", CanonicalKind::Module);
    kind_map.insert("statement_block", CanonicalKind::Block);
    kind_map.insert("statement", CanonicalKind::Statement);
    
    // Declarations
    kind_map.insert("function_declaration", CanonicalKind::FunctionDeclaration);
    kind_map.insert("class_declaration", CanonicalKind::ClassDeclaration);
    kind_map.insert("interface_declaration", CanonicalKind::InterfaceDeclaration);
    kind_map.insert("variable_declaration", CanonicalKind::VariableDeclaration);
    kind_map.insert("lexical_declaration", CanonicalKind::VariableDeclaration);
    
    // Functions
    kind_map.insert("formal_parameters", CanonicalKind::ParameterList);
    kind_map.insert("parameter", CanonicalKind::Parameter);
    kind_map.insert("arrow_function", CanonicalKind::FunctionDeclaration);
    kind_map.insert("function_expression", CanonicalKind::FunctionDeclaration);
    
    // Expressions
    kind_map.insert("binary_expression", CanonicalKind::BinaryExpression);
    kind_map.insert("unary_expression", CanonicalKind::UnaryExpression);
    kind_map.insert("call_expression", CanonicalKind::CallExpression);
    kind_map.insert("member_expression", CanonicalKind::MemberExpression);
    kind_map.insert("subscript_expression", CanonicalKind::IndexExpression);
    kind_map.insert("assignment_expression", CanonicalKind::AssignmentExpression);
    
    // Control flow
    kind_map.insert("if_statement", CanonicalKind::IfStatement);
    kind_map.insert("for_statement", CanonicalKind::ForLoop);
    kind_map.insert("for_in_statement", CanonicalKind::ForLoop);
    kind_map.insert("for_of_statement", CanonicalKind::ForLoop);
    kind_map.insert("while_statement", CanonicalKind::WhileLoop);
    kind_map.insert("do_statement", CanonicalKind::DoWhileLoop);
    kind_map.insert("switch_statement", CanonicalKind::SwitchStatement);
    kind_map.insert("switch_case", CanonicalKind::CaseClause);
    kind_map.insert("break_statement", CanonicalKind::BreakStatement);
    kind_map.insert("continue_statement", CanonicalKind::ContinueStatement);
    kind_map.insert("return_statement", CanonicalKind::ReturnStatement);
    kind_map.insert("throw_statement", CanonicalKind::ThrowStatement);
    kind_map.insert("try_statement", CanonicalKind::TryStatement);
    kind_map.insert("catch_clause", CanonicalKind::CatchClause);
    
    // Literals
    kind_map.insert("string", CanonicalKind::StringLiteral);
    kind_map.insert("number", CanonicalKind::NumberLiteral);
    kind_map.insert("true", CanonicalKind::BooleanLiteral);
    kind_map.insert("false", CanonicalKind::BooleanLiteral);
    kind_map.insert("null", CanonicalKind::NullLiteral);
    kind_map.insert("regex", CanonicalKind::RegexLiteral);
    kind_map.insert("template_string", CanonicalKind::TemplateLiteral);
    
    // Comments
    kind_map.insert("comment", CanonicalKind::LineComment);
    
    // Other
    kind_map.insert("identifier", CanonicalKind::Identifier);
    kind_map.insert("ERROR", CanonicalKind::Error);
    
    let mut field_map = HashMap::new();
    field_map.insert("name", "name");
    field_map.insert("body", "body");
    field_map.insert("parameters", "params");
    field_map.insert("value", "value");
    field_map.insert("condition", "condition");
    field_map.insert("consequence", "then");
    field_map.insert("alternate", "else");
    field_map.insert("left", "left");
    field_map.insert("right", "right");
    field_map.insert("operator", "operator");
    field_map.insert("argument", "argument");
    field_map.insert("arguments", "arguments");
    
    LanguageMapping {
        kind_map,
        field_map,
        grammar_version: "0.23.0",
    }
});

/// Go language mapping
pub static GO_MAPPING: Lazy<LanguageMapping> = Lazy::new(|| {
    let mut kind_map = HashMap::new();
    
    // Structural
    kind_map.insert("source_file", CanonicalKind::Module);
    kind_map.insert("block", CanonicalKind::Block);
    
    // Declarations
    kind_map.insert("function_declaration", CanonicalKind::FunctionDeclaration);
    kind_map.insert("method_declaration", CanonicalKind::FunctionDeclaration);
    kind_map.insert("type_declaration", CanonicalKind::TypeAlias);
    kind_map.insert("struct_type", CanonicalKind::StructDeclaration);
    kind_map.insert("interface_type", CanonicalKind::InterfaceDeclaration);
    kind_map.insert("var_declaration", CanonicalKind::VariableDeclaration);
    kind_map.insert("const_declaration", CanonicalKind::ConstantDeclaration);
    kind_map.insert("short_var_declaration", CanonicalKind::VariableDeclaration);
    
    // Functions
    kind_map.insert("parameter_list", CanonicalKind::ParameterList);
    kind_map.insert("parameter_declaration", CanonicalKind::Parameter);
    
    // Expressions
    kind_map.insert("binary_expression", CanonicalKind::BinaryExpression);
    kind_map.insert("unary_expression", CanonicalKind::UnaryExpression);
    kind_map.insert("call_expression", CanonicalKind::CallExpression);
    kind_map.insert("selector_expression", CanonicalKind::MemberExpression);
    kind_map.insert("index_expression", CanonicalKind::IndexExpression);
    kind_map.insert("assignment_statement", CanonicalKind::AssignmentExpression);
    
    // Control flow
    kind_map.insert("if_statement", CanonicalKind::IfStatement);
    kind_map.insert("for_statement", CanonicalKind::ForLoop);
    kind_map.insert("switch_statement", CanonicalKind::SwitchStatement);
    kind_map.insert("expression_case", CanonicalKind::CaseClause);
    kind_map.insert("break_statement", CanonicalKind::BreakStatement);
    kind_map.insert("continue_statement", CanonicalKind::ContinueStatement);
    kind_map.insert("return_statement", CanonicalKind::ReturnStatement);
    
    let mut field_map = HashMap::new();
    field_map.insert("name", "name");
    field_map.insert("type", "type");
    field_map.insert("body", "body");
    field_map.insert("parameters", "parameters");
    field_map.insert("result", "return_type");
    field_map.insert("condition", "condition");
    field_map.insert("consequence", "then");
    field_map.insert("alternative", "else");
    
    LanguageMapping {
        kind_map,
        field_map,
        grammar_version: "0.23.0",
    }
});

/// C language mapping
pub static C_MAPPING: Lazy<LanguageMapping> = Lazy::new(|| {
    let mut kind_map = HashMap::new();
    
    // Structural
    kind_map.insert("translation_unit", CanonicalKind::Module);
    kind_map.insert("compound_statement", CanonicalKind::Block);
    
    // Declarations
    kind_map.insert("function_definition", CanonicalKind::FunctionDeclaration);
    kind_map.insert("declaration", CanonicalKind::VariableDeclaration);
    kind_map.insert("struct_specifier", CanonicalKind::StructDeclaration);
    kind_map.insert("enum_specifier", CanonicalKind::EnumDeclaration);
    kind_map.insert("type_definition", CanonicalKind::TypeAlias);
    
    // Functions
    kind_map.insert("parameter_list", CanonicalKind::ParameterList);
    kind_map.insert("parameter_declaration", CanonicalKind::Parameter);
    
    // Expressions
    kind_map.insert("binary_expression", CanonicalKind::BinaryExpression);
    kind_map.insert("unary_expression", CanonicalKind::UnaryExpression);
    kind_map.insert("call_expression", CanonicalKind::CallExpression);
    kind_map.insert("field_expression", CanonicalKind::MemberExpression);
    kind_map.insert("subscript_expression", CanonicalKind::IndexExpression);
    kind_map.insert("assignment_expression", CanonicalKind::AssignmentExpression);
    
    // Control flow
    kind_map.insert("if_statement", CanonicalKind::IfStatement);
    kind_map.insert("for_statement", CanonicalKind::ForLoop);
    kind_map.insert("while_statement", CanonicalKind::WhileLoop);
    kind_map.insert("do_statement", CanonicalKind::DoWhileLoop);
    kind_map.insert("switch_statement", CanonicalKind::SwitchStatement);
    kind_map.insert("case_statement", CanonicalKind::CaseClause);
    kind_map.insert("break_statement", CanonicalKind::BreakStatement);
    kind_map.insert("continue_statement", CanonicalKind::ContinueStatement);
    kind_map.insert("return_statement", CanonicalKind::ReturnStatement);
    
    let mut field_map = HashMap::new();
    field_map.insert("declarator", "name");
    field_map.insert("type", "type");
    field_map.insert("body", "body");
    field_map.insert("parameters", "parameters");
    field_map.insert("condition", "condition");
    field_map.insert("consequence", "then");
    field_map.insert("alternative", "else");
    
    LanguageMapping {
        kind_map,
        field_map,
        grammar_version: "0.23.0",
    }
});

/// Java language mapping
pub static JAVA_MAPPING: Lazy<LanguageMapping> = Lazy::new(|| {
    let mut kind_map = HashMap::new();
    
    // Structural
    kind_map.insert("program", CanonicalKind::Module);
    kind_map.insert("block", CanonicalKind::Block);
    
    // Declarations
    kind_map.insert("method_declaration", CanonicalKind::FunctionDeclaration);
    kind_map.insert("class_declaration", CanonicalKind::ClassDeclaration);
    kind_map.insert("interface_declaration", CanonicalKind::InterfaceDeclaration);
    kind_map.insert("enum_declaration", CanonicalKind::EnumDeclaration);
    kind_map.insert("local_variable_declaration", CanonicalKind::VariableDeclaration);
    kind_map.insert("field_declaration", CanonicalKind::VariableDeclaration);
    
    // Functions
    kind_map.insert("formal_parameters", CanonicalKind::ParameterList);
    kind_map.insert("formal_parameter", CanonicalKind::Parameter);
    
    // Expressions
    kind_map.insert("binary_expression", CanonicalKind::BinaryExpression);
    kind_map.insert("unary_expression", CanonicalKind::UnaryExpression);
    kind_map.insert("method_invocation", CanonicalKind::CallExpression);
    kind_map.insert("field_access", CanonicalKind::MemberExpression);
    kind_map.insert("array_access", CanonicalKind::IndexExpression);
    kind_map.insert("assignment_expression", CanonicalKind::AssignmentExpression);
    kind_map.insert("lambda_expression", CanonicalKind::FunctionDeclaration);
    
    // Control flow
    kind_map.insert("if_statement", CanonicalKind::IfStatement);
    kind_map.insert("for_statement", CanonicalKind::ForLoop);
    kind_map.insert("enhanced_for_statement", CanonicalKind::ForLoop);
    kind_map.insert("while_statement", CanonicalKind::WhileLoop);
    kind_map.insert("do_statement", CanonicalKind::DoWhileLoop);
    kind_map.insert("switch_expression", CanonicalKind::SwitchStatement);
    kind_map.insert("switch_label", CanonicalKind::CaseClause);
    kind_map.insert("break_statement", CanonicalKind::BreakStatement);
    kind_map.insert("continue_statement", CanonicalKind::ContinueStatement);
    kind_map.insert("return_statement", CanonicalKind::ReturnStatement);
    
    let mut field_map = HashMap::new();
    field_map.insert("name", "name");
    field_map.insert("type", "type");
    field_map.insert("body", "body");
    field_map.insert("parameters", "parameters");
    field_map.insert("condition", "condition");
    field_map.insert("consequence", "then");
    field_map.insert("alternative", "else");
    
    LanguageMapping {
        kind_map,
        field_map,
        grammar_version: "0.23.0",
    }
});

/// Get language mapping for a specific language
pub fn get_language_mapping(language: &str) -> Option<&'static LanguageMapping> {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => Some(&RUST_MAPPING),
        "python" | "py" => Some(&PYTHON_MAPPING),
        "javascript" | "js" | "typescript" | "ts" => Some(&JAVASCRIPT_MAPPING),
        "go" => Some(&GO_MAPPING),
        "c" => Some(&C_MAPPING),
        "cpp" | "c++" | "cc" | "cxx" => Some(&C_MAPPING), // C++ uses C mapping for now
        "java" => Some(&JAVA_MAPPING),
        _ => None,
    }
}

/// Map a tree-sitter kind to canonical kind for a language
pub fn map_kind(language: &str, ts_kind: &str) -> CanonicalKind {
    get_language_mapping(language)
        .and_then(|mapping| mapping.kind_map.get(ts_kind).copied())
        .unwrap_or(CanonicalKind::Unknown)
}

/// Map a tree-sitter field to canonical field for a language
/// Returns None if no mapping found (caller should use original field name)
pub fn map_field(language: &str, ts_field: &str) -> Option<&'static str> {
    get_language_mapping(language)
        .and_then(|mapping| mapping.field_map.get(ts_field).copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rust_mapping() {
        assert_eq!(map_kind("rust", "function_item"), CanonicalKind::FunctionDeclaration);
        assert_eq!(map_kind("rust", "if_expression"), CanonicalKind::IfStatement);
        assert_eq!(map_kind("rust", "string_literal"), CanonicalKind::StringLiteral);
        assert_eq!(map_kind("rust", "unknown_node"), CanonicalKind::Unknown);
    }
    
    #[test]
    fn test_python_mapping() {
        assert_eq!(map_kind("python", "function_definition"), CanonicalKind::FunctionDeclaration);
        assert_eq!(map_kind("python", "if_statement"), CanonicalKind::IfStatement);
        assert_eq!(map_kind("python", "string"), CanonicalKind::StringLiteral);
    }
    
    #[test]
    fn test_javascript_mapping() {
        assert_eq!(map_kind("javascript", "function_declaration"), CanonicalKind::FunctionDeclaration);
        assert_eq!(map_kind("js", "if_statement"), CanonicalKind::IfStatement);
        assert_eq!(map_kind("typescript", "template_string"), CanonicalKind::TemplateLiteral);
    }
    
    #[test]
    fn test_field_mapping() {
        assert_eq!(map_field("rust", "parameters"), Some("params"));
        assert_eq!(map_field("rust", "return_type"), Some("returnType"));
        assert_eq!(map_field("python", "consequence"), Some("then"));
        assert_eq!(map_field("javascript", "alternate"), Some("else"));
        assert_eq!(map_field("rust", "unknown_field"), None);
    }
    
    #[test]
    fn test_grammar_versions() {
        assert_eq!(RUST_MAPPING.grammar_version, "0.23.0");
        assert_eq!(PYTHON_MAPPING.grammar_version, "0.23.0");
        assert_eq!(JAVASCRIPT_MAPPING.grammar_version, "0.23.0");
        assert_eq!(GO_MAPPING.grammar_version, "0.23.0");
        assert_eq!(C_MAPPING.grammar_version, "0.23.0");
        assert_eq!(JAVA_MAPPING.grammar_version, "0.23.0");
    }
    
    #[test]
    fn test_go_mapping() {
        assert_eq!(map_kind("go", "function_declaration"), CanonicalKind::FunctionDeclaration);
        assert_eq!(map_kind("go", "if_statement"), CanonicalKind::IfStatement);
        assert_eq!(map_kind("go", "struct_type"), CanonicalKind::StructDeclaration);
        assert_eq!(map_kind("go", "interface_type"), CanonicalKind::InterfaceDeclaration);
    }
    
    #[test]
    fn test_c_mapping() {
        assert_eq!(map_kind("c", "function_definition"), CanonicalKind::FunctionDeclaration);
        assert_eq!(map_kind("c", "if_statement"), CanonicalKind::IfStatement);
        assert_eq!(map_kind("c", "struct_specifier"), CanonicalKind::StructDeclaration);
        assert_eq!(map_kind("c", "while_statement"), CanonicalKind::WhileLoop);
    }
    
    #[test]
    fn test_java_mapping() {
        assert_eq!(map_kind("java", "method_declaration"), CanonicalKind::FunctionDeclaration);
        assert_eq!(map_kind("java", "class_declaration"), CanonicalKind::ClassDeclaration);
        assert_eq!(map_kind("java", "interface_declaration"), CanonicalKind::InterfaceDeclaration);
        assert_eq!(map_kind("java", "lambda_expression"), CanonicalKind::FunctionDeclaration);
    }
}
