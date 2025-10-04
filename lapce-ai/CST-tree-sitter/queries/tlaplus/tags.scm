; Extracted from Codex tlaplus.ts


; Module declarations
(module
  name: (identifier) @name.definition.module) @definition.module

; Operator definitions with optional parameters
(operator_definition
  name: (identifier) @name.definition.operator
  parameter: (identifier)?) @definition.operator

; Function definitions with bounds
(function_definition
  name: (identifier) @name.definition.function
  (quantifier_bound)?) @definition.function

; Variable declarations
(variable_declaration
  (identifier) @name.definition.variable) @definition.variable

; Constant declarations
(constant_declaration
  (identifier) @name.definition.constant) @definition.constant
