; TypeScript locals.scm - Scope tracking for tree-sitter

; Scopes
(statement_block) @local.scope
(function_declaration) @local.scope
(function_expression) @local.scope
(arrow_function) @local.scope
(method_definition) @local.scope
(for_statement) @local.scope
(for_in_statement) @local.scope
(while_statement) @local.scope
(do_statement) @local.scope
(if_statement) @local.scope
(switch_statement) @local.scope
(try_statement) @local.scope
(catch_clause) @local.scope
(class_declaration) @local.scope
(interface_declaration) @local.scope
(enum_declaration) @local.scope
(module) @local.scope
(namespace_statement) @local.scope

; Definitions
(variable_declarator
  name: (identifier) @local.definition)
(function_declaration
  name: (identifier) @local.definition)
(class_declaration
  name: (identifier) @local.definition)
(interface_declaration
  name: (type_identifier) @local.definition)
(enum_declaration
  name: (identifier) @local.definition)
(type_alias_declaration
  name: (type_identifier) @local.definition)
(formal_parameters
  (required_parameter pattern: (identifier) @local.definition))
(formal_parameters
  (optional_parameter pattern: (identifier) @local.definition))
(catch_clause
  parameter: (identifier) @local.definition)

; Type parameters
(type_parameter name: (type_identifier) @local.definition)

; References
(identifier) @local.reference
(type_identifier) @local.reference
