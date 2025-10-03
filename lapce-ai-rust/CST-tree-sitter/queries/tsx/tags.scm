; Classes
(class_declaration name: (identifier); Extracted from Codex tsx.ts
; This query captures React component definitions in TSX files:
; - Function Components
; - Class Components
; - Higher Order Components
; - Type Definitions
; - Props Interfaces
; - State Definitions
; - Generic Components

; NOTE: TSX includes all TypeScript queries PLUS React-specific patterns
; (TypeScript queries from typescript.ts are inherited)

(function_signature
  name: (identifier) @name.definition.function) @definition.function

(method_signature
  name: (property_identifier) @name.definition.method) @definition.method

(abstract_method_signature
  name: (property_identifier) @name.definition.method) @definition.method

(abstract_class_declaration
  name: (type_identifier) @name.definition.class) @definition.class

(module
  name: (identifier) @name.definition.module) @definition.module

(function_declaration
  name: (identifier) @name.definition.function) @definition.function

(method_definition
  name: (property_identifier) @name.definition.method) @definition.method

(class_declaration
  name: (type_identifier) @name.definition.class) @definition.class

(call_expression
  function: (identifier) @func_name
  arguments: (arguments
    (string) @name
    [(arrow_function) (function_expression)]) @definition.test)
  (#match? @func_name "^(describe|test|it)$")

(assignment_expression
  left: (member_expression
    object: (identifier) @obj
    property: (property_identifier) @prop)
  right: [(arrow_function) (function_expression)]) @definition.test
  (#eq? @obj "exports")
  (#eq? @prop "test")
(arrow_function) @definition.lambda

(switch_statement) @definition.switch
(switch_case) @definition.case
(switch_default) @definition.default

(enum_declaration
  name: (identifier) @name.definition.enum) @definition.enum

(export_statement
  decorator: (decorator
    (call_expression
      function: (identifier) @name.definition.decorator))
  declaration: (class_declaration
    name: (type_identifier) @name.definition.decorated_class)) @definition.decorated_class

(class_declaration
  name: (type_identifier) @name.definition.class) @definition.class

(internal_module
  name: (identifier) @name.definition.namespace) @definition.namespace

(interface_declaration
  name: (type_identifier) @name.definition.interface
  type_parameters: (type_parameters)?) @definition.interface

(type_alias_declaration
  name: (type_identifier) @name.definition.type
  type_parameters: (type_parameters)?) @definition.type

(type_alias_declaration
  name: (type_identifier) @name.definition.utility_type) @definition.utility_type

(public_field_definition
  name: (property_identifier) @name.definition.property) @definition.property

(method_definition
  name: (property_identifier) @name.definition.constructor
  (#eq? @name.definition.constructor "constructor")) @definition.constructor

(method_definition
  name: (property_identifier) @name.definition.accessor) @definition.accessor

(function_declaration
  name: (identifier) @name.definition.async_function) @definition.async_function

(variable_declaration
  (variable_declarator
    name: (identifier) @name.definition.async_arrow
    value: (arrow_function))) @definition.async_arrow

; Function Components - Both function declarations and arrow functions
(function_declaration
  name: (identifier) @name) @definition.component

; Arrow Function Components
(variable_declaration
  (variable_declarator
    name: (identifier) @name
    value: (arrow_function))) @definition.component

; Export Statement Components
(export_statement
  (variable_declaration
    (variable_declarator
      name: (identifier) @name
      value: (arrow_function)))) @definition.component

; Class Components
(class_declaration
  name: (type_identifier) @name) @definition.class_component

; Interface Declarations
(interface_declaration
  name: (type_identifier) @name) @definition.interface

; Type Alias Declarations
(type_alias_declaration
  name: (type_identifier) @name) @definition.type

; HOC Components
(variable_declaration
  (variable_declarator
    name: (identifier) @name
    value: (call_expression
      function: (identifier)))) @definition.component

; JSX Component Usage - Capture all components in JSX
(jsx_element
  open_tag: (jsx_opening_element
    name: [(identifier) @component (member_expression) @component])) @definition.jsx_element

; Self-closing JSX elements
(jsx_self_closing_element
  name: [(identifier) @component (member_expression) @component]) @definition.jsx_self_closing_element

; Capture all identifiers in JSX expressions that start with capital letters
(jsx_expression
  (identifier) @jsx_component) @definition.jsx_component

; Capture all member expressions in JSX
(member_expression
  object: (identifier) @object
  property: (property_identifier) @property) @definition.member_component

; Capture components in conditional expressions
(ternary_expression
  consequence: (parenthesized_expression
    (jsx_element
      open_tag: (jsx_opening_element
        name: (identifier) @component)))) @definition.conditional_component

(ternary_expression
  alternative: (jsx_self_closing_element
    name: (identifier) @component)) @definition.conditional_component

; Generic Components
(function_declaration
  name: (identifier) @name
  type_parameters: (type_parameters)) @definition.generic_component
