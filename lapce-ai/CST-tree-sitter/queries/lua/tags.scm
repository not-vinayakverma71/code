; Classes
(class_declaration name: (identifier) @name) @definition.class
(class_definition name: (identifier) @name) @definition.class

; Functions
(function_declaration name: (identifier); Extracted from Codex lua.ts
; Supported Lua structures:
; - function definitions (global, local, and method)
; - table constructors
; - variable declarations
; - class-like structures

; Function definitions
(function_definition_statement
  name: (identifier) @name.definition.function) @definition.function

(function_definition_statement
  name: (variable
    table: (identifier)
    field: (identifier) @name.definition.method)) @definition.method

(local_function_definition_statement
  name: (identifier) @name.definition.function) @definition.function

; Table constructors (class-like structures)
(local_variable_declaration
  (variable_list
    (variable name: (identifier) @name.definition.table))
  (expression_list
    value: (table))) @definition.table

; Variable declarations
(variable_assignment
  (variable_list
    (variable name: (identifier) @name.definition.variable))) @definition.variable

; Local variable declarations
(local_variable_declaration
  (variable_list
    (variable name: (identifier) @name.definition.variable))) @definition.variable

; Modules
(module_declaration name: (identifier) @name) @definition.module
