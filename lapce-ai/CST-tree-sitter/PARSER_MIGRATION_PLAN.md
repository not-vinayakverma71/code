# Parser Migration Plan - 23 Languages to Tree-sitter 0.24

## Version Analysis Results:

### Missing parser.c (need generation):
- ❌ gradle
- ❌ sql  
- ❌ xml

### Using tree-sitter-language 0.1 (newer API):
- ✅ ada, crystal, fortran, haskell, julia, matlab, r, racket, sql, vhdl, xml, yaml

### Using old tree-sitter versions:
- ⚠️ graphql (0.17)
- ⚠️ perl (0.17)
- ⚠️ prolog (0.17)
- ⚠️ gradle (0.20)
- ⚠️ kotlin (>=0.21)
- ⚠️ vim (>=0.21)
- ⚠️ zig (>=0.21)
- ⚠️ dart (0.22.6)
- ⚠️ abap (>=0.22.5)
- ⚠️ nim (~0.25)

### Without Cargo.toml:
- ⚠️ clojure

## Phase 1: High Priority 10 Languages

### Tier 1 - Absolutely Critical:
1. **SQL** - Every database (missing parser.c)
2. **Kotlin** - Android official (0.21+)
3. **YAML** - Kubernetes/CI/CD (language 0.1)

### Tier 2 - Enterprise Essential:
4. **R** - Statistical analysis (language 0.1)
5. **MATLAB** - Engineering standard (language 0.1)
6. **Perl** - System admin (0.17)
7. **Dart** - Flutter mobile (0.22.6)

### Tier 3 - Domain Critical:
8. **Julia** - Scientific computing (language 0.1)
9. **Haskell** - Financial modeling (language 0.1)

### Tier 4 - Web & Config:
10. **GraphQL** - Modern APIs (0.17)

## Phase 2: Remaining 13 Languages

11. XML (missing parser.c)
12. Vim (0.21+)
13. Zig (0.21+)
14. ABAP (0.22.5+)
15. Nim (0.25)
16. Clojure (no Cargo)
17. Crystal (language 0.1)
18. Fortran (language 0.1)
19. VHDL (language 0.1)
20. Racket (language 0.1)
21. Ada (language 0.1)
22. Prolog (0.17)
23. Gradle (0.20, missing parser.c)

## Migration Steps Per Language:

### Step 1: Generate parser.c (if missing)
```bash
cd external-grammars/tree-sitter-{lang}
npm install
npx tree-sitter generate
```

### Step 2: Update Cargo.toml
```toml
[dependencies]
tree-sitter = "0.24"
# Remove tree-sitter-language if present
```

### Step 3: Fix API changes
- Old: `language()` returns raw pointer
- New 0.24: Use `LANGUAGE` constant or proper Language type
- Scanner interface may need updates

### Step 4: Create local Rust bindings
```rust
// In bindings/rust/lib.rs
use tree_sitter::Language;

extern "C" { fn tree_sitter_LANG() -> Language; }

pub fn language() -> Language {
    unsafe { tree_sitter_LANG() }
}
```

### Step 5: Add to Cargo.toml as path dependency
```toml
tree-sitter-kotlin = { path = "external-grammars/tree-sitter-kotlin" }
```

### Step 6: Test compilation
```bash
cargo build --release
```

## Success Criteria:
- ✅ All 23 languages compile with tree-sitter 0.24
- ✅ All parser.c files generated
- ✅ All Cargo.toml files updated
- ✅ Zero compilation errors
- ✅ Can parse sample files for each language
- ✅ 100% pure Rust native implementation
