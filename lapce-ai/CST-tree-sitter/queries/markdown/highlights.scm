; Markdown syntax highlighting queries

; Headings
(atx_heading
  (atx_h1_marker) @markup.heading.1)
(atx_heading
  (atx_h2_marker) @markup.heading.2)
(atx_heading
  (atx_h3_marker) @markup.heading.3)
(atx_heading
  (atx_h4_marker) @markup.heading.4)
(atx_heading
  (atx_h5_marker) @markup.heading.5)
(atx_heading
  (atx_h6_marker) @markup.heading.6)

(setext_heading
  (heading_content) @markup.heading)

; Code
(code_span) @markup.raw.inline
(code_fence_content) @markup.raw.block
(fenced_code_block
  (info_string) @label)
(indented_code_block) @markup.raw.block

; Lists
(list_marker_plus) @punctuation.special
(list_marker_minus) @punctuation.special
(list_marker_star) @punctuation.special
(list_marker_dot) @punctuation.special
(list_marker_parenthesis) @punctuation.special

; Links and references
(link_text) @markup.link.text
(link_destination) @markup.link.url
(link_title) @string
(link_reference_definition
  (link_label) @markup.link.label)

; Images
(image_description) @markup.link.text
(image) @markup.link

; Emphasis
(emphasis) @markup.italic
(strong_emphasis) @markup.bold

; Blockquotes
(block_quote_marker) @punctuation.special
(block_quote) @markup.quote

; Horizontal rules
(thematic_break) @punctuation.delimiter

; HTML
(html_block) @text.html
(html_tag) @tag

; Tables
(pipe_table_header) @markup.heading
(pipe_table_delimiter_row) @punctuation.delimiter
(pipe_table_cell) @text
