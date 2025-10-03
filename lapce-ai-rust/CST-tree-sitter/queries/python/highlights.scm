; Python syntax highlighting queries for tree-sitter

; Keywords
[
  "and"
  "as"
  "assert"
  "async"
  "await"
  "break"
  "class"
  "continue"
  "def"
  "del"
  "elif"
  "else"
  "except"
  "finally"
  "for"
  "from"
  "global"
  "if"
  "import"
  "in"
  "is"
  "lambda"
  "nonlocal"
  "not"
  "or"
  "pass"
  "raise"
  "return"
  "try"
  "while"
  "with"
  "yield"
  "match"
  "case"
] @keyword

; Functions
(function_definition name: (identifier) @function)
(call function: (identifier) @function.call)
(call function: (attribute attribute: (identifier) @function.method.call))
(decorator) @function.decorator

; Classes
(class_definition name: (identifier) @type.class)


; Built-in functions
((identifier) @function.builtin
 (#match? @function.builtin "^(abs|all|any|ascii|bin|bool|breakpoint|bytearray|bytes|callable|chr|classmethod|compile|complex|delattr|dict|dir|divmod|enumerate|eval|exec|filter|float|format|frozenset|getattr|globals|hasattr|hash|help|hex|id|input|int|isinstance|issubclass|iter|len|list|locals|map|max|memoryview|min|next|object|oct|open|ord|pow|print|property|range|repr|reversed|round|set|setattr|slice|sorted|staticmethod|str|sum|super|tuple|type|vars|zip|__import__)$"))

; Constants
(none) @constant.builtin
(true) @constant.builtin.boolean
(false) @constant.builtin.boolean
(ellipsis) @constant.builtin

; Variables
(identifier) @variable
(attribute attribute: (identifier) @property)

; Literals
(string) @string
(concatenated_string) @string
(integer) @number
(float) @number

; Special variables
((identifier) @variable.builtin
 (#match? @variable.builtin "^(self|cls)$"))

; Comments
(comment) @comment

; Operators
[
  "+" "-" "*" "/" "//" "%" "**"
  "<<" ">>" "|" "^" "&" "@"
  "==" "!=" "<" ">" "<=" ">="
  "and" "or" "not" "in" "is"
  "=" "+=" "-=" "*=" "/=" "//=" "%=" "**="
  "<<=" ">>=" "&=" "|=" "^=" "@="
  "->" ":="
] @operator

; Punctuation
[
  ";" ","
] @punctuation.delimiter

[
  "(" ")" "[" "]" "{" "}"
] @punctuation.bracket

[
  ":"
] @punctuation.special
