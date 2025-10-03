; Scopes
(function_declaration) @scope
(function_definition) @scope
(class_declaration) @scope
(class_definition) @scope
(block) @scope

; Definitions
(parameter name: (identifier) @definition)
(variable_declaration name: (identifier) @definition)

; References
(identifier) @reference
