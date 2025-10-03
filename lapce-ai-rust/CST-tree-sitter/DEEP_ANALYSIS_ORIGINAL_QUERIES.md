# DEEP ANALYSIS: Original Query Files

## CRITICAL DISCOVERY

After reading 20+ original directories, I discovered **TWO CLASSES** of query files:

### CLASS A: PROPER Language-Specific Queries (8 languages)

These are REAL, production-quality query files with language-specific content:

1. **rust/** (85 lines highlights.scm)
   - Rust-specific keywords: "async", "await", "impl", "trait", "mod", "pub"
   - Actual grammar nodes: `(function_item)`, `(macro_invocation)`, `(trait_item)`
   - Proper operators: "->", "::", "..="
   - 32 lines injections.scm with markdown in doc comments, SQL in macros
   - 28 lines locals.scm with closure_expression, match_arm scopes

2. **python/** (101 lines highlights.scm)
   - Python keywords: "lambda", "nonlocal", "pass", "match", "case"
   - Built-in function regex matching
   - Nodes: `(function_definition)`, `(decorator)`, `(class_definition)`
   - 38 lines injections with reStructuredText docstrings, f-strings
   - 43 lines locals with for_statement, with_statement scopes

3. **go/** (130 lines highlights.scm)
   - Go keywords: "chan", "defer", "fallthrough", "select", "range"
   - Nodes: `(selector_expression)`, `(method_declaration)`, `(field_identifier)`
   - 36 lines injections with regex in regexp package, templates
   - 56 lines locals with short_var_declaration, type_switch

4. **java/** (85 lines highlights.scm)
   - Java-specific nodes: `(annotation)`, `(constructor_declaration)`
   - Built-in types regex: "boolean|byte|char|double|float|int|long|short|void"
   - 32 lines injections with Javadoc, XML in strings
   - 62 lines locals with enhanced_for_statement, catch_formal_parameter

5. **typescript/** (137 lines highlights.scm)
   - TS keywords: "interface", "namespace", "readonly", "satisfies", "keyof"
   - Type-specific nodes: `(type_annotation)`, `(type_predicate)`, `(type_parameter)`
   - 41 lines injections with JSDoc, GraphQL, SQL tagged templates
   - 49 lines locals with interface_declaration, enum_declaration

6. **javascript/** (112 lines highlights.scm)
   - JS-specific: `(template_string)`, `(regex)`, `(super)`, `(this)`
   - 34 lines injections with regex, JSDoc, tagged templates
   - 35 lines locals with arrow_function, catch_clause

7. **c/** (57 lines highlights.scm)
   - C-specific nodes: `(struct_specifier)`, `(union_specifier)`, `(pointer_declarator)`
   - 24 lines injections with inline assembly
   - 55 lines locals with translation_unit, compound_statement

8. **cpp/** - WAIT, cpp is actually BROKEN (see Class B)

### CLASS B: GENERIC GARBAGE Templates (49+ languages)

These files have **IDENTICAL generic content** with WRONG keywords:

- **cpp/, ruby/, php/, lua/, bash/, swift/, scala/, kotlin/, html/, css/, json/, yaml/, toml/, elm/, ocaml/, sql/, graphql/, dart/, haskell/, julia/, matlab/, perl/, clojure/**

All contain:
```scm
; Keywords
[
  "if" "else" "elif" "then" "fi"        # ← BASH keywords in ALL languages!
  "for" "while" "do" "done"              # ← "done" doesn't exist in most languages
  "function" "def" "class" "struct"      # ← Mix of Python/Ruby/C syntax
```

All have generic node names that DON'T match actual grammars:
```scm
(function_definition name: (identifier) @function)  # ← May not exist in that language
(type_identifier) @type                             # ← Generic, not language-specific
```

**These are USELESS and will cause tree-sitter query errors!**

## THE PATTERN from CLASS A (Good Queries)

### 1. highlights.scm Structure

```scm
; STEP 1: Language-specific keywords (actual tokens from grammar)
[
  "keyword1" "keyword2" "keyword3"
] @keyword

; STEP 2: Functions using ACTUAL grammar node names
(actual_function_node_name name: (identifier) @function)
(actual_call_node name: (identifier) @function.call)

; STEP 3: Types using ACTUAL type nodes
(type_identifier) @type
(builtin_type_node) @type.builtin

; STEP 4: Variables using ACTUAL variable nodes
(identifier) @variable
(actual_property_node) @property

; STEP 5: Literals using ACTUAL literal nodes
(string_literal_node) @string
(number_literal_node) @number

; STEP 6: Comments using ACTUAL comment nodes
(comment_node) @comment

; STEP 7: Operators (actual operator tokens)
["+" "-" "*" ...] @operator

; STEP 8: Punctuation (actual punctuation tokens)
["(" ")" "[" "]"] @punctuation.bracket
```

### 2. injections.scm Purpose

Inject other languages into embedded content:
- Documentation comments → markdown/rst
- String literals → sql/regex/json
- Tagged templates → graphql/css
- Macro content → format strings

### 3. locals.scm Purpose

Track variable scopes and definitions:
- Define all scope-creating nodes
- Define all definition-creating nodes
- Mark all identifier references

### 4. tags.scm Purpose

Extract symbols for code navigation:
- Functions → @definition.function
- Classes → @definition.class
- Methods → @definition.method
- Variables → @definition.variable

### 5. folds.scm Purpose

Define code folding regions:
- Function bodies
- Class bodies
- Control flow blocks
- Comments

## REQUIRED ACTIONS

1. **Identify Class A languages**: Only rust, python, go, java, typescript, javascript, c (7 total)

2. **For my 17 missing languages**, I MUST:
   - Find each language's tree-sitter grammar repository
   - Read grammar.js to see actual node names
   - Find official query examples from tree-sitter org
   - Create language-specific queries based on ACTUAL grammar

3. **Cannot use templates**: Each language needs custom queries matching its grammar

4. **The 17 languages I need to create**:
   - elixir, nix, latex, make, cmake
   - verilog, erlang, commonlisp, hlsl, hcl
   - solidity, systemverilog, embedded_template
   - abap, crystal, vhdl, prolog

## NEXT STEPS

1. Find tree-sitter grammar repos for each language
2. Study each grammar.js file
3. Find existing query examples
4. Create proper language-specific queries
5. Test each query file

This will take significant time but is the ONLY correct approach.
