# Codex Tree-Sitter Queries Analysis

## Location
`/home/verma/lapce/Codex/src/services/tree-sitter/queries/`

## What They Have

**29 TypeScript files** containing tree-sitter query strings:
- c-sharp.ts, c.ts, cpp.ts, css.ts, elisp.ts
- **elixir.ts**, embedded_template.ts, go.ts, html.ts
- java.ts, javascript.ts, kotlin.ts, lua.ts
- ocaml.ts, php.ts, python.ts, ruby.ts
- rust.ts, scala.ts, **solidity.ts**, swift.ts
- systemrdl.ts, tlaplus.ts, toml.ts, tsx.ts
- typescript.ts, vue.ts, zig.ts

## Important Differences from Our Needs

### What Codex Queries ARE:
1. **TypeScript files** exporting query strings (not .scm files)
2. **Symbol extraction ONLY** - for code navigation/definitions
3. **Single-purpose** - only extract definitions like functions, classes, etc.
4. **Tags-equivalent** - similar to our tags.scm files

### What Codex Queries are NOT:
1. ❌ NOT syntax highlighting queries
2. ❌ NOT injection queries  
3. ❌ NOT locals/scope tracking
4. ❌ NOT fold queries
5. ❌ NOT the 5-file structure we need

## Example: Elixir Query

```typescript
export default String.raw`
; Module, Protocol, and Implementation definitions
(call
  target: (identifier) @function
  (arguments) @args
  (do_block)?
  (#match? @function "^(defmodule|defprotocol|defimpl)$")) @definition.module

; Function definitions
(call
  target: (identifier) @function
  (arguments) @args
  (do_block)?
  (#eq? @function "def")) @definition.function
```

Shows actual Elixir grammar nodes: `(call)`, `(identifier)`, `(arguments)`, `(do_block)`

## Example: Solidity Query

```typescript
export const solidityQuery = `
; Contract declarations
(contract_declaration
  name: (identifier) @name.definition.contract) @definition.contract

(function_definition
  name: (identifier) @name.definition.function) @definition.function
```

Shows actual Solidity nodes: `(contract_declaration)`, `(function_definition)`

## What We Can Learn

These files show **ACTUAL GRAMMAR NODE NAMES** for:
- ✅ elixir (call, identifier, do_block, arguments)
- ✅ solidity (contract_declaration, function_definition, struct_declaration)
- ✅ embedded_template (directive, code, output_directive, comment_directive)

This is valuable for creating our 5 query files!

## What We Still Need

For each of our 17 missing languages, we need to create 5 separate .scm files:

1. **highlights.scm** - Keywords, types, literals (NOT in Codex)
2. **injections.scm** - Embedded languages (NOT in Codex)
3. **locals.scm** - Scope tracking (NOT in Codex)
4. **tags.scm** - Symbol extraction (SIMILAR to Codex queries)
5. **folds.scm** - Code folding (NOT in Codex)

## Languages Overlap

Codex has queries for 3 of our 17 missing languages:
1. **elixir** ✓ (can use node names)
2. **solidity** ✓ (can use node names)
3. **embedded_template** ✓ (can use node names)

Still need grammar research for:
- nix, latex, make, cmake
- verilog, erlang, commonlisp, hlsl, hcl
- systemverilog, abap, crystal, vhdl, prolog

## Action Plan

1. Use Codex queries to understand node names for elixir, solidity, embedded_template
2. Still need to research grammars for remaining 14 languages
3. Create full 5-file structure for all 17 languages
4. Cannot just copy Codex - need full highlighting, injections, locals, folds
