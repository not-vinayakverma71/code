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

; Types
(type_identifier) @type
(primitive_type) @type.builtin

; Variables
(identifier) @variable
(parameter) @variable.parameter

; Literals
(string_literal) @string
(number_literal) @number
(boolean_literal) @constant.builtin

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
