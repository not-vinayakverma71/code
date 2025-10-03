; Java injections.scm - Language injection for embedded code

; Javadoc comments
((block_comment) @injection.content
 (#match? @injection.content "^/\\*\\*")
 (#set! injection.language "javadoc"))

; SQL in strings (common pattern)
((string_literal) @injection.content
 (#match? @injection.content "(?i)^\"\\s*(SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)")
 (#set! injection.language "sql"))

; Regular expressions in Pattern.compile
(method_invocation
  object: (identifier) @_pattern
  name: (identifier) @_method
  arguments: (argument_list
    (string_literal) @injection.content)
  (#eq? @_pattern "Pattern")
  (#match? @_method "^(compile|matches)$")
  (#set! injection.language "regex"))

; XML in strings (common in Java)
((string_literal) @injection.content
 (#match? @injection.content "^\"\\s*<\\?xml")
 (#set! injection.language "xml"))

; JSON in strings
((string_literal) @injection.content
 (#match? @injection.content "^\"\\s*\\{")
 (#set! injection.language "json"))
