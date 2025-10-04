; Go injections.scm - Language injection for embedded code

; Regular expressions
(call_expression
  function: (selector_expression
    field: (field_identifier) @_method)
  arguments: (argument_list
    (interpreted_string_literal) @injection.content)
  (#match? @_method "^(Compile|MustCompile|Match|MatchString|FindString|FindAllString|ReplaceAllString)$")
  (#set! injection.language "regex"))

; SQL queries (common pattern)
((interpreted_string_literal) @injection.content
 (#match? @injection.content "(?i)^`\\s*(SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)")
 (#set! injection.language "sql"))

; Template strings in html/template and text/template
(call_expression
  function: (selector_expression
    operand: (identifier) @_template
    field: (field_identifier) @_method)
  arguments: (argument_list
    (interpreted_string_literal) @injection.content)
  (#match? @_template "^(tmpl|template|tpl)$")
  (#match? @_method "^(Parse|ParseFiles|ParseGlob|New)$")
  (#set! injection.language "go-template"))

; JSON in json.Marshal/Unmarshal
((interpreted_string_literal) @injection.content
 (#match? @injection.content "^`\\s*\\{")
 (#set! injection.language "json"))

; Comments
(comment) @injection.content
(#set! injection.language "comment")
