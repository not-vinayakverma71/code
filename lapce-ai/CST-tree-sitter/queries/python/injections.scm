; Python injections.scm - Language injection for embedded code

; Docstrings are reStructuredText
((expression_statement
  (string)) @injection.content
 (#set! injection.language "rst"))

; F-strings contain Python expressions
(interpolation) @injection.content
(#set! injection.language "python")

; Regular expressions in re module calls
(call
  function: (attribute
    object: (identifier) @_re
    attribute: (identifier) @_method)
  arguments: (argument_list
    (string) @injection.content)
  (#eq? @_re "re")
  (#match? @_method "^(compile|search|match|fullmatch|findall|finditer|sub|subn|split)$")
  (#set! injection.language "regex"))

; SQL strings (common pattern)
((string) @injection.content
 (#match? @injection.content "(?i)^[\"']{1,3}\\s*(SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)")
 (#set! injection.language "sql"))

; JSON in json.loads/dumps calls
(call
  function: (attribute
    object: (identifier) @_json
    attribute: (identifier) @_method)
  arguments: (argument_list
    (string) @injection.content)
  (#eq? @_json "json")
  (#match? @_method "^(loads|load)$")
  (#set! injection.language "json"))
