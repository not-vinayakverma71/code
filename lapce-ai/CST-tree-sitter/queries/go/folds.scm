; Go folds.scm - Code folding regions

; Function bodies
(function_declaration body: (_) @fold)
(method_declaration body: (_) @fold)
(func_literal body: (_) @fold)

; Type declarations
(type_declaration) @fold

; Struct definitions
(struct_type body: (_) @fold)

; Interface definitions
(interface_type body: (_) @fold)

; Control flow
(if_statement consequence: (_) @fold)
(if_statement alternative: (_) @fold)
(for_statement body: (_) @fold)
(expression_switch_statement body: (_) @fold)
(type_switch_statement body: (_) @fold)
(select_statement body: (_) @fold)
(expression_case body: (_) @fold)
(type_case body: (_) @fold)
(communication_case body: (_) @fold)
(default_case body: (_) @fold)

; Blocks
(block) @fold

; Composite literals
(composite_literal) @fold

; Import groups
(import_declaration) @fold

; Comments
(comment) @fold
