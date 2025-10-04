; Python locals.scm - Scope tracking for tree-sitter

; Scopes
(module) @local.scope
(function_definition) @local.scope
(class_definition) @local.scope
(for_statement) @local.scope
(while_statement) @local.scope
(with_statement) @local.scope
(if_statement) @local.scope
(elif_clause) @local.scope
(else_clause) @local.scope
(try_statement) @local.scope
(except_clause) @local.scope
(finally_clause) @local.scope
(match_statement) @local.scope
(case_clause) @local.scope

; Definitions
(function_definition name: (identifier) @local.definition)
(class_definition name: (identifier) @local.definition)
(assignment left: (identifier) @local.definition)
(assignment left: (pattern_list (identifier) @local.definition))
(assignment left: (tuple_pattern (identifier) @local.definition))
(assignment left: (list_pattern (identifier) @local.definition))
(for_statement left: (identifier) @local.definition)
(for_statement left: (pattern_list (identifier) @local.definition))
(for_statement left: (tuple_pattern (identifier) @local.definition))
(parameters (identifier) @local.definition)
(parameters (default_parameter name: (identifier) @local.definition))
(parameters (typed_parameter (identifier) @local.definition))
(parameters (typed_default_parameter (identifier) @local.definition))
(parameters (list_splat_pattern (identifier) @local.definition))
(parameters (dictionary_splat_pattern (identifier) @local.definition))
(with_item value: (identifier) @local.definition)
(with_item value: (as_pattern alias: (as_pattern_target (identifier) @local.definition)))
(except_clause (as_pattern alias: (as_pattern_target (identifier) @local.definition)))
(import_statement name: (aliased_import alias: (identifier) @local.definition))
(import_from_statement name: (aliased_import alias: (identifier) @local.definition))

; References
(identifier) @local.reference
