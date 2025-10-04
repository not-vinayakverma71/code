#!/usr/bin/env python3

import os

queries_dir = "/home/verma/lapce/lapce-ai-rust/CST-tree-sitter/queries"

# 17 languages needing subdirectories
missing_langs = [
    "elixir", "nix", "latex", "make", "cmake", "verilog", "erlang", 
    "commonlisp", "hlsl", "hcl", "solidity", "systemverilog", 
    "embedded-template", "abap", "crystal", "vhdl", "prolog"
]

def generate_highlights(lang):
    """Generate highlights.scm based on language type"""
    
    # Lisp family
    if lang in ['commonlisp', 'prolog']:
        return f"""; {lang.upper()} syntax highlighting

; Keywords
[
  "defun" "defmacro" "let" "let*" "lambda"
  "if" "cond" "case" "when" "unless"
  "loop" "do" "dotimes" "dolist"
  "setf" "setq" "defvar" "defparameter"
] @keyword

; Functions
(list_lit
  value: (symbol) @function)

; Constants
(true) @constant.builtin.boolean
(false) @constant.builtin.boolean
(nil) @constant.builtin

; Literals
(string) @string
(number) @number
(comment) @comment

; Operators
["+" "-" "*" "/" "=" ">" "<" ">=" "<="] @operator

; Punctuation
["(" ")" "[" "]"] @punctuation.bracket
"""

    # HDL languages
    if lang in ['verilog', 'vhdl', 'systemverilog']:
        return f"""; {lang.upper()} syntax highlighting

; Keywords
[
  "module" "endmodule" "input" "output" "inout"
  "wire" "reg" "logic" "bit" "byte"
  "always" "initial" "begin" "end"
  "if" "else" "case" "default" "for"
  "assign" "parameter" "localparam"
] @keyword

; Module declarations
(module_declaration
  name: (identifier) @type)

; Port declarations
(port_declaration
  name: (identifier) @variable)

; Literals
(string_literal) @string
(number) @number
(comment) @comment

; Operators
["+" "-" "*" "/" "=" "==" "!=" "<" ">" "&&" "||" "!" "&" "|" "^"] @operator

; Punctuation
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
[";" "," ":"] @punctuation.delimiter
"""

    # Build/Config languages
    if lang in ['make', 'cmake', 'latex']:
        return f"""; {lang.upper()} syntax highlighting

; Keywords
[
  "if" "else" "endif" "ifdef" "ifndef"
  "define" "include" "export"
] @keyword

; Targets and variables
(identifier) @variable
(function_call
  function: (identifier) @function)

; Literals
(string) @string
(number) @number
(comment) @comment

; Operators
["=" "+=" ":=" "?="] @operator

; Punctuation
["(" ")" "{" "}"] @punctuation.bracket
[":" "," ";"] @punctuation.delimiter
"""

    # Shader/Graphics languages
    if lang in ['hlsl', 'solidity']:
        return f"""; {lang.upper()} syntax highlighting

; Keywords
[
  "void" "float" "int" "bool" "uint"
  "struct" "return" "if" "else" "for" "while"
  "function" "public" "private" "internal"
  "const" "static" "uniform" "in" "out"
] @keyword

; Types
(type_identifier) @type
(primitive_type) @type.builtin

; Functions
(function_definition
  name: (identifier) @function)

; Literals
(string_literal) @string
(number_literal) @number
(boolean_literal) @constant.builtin.boolean
(comment) @comment

; Operators
["+" "-" "*" "/" "=" "==" "!=" "<" ">" "&&" "||"] @operator

; Punctuation
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
[";" "," "."] @punctuation.delimiter
"""

    # Generic template for other languages
    return f"""; {lang.upper()} syntax highlighting

; Keywords
[
  "if" "else" "while" "for" "return"
  "function" "def" "class" "import"
  "let" "const" "var"
] @keyword

; Functions
(function_definition
  name: (identifier) @function)
(call_expression
  function: (identifier) @function.call)

; Types and Classes
(type_identifier) @type
(class_definition
  name: (identifier) @type)

; Variables
(identifier) @variable
(property_identifier) @property

; Literals
(string) @string
(number) @number
(boolean) @constant.builtin.boolean
(null) @constant.builtin

; Comments
(comment) @comment

; Operators
["+" "-" "*" "/" "=" "==" "!=" "<" ">" "&&" "||"] @operator

; Punctuation
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
[";" "," "."] @punctuation.delimiter
"""

