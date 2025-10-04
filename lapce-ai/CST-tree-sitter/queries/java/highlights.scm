; Keywords
[
  "if" "else" "elif" "then" "fi"
  "for" "while" "do" "done"
  "break" "continue" "return"
  "function" "def" "class" "struct"
  "import" "export" "module" "package"
  "const" "let" "var" "static"
  "public" "private" "protected"
  "async" "await" "yield"
  "try" "catch" "finally" "throw"
] @keyword

; Functions
(function_declaration name: (identifier) @function)
(function_definition name: (identifier) @function)
(method_definition name: (identifier) @function.method)
(call_expression function: (identifier) @function.call)
(method_declaration name: (identifier) @function.method)
(constructor_declaration name: (identifier) @constructor)
(method_invocation name: (identifier) @function.call)

; Classes and Types
(class_declaration
  name: (identifier) @type)
(interface_declaration
  name: (identifier) @type)
(enum_declaration
  name: (identifier) @type)
(type_identifier) @type

; Built-in types
((type_identifier) @type.builtin
 (#match? @type.builtin "^(boolean|byte|char|double|float|int|long|short|void)$"))

; Annotations
(annotation
  name: (identifier) @attribute)
(marker_annotation
  name: (identifier) @attribute)

; Constants
(true) @constant.builtin.boolean
(false) @constant.builtin.boolean
(null_literal) @constant.builtin

; Variables
(identifier) @variable
(field_access
  field: (identifier) @property)
(parameter) @variable.parameter
(formal_parameter name: (identifier) @variable.parameter)
(field_declaration declarator: (variable_declarator name: (identifier) @variable.field))

; Literals
(string_literal) @string
(character_literal) @string
(decimal_integer_literal) @number
(hex_integer_literal) @number
(octal_integer_literal) @number
(binary_integer_literal) @number
(decimal_floating_point_literal) @number
(hex_floating_point_literal) @number
(number_literal) @number

; Comments
(comment) @comment
(block_comment) @comment

; Operators
[
  "+" "-" "*" "/" "%"
  "==" "!=" "<" ">" "<=" ">="
  "&&" "||" "!"
  "=" "+=" "-=" "*=" "/="
  "&" "|" "^" "~" "<<" ">>"
] @operator

; Punctuation
[
  "(" ")" "[" "]" "{" "}"
  "," ";" ":" "."
  "->" "=>" "::"
] @punctuation
