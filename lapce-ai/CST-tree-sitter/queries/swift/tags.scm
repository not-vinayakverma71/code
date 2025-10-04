; Extracted from Codex swift.ts


; Class declarations - captures standard, final, and open classes
(class_declaration
  name: (type_identifier) @name) @definition.class

; Protocol declarations - captures protocols with requirements
(protocol_declaration
  name: (type_identifier) @name) @definition.interface

; Method declarations in classes/structs/enums/extensions
(function_declaration
  name: (simple_identifier) @name) @definition.method

; Static/class method declarations
(function_declaration
  (modifiers
    (property_modifier))
  name: (simple_identifier) @name) @definition.static_method

; Initializers - captures designated initializers
(init_declaration
  "init" @name) @definition.initializer

; Convenience initializers
(init_declaration
  (modifiers (member_modifier))
  "init" @name) @definition.convenience_initializer

; Deinitializers
(deinit_declaration
  "deinit" @name) @definition.deinitializer

; Subscript declarations
(subscript_declaration
  (parameter) @name) @definition.subscript

; Property declarations - captures stored properties
(property_declaration
  (pattern) @name) @definition.property

; Computed property declarations with accessors
(property_declaration
  (pattern)
  (computed_property)) @definition.computed_property

; Type aliases
(typealias_declaration
  name: (type_identifier) @name) @definition.type_alias

; Protocol property requirements
(protocol_property_declaration
  name: (pattern) @name) @definition.protocol_property

; Protocol method requirements
(protocol_function_declaration
  name: (simple_identifier) @name) @definition.protocol_method
