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

; Types
(primitive_type) @type.builtin
(type_identifier) @type
(auto) @type.builtin

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
