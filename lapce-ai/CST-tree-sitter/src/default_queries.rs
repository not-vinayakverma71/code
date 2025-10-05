//! Default queries for languages without .scm files
//! Provides fallback highlighting and symbol extraction

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Default highlight query that works for most C-like languages
pub static DEFAULT_HIGHLIGHT_QUERY: &str = r#"
; Keywords
[
  "if" "else" "elif" "endif"
  "for" "while" "do" "break" "continue"
  "function" "func" "fn" "def" "lambda"
  "return" "yield"
  "class" "struct" "enum" "interface" "trait"
  "public" "private" "protected" "static"
  "const" "let" "var" "val"
  "import" "export" "module" "package" "use"
  "try" "catch" "finally" "throw" "throws"
  "new" "delete" "typeof" "instanceof"
  "async" "await"
  "override" "virtual" "abstract"
  "extends" "implements"
] @keyword

; Functions and methods
(function_declaration name: (identifier) @function)
(function_definition name: (identifier) @function)
(method_declaration name: (identifier) @method)
(method_definition name: (identifier) @method)
(call_expression function: (identifier) @function.call)

; Types
[
  (type_identifier)
  (primitive_type)
  (interface_declaration name: (identifier) @type)
  (class_declaration name: (identifier) @type)
  (struct_declaration name: (identifier) @type)
  (enum_declaration name: (identifier) @type)
] @type

; Variables and parameters
(variable_declaration name: (identifier) @variable)
(parameter_declaration name: (identifier) @parameter)
(field_declaration name: (identifier) @field)

; Constants
(const_declaration name: (identifier) @constant)
(enum_member name: (identifier) @constant)

; Strings
[
  (string)
  (string_literal)
  (template_string)
  (char_literal)
] @string

; Numbers
[
  (number)
  (integer)
  (float)
  (hex_integer)
  (binary_integer)
  (octal_integer)
] @number

; Comments
[
  (comment)
  (line_comment)
  (block_comment)
] @comment

; Operators
[
  "+" "-" "*" "/" "%" 
  "==" "!=" "<" ">" "<=" ">="
  "&&" "||" "!"
  "=" "+=" "-=" "*=" "/="
  "&" "|" "^" "~" "<<" ">>"
  "." "->" "::" "=>"
] @operator

; Punctuation
[
  "(" ")" "[" "]" "{" "}"
  "," ";" ":"
] @punctuation
"#;

/// Default locals query for scope tracking
pub static DEFAULT_LOCALS_QUERY: &str = r#"
; Scopes
[
  (function_declaration)
  (function_definition)
  (method_declaration)
  (method_definition)
  (class_declaration)
  (class_definition)
  (block)
  (if_statement)
  (for_statement)
  (while_statement)
] @scope

; Definitions
(function_declaration name: (identifier) @definition.function)
(variable_declaration name: (identifier) @definition.var)
(parameter_declaration name: (identifier) @definition.parameter)
(field_declaration name: (identifier) @definition.field)

; References
(identifier) @reference
"#;

/// Default tags query for symbol extraction
pub static DEFAULT_TAGS_QUERY: &str = r#"
; Class/Struct definitions
(class_declaration
  name: (identifier) @name) @definition.class

(struct_declaration
  name: (identifier) @name) @definition.struct

; Function definitions
(function_declaration
  name: (identifier) @name) @definition.function

(function_definition
  name: (identifier) @name) @definition.function

(method_declaration
  name: (identifier) @name) @definition.method

(method_definition
  name: (identifier) @name) @definition.method

; Variable definitions
(variable_declaration
  name: (identifier) @name) @definition.variable

(const_declaration
  name: (identifier) @name) @definition.constant

; Field definitions
(field_declaration
  name: (identifier) @name) @definition.field

; Enum definitions
(enum_declaration
  name: (identifier) @name) @definition.enum

(enum_member
  name: (identifier) @name) @definition.enumerator

; Interface/Trait definitions
(interface_declaration
  name: (identifier) @name) @definition.interface

(trait_declaration
  name: (identifier) @name) @definition.interface
"#;

/// Default fold query for code folding
pub static DEFAULT_FOLDS_QUERY: &str = r#"
[
  (function_declaration body: (_) @fold)
  (function_definition body: (_) @fold)
  (class_declaration body: (_) @fold)
  (class_definition body: (_) @fold)
  (if_statement consequence: (_) @fold)
  (for_statement body: (_) @fold)
  (while_statement body: (_) @fold)
  (block) @fold
] 
"#;

