; JavaScript syntax highlighting queries for tree-sitter

; Keywords
[
  "async"
  "await"
  "break"
  "case"
  "catch"
  "class"
  "const"
  "continue"
  "debugger"
  "default"
  "delete"
  "do"
  "else"
  "export"
  "extends"
  "finally"
  "for"
  "function"
  "get"
  "if"
  "import"
  "in"
  "instanceof"
  "let"
  "new"
  "of"
  "return"
  "set"
  "static"
  "switch"
  "throw"
  "try"
  "typeof"
  "var"
  "void"
  "while"
  "with"
  "yield"
] @keyword

; Function declarations
(function_declaration
  name: (identifier) @function)
(function
  name: (identifier) @function)
(generator_function_declaration
  name: (identifier) @function)
(method_definition
  name: (property_identifier) @function.method)
(arrow_function) @function
(call_expression
  function: (identifier) @function.call)

; Classes
(class_declaration
  name: (identifier) @type)
(class
  name: (identifier) @type)

; Variables and Properties
(identifier) @variable
(property_identifier) @property
(shorthand_property_identifier) @property

; Literals
(string) @string
(template_string) @string
(regex) @string.regex
(number) @number
(true) @constant.builtin.boolean
(false) @constant.builtin.boolean
(null) @constant.builtin
(undefined) @constant.builtin
(this) @variable.builtin
(super) @variable.builtin

; Comments
(comment) @comment

; Operators
[
  "="
  "+=" "-=" "*=" "/=" "%="
  "<<=" ">>=" ">>>="
  "&=" "|=" "^="
  "&&" "||" "??"
  "==" "!=" "===" "!=="
  "<" ">" "<=" ">="
  "+" "-" "*" "/" "%"
  "**"
  "++" "--"
  "!" "~"
  "..." "?."
] @operator

; Punctuation
[
  ";" ","
] @punctuation.delimiter

[
  "(" ")" "[" "]" "{" "}"
] @punctuation.bracket

[
  "=>"
] @punctuation.special
