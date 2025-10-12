// Per-language transformers matching Codex 1:1 symbol format
// CRITICAL: Symbol format must match Codex exactly (docs/05-TREE-SITTER-INTEGRATION.md)

pub mod rust_transformer;
pub mod javascript_transformer;
pub mod typescript_transformer;
pub mod python_transformer;
pub mod go_transformer;
pub mod java_transformer;
pub mod c_transformer;
pub mod cpp_transformer;
pub mod html_transformer;
pub mod css_transformer;
pub mod json_transformer;
pub mod bash_transformer;
pub mod c_sharp_transformer;
pub mod ruby_transformer;
pub mod php_transformer;
pub mod lua_transformer;
pub mod swift_transformer;
pub mod scala_transformer;
pub mod elixir_transformer;
pub mod ocaml_transformer;
pub mod nix_transformer;
pub mod make_transformer;
pub mod cmake_transformer;
pub mod verilog_transformer;
pub mod erlang_transformer;
pub mod d_transformer;
pub mod pascal_transformer;
pub mod commonlisp_transformer;
pub mod objc_transformer;
pub mod groovy_transformer;
pub mod embedded_template_transformer;

use crate::processors::cst_to_ast_pipeline::{AstNode, AstNodeType};
use crate::error::Result;

/// Trait for language-specific AST transformation
pub trait LanguageTransformer: Send + Sync {
    /// Transform a tree-sitter node into our AST format
    /// Must match Codex symbol format exactly
    fn transform(&self, node: &tree_sitter::Node, source: &str) -> Result<AstNode>;
    
    /// Get the language name
    fn language_name(&self) -> &'static str;
}

/// Get transformer for a specific language
/// Covers all 31 core languages
pub fn get_transformer(language: &str) -> Option<Box<dyn LanguageTransformer>> {
    match language {
        // Top 12 priority languages
        "rust" => Some(Box::new(rust_transformer::RustTransformer)),
        "javascript" => Some(Box::new(javascript_transformer::JavaScriptTransformer)),
        "typescript" => Some(Box::new(typescript_transformer::TypeScriptTransformer)),
        "python" => Some(Box::new(python_transformer::PythonTransformer)),
        "go" => Some(Box::new(go_transformer::GoTransformer)),
        "java" => Some(Box::new(java_transformer::JavaTransformer)),
        "c" => Some(Box::new(c_transformer::CTransformer)),
        "cpp" => Some(Box::new(cpp_transformer::CppTransformer)),
        "html" => Some(Box::new(html_transformer::HtmlTransformer)),
        "css" => Some(Box::new(css_transformer::CssTransformer)),
        "json" => Some(Box::new(json_transformer::JsonTransformer)),
        "bash" => Some(Box::new(bash_transformer::BashTransformer)),
        
        // Remaining 19 core languages
        "c_sharp" => Some(Box::new(c_sharp_transformer::C_sharpTransformer)),
        "ruby" => Some(Box::new(ruby_transformer::RubyTransformer)),
        "php" => Some(Box::new(php_transformer::PhpTransformer)),
        "lua" => Some(Box::new(lua_transformer::LuaTransformer)),
        "swift" => Some(Box::new(swift_transformer::SwiftTransformer)),
        "scala" => Some(Box::new(scala_transformer::ScalaTransformer)),
        "elixir" => Some(Box::new(elixir_transformer::ElixirTransformer)),
        "ocaml" => Some(Box::new(ocaml_transformer::OcamlTransformer)),
        "nix" => Some(Box::new(nix_transformer::NixTransformer)),
        "make" => Some(Box::new(make_transformer::MakeTransformer)),
        "cmake" => Some(Box::new(cmake_transformer::CmakeTransformer)),
        "verilog" => Some(Box::new(verilog_transformer::VerilogTransformer)),
        "erlang" => Some(Box::new(erlang_transformer::ErlangTransformer)),
        "d" => Some(Box::new(d_transformer::DTransformer)),
        "pascal" => Some(Box::new(pascal_transformer::PascalTransformer)),
        "commonlisp" => Some(Box::new(commonlisp_transformer::CommonlispTransformer)),
        "objc" => Some(Box::new(objc_transformer::ObjcTransformer)),
        "groovy" => Some(Box::new(groovy_transformer::GroovyTransformer)),
        "embedded_template" => Some(Box::new(embedded_template_transformer::Embedded_templateTransformer)),
        
        _ => None,
    }
}
