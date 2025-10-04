# Tree-Sitter Version Conflicts

## The Problem

**Core Issue**: tree-sitter-javascript 0.25.0 and tree-sitter-typescript 0.23.2 require tree-sitter API version 15 (tree-sitter 0.25+), but project uses tree-sitter 0.24.7 (API version 14).

## Languages Causing Conflicts

### 23 External Grammar Dependencies (Path-based)

All locked to tree-sitter 0.24:

**Phase 1 - High Priority (10 languages)**:
1. `tree-sitter-kotlin` - path dependency
2. `tree-sitter-yaml` - path dependency
3. `tree-sitter-r` - path dependency
4. `tree-sitter-matlab` - path dependency
5. `tree-sitter-perl` - path dependency
6. `tree-sitter-dart` - path dependency
7. `tree-sitter-julia` - path dependency
8. `tree-sitter-haskell` - path dependency
9. `tree-sitter-graphql` - path dependency
10. `tree-sitter-sql` - path dependency

**Phase 2 - Remaining (13 languages)**:
11. `tree-sitter-zig` - path dependency
12. `tree-sitter-vim` - path dependency
13. `tree-sitter-abap` - path dependency
14. `tree-sitter-nim` - path dependency
15. `tree-sitter-clojure` - path dependency
16. `tree-sitter-crystal` - path dependency
17. `tree-sitter-fortran` - path dependency
18. `tree-sitter-vhdl` - path dependency
19. `tree-sitter-racket` - path dependency
20. `tree-sitter-ada` - path dependency
21. `tree-sitter-prolog` - path dependency
22. `tree-sitter-pascal` - path dependency
23. `tree-sitter-fsharp` - path dependency

## Languages Already Compatible

These work with tree-sitter 0.24.7:

- ✅ tree-sitter-rust 0.24.0
- ❌ tree-sitter-javascript 0.25.0 (needs 0.25+)
- ❌ tree-sitter-typescript 0.23.2 (needs 0.25+)
- ✅ tree-sitter-python 0.25.0
- ✅ tree-sitter-go 0.25.0
- ✅ tree-sitter-java 0.23.5
- ✅ tree-sitter-c 0.24.1
- ✅ tree-sitter-cpp 0.23.4
- ✅ tree-sitter-c-sharp 0.23.1
- ✅ tree-sitter-ruby 0.23.1
- ✅ tree-sitter-php 0.24.2
- ✅ tree-sitter-lua 0.2.0
- ✅ tree-sitter-bash 0.25.0
- ✅ tree-sitter-css 0.23.2
- ✅ tree-sitter-json 0.24.8
- ✅ tree-sitter-swift 0.7.1
- ✅ tree-sitter-scala 0.24.0
- ✅ tree-sitter-elixir 0.3.4
- ✅ tree-sitter-html 0.23.2
- ✅ tree-sitter-elm 5.8.0

## Solution Options

### Option 1: Downgrade JS/TS (Quick Fix)
```toml
tree-sitter-javascript = "0.24.0"  # Instead of 0.25.0
tree-sitter-typescript = "0.23.0"  # Compatible with 0.24
```

### Option 2: Upgrade All 23 External Grammars
- Update each external grammar to tree-sitter 0.25
- Time-consuming but proper solution

### Option 3: Test with Working Languages
- Use Rust parser (works with 0.24.7)
- Parse Rust files from Codex folder
- Get real CST memory measurement

## Impact

**Cannot test JS/TS files** until version conflict resolved.

**Total conflicting languages**: 25
- 23 external path dependencies (locked to 0.24)
- 2 registry dependencies requiring 0.25+ (JS/TS)