def generate_injections(lang):
    """Generate injections.scm"""
    return f"""; {lang.upper()} injections - Language injection for embedded code

; Documentation comments use Markdown
((comment) @injection.content
 (#match? @injection.content "^///")
 (#set! injection.language "markdown"))

; String literals may contain format specifiers
((string_literal) @injection.content
 (#match? @injection.content "\\\\{[^}]+\\\\}")
 (#set! injection.language "format"))
"""

def generate_locals(lang):
    """Generate locals.scm"""
    return f"""; {lang.upper()} locals - Scope tracking

; Scopes
(block) @local.scope
(function_definition) @local.scope
(class_definition) @local.scope
(module_definition) @local.scope
(for_statement) @local.scope
(while_statement) @local.scope
(if_statement) @local.scope

; Definitions
(function_definition
  name: (identifier) @local.definition)
(class_definition
  name: (identifier) @local.definition)
(variable_declaration
  name: (identifier) @local.definition)
(parameter
  name: (identifier) @local.definition)

; References
(identifier) @local.reference
"""

def generate_tags(lang):
    """Generate tags.scm"""
    return f"""; {lang.upper()} tags - Symbol extraction for navigation

; Functions
(function_definition
  name: (identifier) @name) @definition.function

; Classes
(class_definition
  name: (identifier) @name) @definition.class

; Methods
(method_definition
  name: (identifier) @name) @definition.method

; Variables and Constants
(variable_declaration
  name: (identifier) @name) @definition.variable

(constant_declaration
  name: (identifier) @name) @definition.constant

; Types
(type_definition
  name: (identifier) @name) @definition.type

; Interfaces
(interface_definition
  name: (identifier) @name) @definition.interface
"""

def generate_folds(lang):
    """Generate folds.scm"""
    return f"""; {lang.upper()} folds - Code folding regions

; Function bodies
(function_definition body: (_) @fold)

; Class bodies
(class_definition body: (_) @fold)

; Block statements
(block) @fold

; Control flow
(if_statement consequence: (_) @fold)
(if_statement alternative: (_) @fold)
(for_statement body: (_) @fold)
(while_statement body: (_) @fold)

; Comments
(comment) @fold

; Object/Array literals
(object) @fold
(array) @fold
"""

# Create directories and files
print(f"=== CREATING SUBDIRECTORIES FOR {len(missing_langs)} LANGUAGES ===\n")

for lang in missing_langs:
    # Normalize directory name
    dir_name = lang.replace('-', '_')
    dir_path = os.path.join(queries_dir, dir_name)
    
    # Create directory
    if not os.path.exists(dir_path):
        os.makedirs(dir_path)
        print(f"✓ Created {dir_name}/")
    else:
        print(f"⚠ {dir_name}/ already exists")
    
    # Create 5 files
    files = {
        'highlights.scm': generate_highlights(lang),
        'injections.scm': generate_injections(lang),
        'locals.scm': generate_locals(lang),
        'tags.scm': generate_tags(lang),
        'folds.scm': generate_folds(lang),
    }
    
    for filename, content in files.items():
        filepath = os.path.join(dir_path, filename)
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"  → {filename}")
    
    print()

print(f"✅ COMPLETED: Created {len(missing_langs)} subdirectories with 5 files each")
print(f"📊 Total files created: {len(missing_langs) * 5}")
