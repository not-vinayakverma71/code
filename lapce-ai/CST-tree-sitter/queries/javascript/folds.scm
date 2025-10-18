; JavaScript folds.scm - Code folding regions

; Function bodies
(function_declaration body: (_) @fold)
(function_expression body: (_) @fold)
(arrow_function body: (_) @fold)
(generator_function_declaration body: (_) @fold)

; Method definitions
(method_definition body: (_) @fold)

; Class bodies
(class_declaration body: (_) @fold)
(class body: (_) @fold)

; Control flow
(if_statement consequence: (_) @fold)
(if_statement alternative: (_) @fold)
(for_statement body: (_) @fold)
(for_in_statement body: (_) @fold)
(while_statement body: (_) @fold)
(do_statement body: (_) @fold)
(switch_statement body: (_) @fold)
(try_statement body: (_) @fold)
(catch_clause body: (_) @fold)

; Block statements
(statement_block) @fold

; Object literals
(object) @fold

; Array literals (large arrays)
(array) @fold

; Comments
(comment) @fold