/// Default injection query for embedded languages
pub static DEFAULT_INJECTIONS_QUERY: &str = r#"
; SQL in strings
((string) @injection.content
  (#match? @injection.content "^[\\s]*SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER")
  (#set! injection.language "sql"))

; Regex in strings
((string) @injection.content
  (#match? @injection.content "^/.*/$")
  (#set! injection.language "regex"))

; JSON in strings
((string) @injection.content
  (#match? @injection.content "^[\\s]*\\{.*\\}[\\s]*$")
  (#set! injection.language "json"))
"#;

/// Language-specific overrides for better accuracy
pub fn get_language_specific_queries() -> HashMap<String, LanguageQueries> {
    let mut queries = HashMap::new();
    
    // Rust-specific queries
    queries.insert("rust".to_string(), LanguageQueries {
        highlights: Some(r#"
; Rust-specific highlights
"fn" @keyword.function
"impl" @keyword.impl  
"trait" @keyword.trait
"struct" @keyword.struct
"enum" @keyword.enum
"mod" @keyword.module
"use" @keyword.use
"pub" @keyword.visibility
"mut" @keyword.mut
"ref" @keyword.ref
"move" @keyword.move
"dyn" @keyword.dyn
"async" @keyword.async
"await" @keyword.await
"unsafe" @keyword.unsafe

(macro_invocation macro: (identifier) @function.macro)
(attribute_item) @attribute
(lifetime) @label
"#.to_string()),
        ..Default::default()
    });
    
    // Python-specific queries
    queries.insert("python".to_string(), LanguageQueries {
        highlights: Some(r#"
; Python-specific highlights
"def" @keyword.function
"class" @keyword.class
"import" @keyword.import
"from" @keyword.import
"as" @keyword.import
"if" @keyword.conditional
"elif" @keyword.conditional
"else" @keyword.conditional
"for" @keyword.repeat
"while" @keyword.repeat
"with" @keyword.with
"try" @keyword.exception
"except" @keyword.exception
"finally" @keyword.exception
"raise" @keyword.exception
"lambda" @keyword.function
"return" @keyword.return
"yield" @keyword.return
"pass" @keyword.pass
"break" @keyword.break
"continue" @keyword.continue

(decorator) @function.decorator
(self) @variable.builtin
"#.to_string()),
        ..Default::default()
    });
    
    // JavaScript/TypeScript-specific queries
    queries.insert("javascript".to_string(), LanguageQueries {
        highlights: Some(r#"
; JavaScript-specific highlights
"function" @keyword.function
"const" @keyword.const
"let" @keyword.let
"var" @keyword.var
"if" @keyword.conditional
"else" @keyword.conditional
"for" @keyword.repeat
"while" @keyword.repeat
"do" @keyword.repeat
"switch" @keyword.conditional
"case" @keyword.conditional
"default" @keyword.conditional
"break" @keyword.break
"continue" @keyword.continue
"return" @keyword.return
"new" @keyword.new
"delete" @keyword.delete
"typeof" @keyword.operator
"instanceof" @keyword.operator
"async" @keyword.async
"await" @keyword.await
"class" @keyword.class
"extends" @keyword.extends
"import" @keyword.import
"export" @keyword.export
"from" @keyword.import

(arrow_function) @function
(template_string) @string.special
(regex) @string.regex
(jsx_element) @tag
(jsx_attribute) @attribute
"#.to_string()),
        ..Default::default()
    });
    
    queries
}

#[derive(Debug, Clone, Default)]
pub struct LanguageQueries {
    pub highlights: Option<String>,
    pub locals: Option<String>,
    pub tags: Option<String>,
    pub folds: Option<String>,
    pub injections: Option<String>,
}

/// Get queries for a specific language with fallback to defaults
pub fn get_queries_for_language(language: &str) -> LanguageQueries {
    let specific_queries = get_language_specific_queries();
    
    if let Some(queries) = specific_queries.get(language) {
        LanguageQueries {
            highlights: queries.highlights.clone().or_else(|| Some(DEFAULT_HIGHLIGHT_QUERY.to_string())),
            locals: queries.locals.clone().or_else(|| Some(DEFAULT_LOCALS_QUERY.to_string())),
            tags: queries.tags.clone().or_else(|| Some(DEFAULT_TAGS_QUERY.to_string())),
            folds: queries.folds.clone().or_else(|| Some(DEFAULT_FOLDS_QUERY.to_string())),
            injections: queries.injections.clone().or_else(|| Some(DEFAULT_INJECTIONS_QUERY.to_string())),
        }
    } else {
        // Return default queries for unknown languages
        LanguageQueries {
            highlights: Some(DEFAULT_HIGHLIGHT_QUERY.to_string()),
            locals: Some(DEFAULT_LOCALS_QUERY.to_string()),
            tags: Some(DEFAULT_TAGS_QUERY.to_string()),
            folds: Some(DEFAULT_FOLDS_QUERY.to_string()),
            injections: Some(DEFAULT_INJECTIONS_QUERY.to_string()),
        }
    }
}
