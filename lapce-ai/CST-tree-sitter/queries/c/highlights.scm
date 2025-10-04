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
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @function))

(call_expression
  function: (identifier) @function.call)

; Types
(primitive_type) @type.builtin
(type_identifier) @type
(struct_specifier name: (type_identifier) @type.struct)
(enum_specifier name: (type_identifier) @type.enum)
(union_specifier name: (type_identifier) @type.union)

; Variables
(declaration

; Literals
(string_literal) @string
(system_lib_string) @string
(char_literal) @string
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
