; Rust folds.scm - Code folding regions

; Function bodies
(function_item body: (_) @fold)

; Impl blocks
(impl_item body: (_) @fold)

; Trait definitions
(trait_item body: (_) @fold)

; Struct definitions
(struct_item body: (_) @fold)

; Enum definitions
(enum_item body: (_) @fold)

; Module definitions
(mod_item body: (_) @fold)

; Block expressions
(block) @fold

; Match expressions
(match_expression body: (_) @fold)

; Control flow
(if_expression consequence: (_) @fold)
(if_expression alternative: (_) @fold)
(for_expression body: (_) @fold)
(while_expression body: (_) @fold)
(loop_expression body: (_) @fold)

; Comments
(block_comment) @fold

; Use statement groups
(use_declaration) @fold
