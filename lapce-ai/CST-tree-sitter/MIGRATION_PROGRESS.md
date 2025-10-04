# Tree-sitter 0.24 Migration Progress

## Phase 1: Cleanup ✅ COMPLETED
- ✅ Used `trash-put` instead of `rm` for safe removal
- ✅ Removed 85+ external-grammars
- ✅ Kept only 23 ❌ languages (missing from our production system)

## Phase 2: Version Updates ✅ COMPLETED  
- ✅ Updated all Cargo.toml files from various versions → tree-sitter 0.24
  - kotlin: 0.21+ → 0.24
  - yaml: tree-sitter-language 0.1 → 0.24
  - dart: 0.22.6 → 0.24
  - abap: 0.22.5+ → 0.24
  - nim: 0.25 → 0.24
  - zig, vim: 0.21+ → 0.24
  - perl, graphql, prolog: 0.17 → 0.24
  - All tree-sitter-language 0.1 → 0.24

## Phase 3: Path Dependencies ✅ COMPLETED
Added 20 languages as path dependencies in main Cargo.toml:

### High Priority 10 (Phase 1):
1. ✅ kotlin
2. ✅ yaml
3. ✅ r
4. ✅ matlab
5. ✅ perl
6. ✅ dart
7. ✅ julia
8. ✅ haskell
9. ✅ graphql
10. ❌ sql (missing parser.c)

### Remaining 13 (Phase 2):
11. ✅ zig
12. ✅ vim
13. ✅ abap
14. ✅ nim
15. ✅ clojure
16. ✅ crystal
17. ✅ fortran
18. ✅ vhdl
19. ✅ racket
20. ✅ ada
21. ✅ prolog
22. ❌ xml (missing parser.c)
23. ❌ gradle (missing parser.c)

## Phase 4: Generate Missing Parser.c ⏳ IN PROGRESS

### Need Generation:
- tree-sitter-sql/src/parser.c
- tree-sitter-xml/src/parser.c  
- tree-sitter-gradle/src/parser.c

### Tool Installed:
- ✅ tree-sitter-cli v0.25.10 installed at ~/.cargo/bin/tree-sitter

### Commands to Run:
```bash
cd external-grammars/tree-sitter-sql && tree-sitter generate
cd external-grammars/tree-sitter-xml && tree-sitter generate
cd external-grammars/tree-sitter-gradle && tree-sitter generate
```

## Phase 5: API Compatibility Fixes ⏳ NEXT

All lib.rs files use modern API:
```rust
use tree_sitter::Language;
extern "C" { fn tree_sitter_LANG() -> Language; }
pub fn language() -> Language { unsafe { tree_sitter_LANG() } }
```

This is compatible with tree-sitter 0.24. No changes needed.

## Phase 6: Compilation Testing ⏳ NEXT

Test each language individually:
```bash
cd external-grammars/tree-sitter-kotlin && cargo build
cd external-grammars/tree-sitter-yaml && cargo build
# ... repeat for all 20 languages
```

Then test main crate:
```bash
cd /home/verma/lapce/lapce-ai-rust/CST-tree-sitter
cargo build --release
```

## Phase 7: Integration Testing ⏳ PENDING

Update native_parser_manager.rs and parser_pool.rs to register all 20 new languages.

## Current Status:
- 23 languages kept in external-grammars/
- 20 languages have path dependencies added
- 3 languages need parser.c generation (SQL, XML, Gradle)
- All Cargo.toml files updated to tree-sitter 0.24
- Ready for systematic compilation and error fixing

## Next Immediate Steps:
1. Generate parser.c for SQL, XML, Gradle
2. Test compile each language individually
3. Fix any API incompatibilities found
4. Test main crate compilation
5. Register new languages in production code
