; C locals.scm - Scope tracking for tree-sitter

; Scopes
(translation_unit) @local.scope
(function_definition) @local.scope
(compound_statement) @local.scope
(for_statement) @local.scope
(while_statement) @local.scope
(do_statement) @local.scope
(if_statement) @local.scope
(switch_statement) @local.scope

; Definitions
(declaration
  declarator: (identifier) @local.definition)

(declaration
  declarator: (pointer_declarator
    declarator: (identifier) @local.definition))

(declaration
  declarator: (array_declarator
    declarator: (identifier) @local.definition))

(function_definition
  declarator: (function_declarator
    declarator: (identifier) @local.definition))

(parameter_declaration
  declarator: (identifier) @local.definition)

(parameter_declaration
  declarator: (pointer_declarator
    declarator: (identifier) @local.definition))

(struct_specifier
  name: (type_identifier) @local.definition)

(enum_specifier
  name: (type_identifier) @local.definition)

(union_specifier
  name: (type_identifier) @local.definition)

(type_definition
  declarator: (type_identifier) @local.definition)

(for_statement
  initializer: (declaration
    declarator: (identifier) @local.definition))

; References
(identifier) @local.reference
(type_identifier) @local.reference
