; Rust Keywords - Only actual tokens that exist in the grammar
"async" @keyword
"await" @keyword
"break" @keyword
"const" @keyword
"continue" @keyword
"else" @keyword
"enum" @keyword
"extern" @keyword
"fn" @keyword
"for" @keyword
"if" @keyword
"impl" @keyword
"in" @keyword
"let" @keyword
"loop" @keyword
"match" @keyword
"mod" @keyword
"move" @keyword
"pub" @keyword
"ref" @keyword
"return" @keyword
"static" @keyword
"struct" @keyword
"trait" @keyword
"type" @keyword
"unsafe" @keyword
"use" @keyword
"where" @keyword
"while" @keyword

; Functions
(function_item name: (identifier) @function)
(function_signature_item name: (identifier) @function) 
(macro_definition name: (identifier) @function.macro)
(call_expression function: (identifier) @function.call)
(call_expression function: (field_expression field: (field_identifier) @function.method.call))
(macro_invocation macro: (identifier) @function.macro)

; Types
(type_identifier) @type
(primitive_type) @type.builtin
(generic_type type: (type_identifier) @type)
(scoped_type_identifier name: (type_identifier) @type)
(struct_item name: (type_identifier) @type)
(enum_item name: (type_identifier) @type)
(trait_item name: (type_identifier) @type)

; Variables and Fields
(field_identifier) @property
(shorthand_field_initializer) @variable
(self) @variable.builtin

; Literals
(string_literal) @string
(char_literal) @string
(integer_literal) @number
(float_literal) @number
(boolean_literal) @constant.builtin

; Comments
(line_comment) @comment
(block_comment) @comment

; Operators
[
  "!" "!=" "%" "%=" "&" "&&" "&=" "*" "*=" 
  "+" "+=" "-" "-=" "->" "." ".." "..="
  "/" "/=" ":" "::" "<" "<<" "<<=" "<=" 
  "=" "==" "=>" ">" ">=" ">>" ">>=" "?"
  "@" "^" "^=" "|" "|=" "||" "~"
] @operator

; Punctuation
[
  "(" ")" "[" "]" "{" "}"
] @punctuation.bracket
[
  ";" ","
] @punctuation.delimiter

; Attributes
(attribute_item) @attribute
(inner_attribute_item) @attribute
