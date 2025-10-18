; Go syntax highlighting queries

; Keywords
[
  "break"
  "case"
  "chan"
  "const"
  "continue"
  "default"
  "defer"
  "else"
  "fallthrough"
  "for"
  "func"
  "go"
  "goto"
  "if"
  "import"
  "interface"
  "map"
  "package"
  "range"
  "return"
  "select"
  "struct"
  "switch"
  "type"
  "var"
] @keyword

; Functions
(function_declaration
  name: (identifier) @function)

(method_declaration
  name: (field_identifier) @function.method)

(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (selector_expression
    field: (field_identifier) @function.method))

; Keywords
"func" @keyword.function
"return" @keyword.control.return
["import" "package"] @keyword.control.import
["if" "else"] @keyword.control.conditional
["for" "range"] @keyword.control.repeat
["switch" "case" "default" "fallthrough"] @keyword.control.conditional
["break" "continue"] @keyword.control
["type" "struct" "interface"] @keyword.type
["var" "const"] @keyword.storage
["defer" "go" "goto"] @keyword.control
["select" "chan"] @keyword.control
"map" @keyword.type
  name: (identifier) @constant)

; Strings
(interpreted_string_literal) @string
(raw_string_literal) @string
(rune_literal) @string

(int_literal) @number
(float_literal) @number
(imaginary_literal) @number

; Comments
(comment) @comment

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "<<"
  ">>"
  "&"
  "&^"
  "|"
  "^"
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  "&&"
  "||"
  "!"
  "="
  "+="
  "-="
  "*="
  "/="
  "%="
  "&="
  "|="
  "^="
  "<<="
  ">>="
  "&^="
  ":="
  "<-"
  "++"
  "--"
  "..."
] @operator

; Punctuation
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  "."
  ":"
  ";"
] @punctuation.delimiter
