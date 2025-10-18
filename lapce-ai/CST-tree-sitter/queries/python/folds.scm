; Python folds.scm - Code folding regions

; Function definitions
(function_definition body: (block) @fold)

; Class definitions
(class_definition body: (block) @fold)

; Control flow
(if_statement consequence: (block) @fold)
(if_statement alternative: (elif_clause) @fold)
(if_statement alternative: (else_clause) @fold)
(elif_clause consequence: (block) @fold)
(else_clause body: (block) @fold)
(for_statement body: (block) @fold)
(while_statement body: (block) @fold)
(with_statement body: (block) @fold)
(try_statement body: (block) @fold)
(except_clause body: (block) @fold)
(finally_clause body: (block) @fold)
(match_statement body: (block) @fold)
(case_clause consequence: (block) @fold)

; List comprehensions
(list_comprehension) @fold
(dictionary_comprehension) @fold
(set_comprehension) @fold

; Lists/Dictionaries/Sets (large ones)
(list) @fold
(dictionary) @fold
(set) @fold
(tuple) @fold

; Multiline strings (docstrings)
(string) @fold

; Comments (block comments don't exist in Python, but multiline strings used as comments)
(expression_statement (string)) @fold
