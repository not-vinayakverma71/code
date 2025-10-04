; Python tags.scm - Symbol extraction; Extracted from Codex python.ts
; Python Tree-sitter Query Patterns

; Class definitions (including decorated)
(class_definition
  name: (identifier) @name.definition.class) @definition.class

(decorated_definition
  definition: (class_definition
    name: (identifier) @name.definition.class)) @definition.class

; Function and method definitions (including async and decorated)
(function_definition
  name: (identifier) @name.definition.function) @definition.function

(decorated_definition
  definition: (function_definition
    name: (identifier) @name.definition.function)) @definition.function

; Lambda expressions
(expression_statement
  (assignment
    left: (identifier) @name.definition.lambda
    right: (parenthesized_expression
      (lambda)))) @definition.lambda

; Generator functions (functions containing yield)
(function_definition
  name: (identifier) @name.definition.generator
  body: (block
    (expression_statement
      (yield)))) @definition.generator

; Comprehensions
(expression_statement
  (assignment
    left: (identifier) @name.definition.comprehension
    right: [
      (list_comprehension)
      (dictionary_comprehension)
      (set_comprehension)
    ])) @definition.comprehension

; With statements
(with_statement) @definition.with_statement

; Try statements
(try_statement) @definition.try_statement

; Import statements
(import_from_statement) @definition.import
(import_statement) @definition.import

; Global/Nonlocal statements
(function_definition
  body: (block
    [(global_statement) (nonlocal_statement)])) @definition.scope

; Match case statements
(function_definition
  body: (block
    (match_statement))) @definition.match_case

; Type annotations
(typed_parameter
  type: (type)) @definition.type_annotation

(expression_statement
  (assignment
    left: (identifier) @name.definition.type
    type: (type))) @definition.type_annotation

; Methods (functions inside classes)
(class_definition
  body: (block
    (function_definition
      name: (identifier) @name.definition.method) @definition.method))

(class_definition
  body: (block
    (decorated_definition
      definition: (function_definition
        name: (identifier) @name)) @definition.method))

; Variables/Constants
(assignment
  left: (identifier) @name) @definition.variable

; Global variables
(expression_statement
  (assignment
    left: (identifier) @name)) @definition.variable

; Import aliases
(import_statement
  name: (aliased_import
    alias: (identifier) @name)) @definition.module

(import_from_statement
  name: (aliased_import
    alias: (identifier) @name)) @definition.variable
