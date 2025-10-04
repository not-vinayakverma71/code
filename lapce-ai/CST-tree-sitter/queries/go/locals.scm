; Go locals.scm - Scope tracking for tree-sitter

; Scopes
(source_file) @local.scope
(function_declaration body: (_) @local.scope)
(method_declaration body: (_) @local.scope)
(func_literal body: (_) @local.scope)
(block) @local.scope
(for_statement body: (_) @local.scope)
(if_statement consequence: (_) @local.scope)
(if_statement alternative: (_) @local.scope)
(expression_switch_statement body: (_) @local.scope)
(type_switch_statement body: (_) @local.scope)
(select_statement body: (_) @local.scope)

; Definitions
(short_var_declaration
  left: (identifier_list
    (identifier) @local.definition))

(var_declaration
  (var_spec
    name: (identifier) @local.definition))

(const_declaration
  (const_spec
    name: (identifier) @local.definition))

(function_declaration
  name: (identifier) @local.definition)

(method_declaration
  name: (field_identifier) @local.definition)

(type_declaration
  (type_spec
    name: (type_identifier) @local.definition))

(parameter_declaration
  (identifier) @local.definition)

(variadic_parameter_declaration
  (identifier) @local.definition)

(for_statement
  (range_clause
    left: (identifier_list
      (identifier) @local.definition)))

(type_switch_statement
  alias: (identifier) @local.definition)

; References
(identifier) @local.reference
(type_identifier) @local.reference
