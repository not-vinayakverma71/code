; Code blocks - class, module, method definitions
(directive
  (code) @name.definition.code) @definition.directive

; Output blocks - expressions
(output_directive
  (code) @output.content) @output

; Comments - documentation and section markers
(comment_directive
  (comment) @name.definition.comment) @definition.comment
