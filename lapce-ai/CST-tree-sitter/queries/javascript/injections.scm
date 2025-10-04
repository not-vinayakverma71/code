; JavaScript injections.scm - Language injection for embedded code

; Regular expressions
(regex) @injection.content
(#set! injection.language "regex")

; Template strings can contain embedded expressions
(template_substitution) @injection.content
(#set! injection.language "javascript")

; JSDoc comments
((comment) @injection.content
 (#match? @injection.content "^/\\*\\*")
 (#set! injection.language "jsdoc"))

; JSON in template literals (common pattern)
((template_string) @injection.content
 (#match? @injection.content "^`[\\s]*\\{")
 (#set! injection.language "json"))

; SQL in tagged template literals
(call_expression
  function: (identifier) @_sql
  arguments: (template_string) @injection.content
  (#eq? @_sql "sql")
  (#set! injection.language "sql"))

; GraphQL in tagged template literals
(call_expression
  function: (identifier) @_graphql
  arguments: (template_string) @injection.content
  (#match? @_graphql "^(gql|graphql)$")
  (#set! injection.language "graphql"))
