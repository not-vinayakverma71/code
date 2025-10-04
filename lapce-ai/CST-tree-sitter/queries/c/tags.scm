; Extracted from Codex c.ts


; Function definitions and declarations
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @name.definition.function))

(declaration
  type: (_)?
  declarator: (function_declarator
    declarator: (identifier) @name.definition.function
    parameters: (parameter_list)?)?) @definition.function

(function_declarator
  declarator: (identifier) @name.definition.function
  parameters: (parameter_list)?) @definition.function

; Struct definitions
(struct_specifier
  name: (type_identifier) @name.definition.struct) @definition.struct

; Union definitions
(union_specifier
  name: (type_identifier) @name.definition.union) @definition.union

; Enum definitions
(enum_specifier
  name: (type_identifier) @name.definition.enum) @definition.enum

; Typedef declarations
(type_definition
  declarator: (type_identifier) @name.definition.type) @definition.type

; Global variables
(declaration
  (storage_class_specifier)?
  type: (_)
  declarator: (identifier) @name.definition.variable) @definition.variable

(declaration
  (storage_class_specifier)?
  type: (_)
  declarator: (init_declarator
    declarator: (identifier) @name.definition.variable)) @definition.variable

; Object-like macros
(preproc_def
  name: (identifier) @name.definition.macro) @definition.macro

; Function-like macros
(preproc_function_def
  name: (identifier) @name.definition.macro) @definition.macro
