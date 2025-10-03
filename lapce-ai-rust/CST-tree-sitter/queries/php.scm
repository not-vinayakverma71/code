; Regular classes
(class_declaration
  name: (name) @name.definition.class) @definition.class

; Abstract classes
(class_declaration
  (abstract_modifier)
  name: (name) @name.definition.abstract_class) @definition.abstract_class

; Final classes
(class_declaration
  (final_modifier)
  name: (name) @name.definition.final_class) @definition.final_class

; Readonly classes (PHP 8.2+)
(class_declaration
  (readonly_modifier)
  name: (name) @name.definition.readonly_class) @definition.readonly_class

; Interfaces
(interface_declaration
  name: (name) @name.definition.interface) @definition.interface

; Traits
(trait_declaration
  name: (name) @name.definition.trait) @definition.trait

; Enums (PHP 8.1+)
(enum_declaration
  name: (name) @name.definition.enum) @definition.enum

; Global functions
(function_definition
  name: (name) @name.definition.function) @definition.function

; Regular methods
(method_declaration
  name: (name) @name.definition.method) @definition.method

; Static methods
(method_declaration
  (static_modifier)
  name: (name) @name.definition.static_method) @definition.static_method

; Abstract methods
(method_declaration
  (abstract_modifier)
  name: (name) @name.definition.abstract_method) @definition.abstract_method

; Final methods
(method_declaration
  (final_modifier)
  name: (name) @name.definition.final_method) @definition.final_method

; Arrow functions (PHP 7.4+)
(arrow_function) @definition.arrow_function

; Regular properties
(property_declaration
  (property_element
    (variable_name
      (name) @name.definition.property))) @definition.property

; Static properties
(property_declaration
  (static_modifier)
  (property_element
    (variable_name
      (name) @name.definition.static_property))) @definition.static_property

; Readonly properties (PHP 8.1+)
(property_declaration
  (readonly_modifier)
  (property_element
    (variable_name
      (name) @name.definition.readonly_property))) @definition.readonly_property

; Constructor property promotion (PHP 8.0+)
(property_promotion_parameter
  name: (variable_name
    (name) @name.definition.promoted_property)) @definition.promoted_property

; Constants
(const_declaration
  (const_element
    (name) @name.definition.constant)) @definition.constant

; Namespaces
(namespace_definition
  name: (namespace_name) @name.definition.namespace) @definition.namespace

; Use statements (imports)
(namespace_use_declaration
  (namespace_use_clause
    (qualified_name) @name.definition.use)) @definition.use

; Anonymous classes
(object_creation_expression
  (declaration_list)) @definition.anonymous_class

; Attributes (PHP 8.0+)
(attribute_group
  (attribute
    (name) @name.definition.attribute)) @definition.attribute

; Match expressions (PHP 8.0+)
(match_expression) @definition.match_expression

; Heredoc syntax
(heredoc) @definition.heredoc

; Nowdoc syntax
(nowdoc) @definition.nowdoc
