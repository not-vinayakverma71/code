; TypeScript syntax highlighting queries for tree-sitter

; TypeScript Keywords (includes all JS keywords plus TS-specific)
[
  "abstract"
  "as"
  "async"
  "await"
  "break"
  "case"
  "catch"
  "class"
  "const"
  "continue"
  "debugger"
  "declare"
  "default"
  "delete"
  "do"
  "else"
  "enum"
  "export"
  "extends"
  "finally"
  "for"
  "function"
  "get"
  "if"
  "implements"
  "import"
  "in"
  "infer"
  "instanceof"
  "interface"
  "is"
  "keyof"
  "let"
  "module"
  "namespace"
  "new"
  "of"
  "private"
  "protected"
  "public"
  "readonly"
  "return"
  "satisfies"
  "set"
  "static"
  "switch"
  "throw"
  "try"
  "type"
  "typeof"
  "var"
  "void"
  "while"
  "with"
  "yield"
] @keyword

; Types
(type_annotation) @type
(type_identifier) @type
(predefined_type) @type.builtin
(type_predicate) @type
(type_parameter) @type.parameter

; Interface and Type declarations
(interface_declaration
  name: (type_identifier) @type)
(type_alias_declaration
  name: (type_identifier) @type)
(enum_declaration
  name: (identifier) @type.enum)

; Functions
(function_declaration
  name: (identifier) @function)
(method_definition
  name: (property_identifier) @function.method)
(call_expression
  function: (identifier) @function.call)
(arrow_function) @function

; Decorators
(decorator) @attribute

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
(block_comment) @comment

; Operators
[
  "="
  "+=" "-=" "*=" "/=" "%="
  "<<=" ">>=" ">>>="
  "&=" "|=" "^="
  "&&" "||" "??" "!"
  "==" "!=" "===" "!=="
  "<" ">" "<=" ">="
  "+" "-" "*" "/" "%"
  "**" "++" "--"
  "~" "..." "?."
  "as" "in" "instanceof" "typeof"
] @operator

; Punctuation
[
  ";" ","
] @punctuation.delimiter

[
  "(" ")" "[" "]" "{" "}"
] @punctuation.bracket

[
  "=>" "::" "?:"
] @punctuation.special
