; C injections.scm - Language injection for embedded code

; Inline assembly (GCC/Clang style)
(gnu_asm_expression
  (string_literal) @injection.content
  (#set! injection.language "asm"))

; SQL in strings (common pattern)
((string_literal) @injection.content
 (#match? @injection.content "(?i)^\"\\s*(SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)")
 (#set! injection.language "sql"))

; Regular expressions (when using regex.h)
(call_expression
  function: (identifier) @_func
  arguments: (argument_list
    (string_literal) @injection.content)
  (#match? @_func "^(regcomp|regexec)$")
  (#set! injection.language "regex"))

; Comments
(comment) @injection.content
(#set! injection.language "comment")
