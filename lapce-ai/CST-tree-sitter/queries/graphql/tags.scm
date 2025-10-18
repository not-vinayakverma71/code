; Classes
(class_declaration name: (identifier) @name) @definition.class
(class_definition name: (identifier) @name) @definition.class

; Functions
(function_declaration name: (identifier) @name) @definition.function
(function_definition name: (identifier) @name) @definition.function
(method_definition name: (identifier) @name) @definition.method

; Variables
(variable_declaration name: (identifier) @name) @definition.variable

; Modules
(module_declaration name: (identifier) @name) @definition.module
