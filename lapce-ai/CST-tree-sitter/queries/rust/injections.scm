; Rust injections.scm - Language injection for embedded code

; Documentation comments use Markdown
((line_comment) @injection.content
 (#match? @injection.content "^//[/!]")
 (#set! injection.language "markdown"))

((block_comment) @injection.content
 (#match? @injection.content "^/\\*[\\*!]")
 (#set! injection.language "markdown"))

; Regex in raw strings (common pattern)
((raw_string_literal) @injection.content
 (#match? @injection.content "^r#*\"")
 (#set! injection.language "regex"))

; SQL in raw strings (when prefixed with sql)
((macro_invocation
  macro: (identifier) @_sql_macro
  (token_tree
    (raw_string_literal) @injection.content))
 (#eq? @_sql_macro "sql")
 (#set! injection.language "sql"))

; Format strings
((macro_invocation
  macro: (identifier) @_format_macro
  (token_tree
    (string_literal) @injection.content))
 (#match? @_format_macro "^(format|print|println|eprint|eprintln|write|writeln)$")
 (#set! injection.language "rust-format"))
