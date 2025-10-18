; Java locals.scm - Scope tracking for tree-sitter

; Scopes
(program) @local.scope
(class_body) @local.scope
(interface_body) @local.scope
(enum_body) @local.scope
(constructor_body) @local.scope
(block) @local.scope
(method_declaration) @local.scope
(for_statement) @local.scope
(enhanced_for_statement) @local.scope
(while_statement) @local.scope
(do_statement) @local.scope
(if_statement) @local.scope
(switch_expression) @local.scope
(try_statement) @local.scope
(catch_clause) @local.scope
(finally_clause) @local.scope
(lambda_expression) @local.scope

; Definitions
(local_variable_declaration
  declarator: (variable_declarator
    name: (identifier) @local.definition))

(formal_parameter
  name: (identifier) @local.definition)

(catch_formal_parameter
  name: (identifier) @local.definition)

(enhanced_for_statement
  name: (identifier) @local.definition)

(method_declaration
  name: (identifier) @local.definition)

(constructor_declaration
  name: (identifier) @local.definition)

(class_declaration
  name: (identifier) @local.definition)

(interface_declaration
  name: (identifier) @local.definition)

(enum_declaration
  name: (identifier) @local.definition)

(field_declaration
  declarator: (variable_declarator
    name: (identifier) @local.definition))

(constant_declaration
  declarator: (variable_declarator
    name: (identifier) @local.definition))

; References
(identifier) @local.reference
(type_identifier) @local.reference
