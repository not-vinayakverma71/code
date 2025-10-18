; Rust locals.scm - Scope tracking for tree-sitter

; Scopes
(block) @local.scope
(function_item) @local.scope
(closure_expression) @local.scope
(while_expression) @local.scope
(for_expression) @local.scope
(loop_expression) @local.scope
(if_expression) @local.scope
(match_expression) @local.scope
(match_arm) @local.scope

; Definitions
(let_declaration pattern: (identifier) @local.definition)
(parameter pattern: (identifier) @local.definition)
(closure_parameters (identifier) @local.definition)
(for_expression pattern: (identifier) @local.definition)
(match_arm pattern: (identifier) @local.definition)
(function_item name: (identifier) @local.definition)
(struct_item name: (type_identifier) @local.definition)
(enum_item name: (type_identifier) @local.definition)
(const_item name: (identifier) @local.definition)
(static_item name: (identifier) @local.definition)

; References
(identifier) @local.reference
