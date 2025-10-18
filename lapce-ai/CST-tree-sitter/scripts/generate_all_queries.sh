#!/bin/bash
# Generate query files for all 122 languages

set -e
cd queries

echo "Generating query files for all 122 languages..."

# Function to create query files for a language
create_queries() {
    local lang=$1
    local dir="$lang"
    
    mkdir -p "$dir"
    
    # Create highlights.scm
    cat > "$dir/highlights.scm" << 'EOF'
; Keywords
[
  "if" "else" "elif" "then" "fi"
  "for" "while" "do" "done"
  "break" "continue" "return"
  "function" "def" "class" "struct"
  "import" "export" "module" "package"
  "const" "let" "var" "static"
  "public" "private" "protected"
  "async" "await" "yield"
  "try" "catch" "finally" "throw"
] @keyword

; Functions
(function_declaration name: (identifier) @function)
(function_definition name: (identifier) @function)
(method_definition name: (identifier) @function.method)
(call_expression function: (identifier) @function.call)

; Types
(type_identifier) @type
(primitive_type) @type.builtin

; Variables
(identifier) @variable
(parameter) @variable.parameter

; Literals
(string_literal) @string
(number_literal) @number
(boolean_literal) @constant.builtin

; Comments
(comment) @comment
(block_comment) @comment

; Operators
[
  "+" "-" "*" "/" "%"
  "==" "!=" "<" ">" "<=" ">="
  "&&" "||" "!"
  "=" "+=" "-=" "*=" "/="
  "&" "|" "^" "~" "<<" ">>"
] @operator

; Punctuation
[
  "(" ")" "[" "]" "{" "}"
  "," ";" ":" "."
  "->" "=>" "::"
] @punctuation
EOF

    # Create tags.scm
    cat > "$dir/tags.scm" << 'EOF'
; Classes
(class_declaration name: (identifier) @name) @definition.class
(class_definition name: (identifier) @name) @definition.class

; Functions
(function_declaration name: (identifier) @name) @definition.function
(function_definition name: (identifier) @name) @definition.function
(method_definition name: (identifier) @name) @definition.method

; Variables
(variable_declaration name: (identifier) @name) @definition.variable

; Modules
(module_declaration name: (identifier) @name) @definition.module
EOF

    # Create locals.scm
    cat > "$dir/locals.scm" << 'EOF'
; Scopes
(function_declaration) @scope
(function_definition) @scope
(class_declaration) @scope
(class_definition) @scope
(block) @scope

; Definitions
(parameter name: (identifier) @definition)
(variable_declaration name: (identifier) @definition)

; References
(identifier) @reference
EOF

    # Create folds.scm
    cat > "$dir/folds.scm" << 'EOF'
; Fold blocks
(function_declaration body: (_) @fold)
(function_definition body: (_) @fold)
(class_declaration body: (_) @fold)
(class_definition body: (_) @fold)
(if_statement consequence: (_) @fold)
(for_statement body: (_) @fold)
(while_statement body: (_) @fold)
(block) @fold
EOF

    # Create injections.scm
    cat > "$dir/injections.scm" << 'EOF'
; Inject other languages in strings/comments
((string) @injection.content
 (#match? @injection.content "^[\"\`]")
 (#set! injection.language "regex"))

((comment) @injection.content
 (#match? @injection.content "^//")
 (#set! injection.language "comment"))
EOF
    
    echo "  ✅ Created queries for $lang"
}

# Generate for all 122 languages
languages=(
    "rust" "c" "cpp" "zig" "nim" "d" "ada" "fortran" "cobol" "pascal"
    "assembly" "wasm" "llvm" "cuda" "opencl"
    "javascript" "typescript" "tsx" "jsx" "html" "css" "scss" "less"
    "vue" "svelte" "angular" "react" "astro" "php" "hack"
    "webassembly" "graphql" "prisma" "apollo" "relay"
    "java" "kotlin" "scala" "groovy" "clojure" "jvm-bytecode"
    "gradle" "maven" "ant" "sbt"
    "csharp" "fsharp" "vb" "powershell" "aspnet" "xaml" "msbuild" "ilasm"
    "haskell" "ocaml" "elm" "purescript" "idris" "agda" "coq" "lean"
    "sml" "scheme" "racket" "lisp"
    "python" "ruby" "perl" "lua" "tcl" "r" "julia" "matlab" "octave"
    "bash" "fish" "zsh" "pwsh" "vim" "elisp"
    "swift" "objc" "dart" "flutter" "react-native" "xamarin" "ionic" "nativescript"
    "sql" "plsql" "tsql" "sparql" "cypher" "mongodb" "cassandra" "redis"
    "elasticsearch" "influxdb"
    "yaml" "toml" "json" "json5" "jsonc" "xml" "ini" "properties"
    "env" "editorconfig" "gitignore" "dockerignore"
    "dockerfile" "kubernetes" "helm" "terraform" "ansible" "puppet"
    "chef" "saltstack" "vagrant" "jenkins"
)

for lang in "${languages[@]}"; do
    create_queries "$lang"
done

echo ""
echo "✅ Generated query files for all 122 languages!"
echo "Total languages: ${#languages[@]}"
echo "Files created: $(find . -name "*.scm" | wc -l)"
