# REAL TASK: Translate Codex TypeScript to Rust

## I COMPLETELY MISUNDERSTOOD!

The task is **NOT** to research grammars and create new queries from scratch.

The task is to **TRANSLATE** Codex's existing TypeScript implementation to Rust!

## What Codex Has

### TypeScript Query Files (need to be converted to .scm)
`/home/verma/lapce/Codex/src/services/tree-sitter/queries/`:
- c-sharp.ts, c.ts, cpp.ts, css.ts, elisp.ts
- **elixir.ts**, embedded_template.ts, go.ts, html.ts
- java.ts, javascript.ts, kotlin.ts, lua.ts
- ocaml.ts, php.ts, python.ts, ruby.ts
- rust.ts, scala.ts, **solidity.ts**, swift.ts
- systemrdl.ts, tlaplus.ts, toml.ts, tsx.ts
- typescript.ts, vue.ts, zig.ts

### TypeScript Logic Files (need to be translated to Rust)
- `index.ts` (416 lines) - Main parsing logic with `processCaptures()`
- `languageParser.ts` - Language parser loading
- `markdownParser.ts` - Markdown parsing

## The Real Task

### Step 1: Convert TypeScript Query Files to .scm

Each `.ts` file exports a query string like:
```typescript
export default String.raw`
; Module definitions
(call
  target: (identifier) @function
  (#eq? @function "defmodule")) @definition.module
`
```

Need to extract the query string and save as `.scm` file.

### Step 2: Translate TypeScript Logic to Rust

The **CRITICAL** function is `processCaptures()` at line 268-368 of `index.ts`:

```typescript
function processCaptures(captures: QueryCapture[], lines: string[], language: string): string | null {
    // ... 100 lines of PERFECTED logic ...
    // Output format: "startLine--endLine | definition_text"
    // MIN_COMPONENT_LINES = 4
    // HTML filtering for JSX/TSX
    // Duplicate line prevention
}
```

This EXACT logic must be translated to Rust - it took YEARS to perfect!

### Step 3: Create Rust Structure

Following the doc structure in `05-TREE-SITTER-INTEGRATION.md`:
```rust
pub struct NativeParserManager {
    parsers: DashMap<FileType, Arc<Parser>>,
    queries: DashMap<FileType, Arc<CompiledQueries>>,
    tree_cache: Arc<TreeCache>,
}

pub struct CompiledQueries {
    highlights: Query,  // From highlights.scm (NOT in Codex)
    locals: Query,      // From locals.scm (NOT in Codex)
    injections: Query,  // From injections.scm (NOT in Codex)
    tags: Query,        // FROM CODEX .ts FILES!
    folds: Query,       // From folds.scm (NOT in Codex)
}
```

## The Problem

Codex only has **tags queries** (symbol extraction).
The Rust design expects **5 query types**.

For the missing 4 types (highlights, locals, injections, folds), we need to:
- Use existing good queries from CST-tree-sitter/queries/ (rust, python, go, java, typescript, javascript, c)
- Create them for languages Codex has but we don't

## Action Plan

1. **Extract queries from Codex's 29 .ts files** → Create .scm files
2. **Translate index.ts logic** → Rust (preserve processCaptures EXACTLY)
3. **Translate languageParser.ts** → Rust
4. **Handle missing query types** → Use existing or create minimal ones
5. **Test with Codex's test files** → Ensure 1:1 output match

## Languages Codex Has

29 languages with queries:
- c-sharp, c, cpp, css, elisp, elixir, embedded_template
- go, html, java, javascript, kotlin, lua
- ocaml, php, python, ruby, rust, scala, solidity
- swift, systemrdl, tlaplus, toml, tsx, typescript
- vue, zig

These should be our PRIMARY focus, not creating new ones!
