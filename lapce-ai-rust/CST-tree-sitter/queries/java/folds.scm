; Java folds.scm - Code folding regions

; Class bodies
(class_body) @fold
(interface_body) @fold
(enum_body) @fold
(annotation_type_body) @fold

; Method bodies
(constructor_body) @fold
(block) @fold

; Control flow
(if_statement consequence: (_) @fold)
(if_statement alternative: (_) @fold)
(for_statement body: (_) @fold)
(enhanced_for_statement body: (_) @fold)
(while_statement body: (_) @fold)
(do_statement body: (_) @fold)
(switch_expression body: (_) @fold)
(switch_block) @fold
(try_statement body: (_) @fold)
(catch_clause body: (_) @fold)
(finally_clause body: (_) @fold)

; Lambda expressions
(lambda_expression body: (_) @fold)

; Array initializers
(array_initializer) @fold

; Comments
(block_comment) @fold

; Import groups
(import_declaration) @fold
