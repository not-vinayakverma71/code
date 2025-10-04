; Inject other languages in strings/comments
((string) @injection.content
 (#match? @injection.content "^[\"\`]")
 (#set! injection.language "regex"))

((comment) @injection.content
 (#match? @injection.content "^//")
 (#set! injection.language "comment"))
