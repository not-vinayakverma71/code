# Tree-sitter 0.23.x Migration Plan

## Phase 0: Core Upgrade (Day 1-2)
- [ ] Update tree-sitter = "0.23.4"
- [ ] Update tree-sitter-highlight = "0.23.0"  
- [ ] Update all 25 language parsers to 0.23.x versions
- [ ] Fix compilation errors
- [ ] Fix API breaking changes
- [ ] Update query files

## Phase 1: Verify Existing Languages (Day 3)
- [ ] Test JavaScript, TypeScript, TSX
- [ ] Test Python, Rust, Go
- [ ] Test C, C++, C#
- [ ] Test Java, Ruby, PHP, Swift
- [ ] Test Lua, Elixir, Scala, Elm, OCaml
- [ ] Test HTML, CSS, JSON, TOML, Bash
- [ ] Test Dockerfile, Markdown

## Phase 2: Add Missing Languages (Day 4-5)
### Mobile & Enterprise
- [ ] Kotlin (Android)
- [ ] Dart (Flutter)
- [ ] Objective-C (iOS legacy)

### Data & Config
- [ ] YAML (configuration)
- [ ] SQL (databases)
- [ ] GraphQL (APIs)
- [ ] INI (config files)

### Scientific & Academic
- [ ] Julia (scientific computing)
- [ ] R (statistics)
- [ ] MATLAB (engineering)
- [ ] LaTeX (documents)
- [ ] BibTeX (citations)

### Functional Programming
- [ ] Clojure (JVM Lisp)
- [ ] Haskell (pure functional)
- [ ] F# (.NET functional)
- [ ] CommonLisp (classic)
- [ ] Scheme (educational)
- [ ] Racket (PLT)

### Systems & Hardware
- [ ] Fortran (HPC)
- [ ] Ada (safety-critical)
- [ ] Verilog (FPGA)
- [ ] VHDL (hardware)
- [ ] SystemVerilog (verification)

### Build & DevOps
- [ ] CMake (build system)
- [ ] Make (build automation)
- [ ] Gradle (JVM builds)
- [ ] Terraform/HCL (infrastructure)

### Blockchain
- [ ] Solidity (Ethereum)
- [ ] Vyper (smart contracts)
- [ ] Cairo (StarkNet)

### Additional Languages
- [ ] Nim (systems)
- [ ] Crystal (Ruby-like)
- [ ] Zig (modern C)
- [ ] D (systems)
- [ ] Perl (scripting)

## Phase 3: Testing & Performance (Day 6)
- [ ] Run production_test_final for all 60 languages
- [ ] Measure memory usage (<5MB requirement)
- [ ] Verify parse speed (>125K lines/sec)
- [ ] Test incremental parsing (<10ms)
- [ ] Check cache hit rate (>90%)

## Success Criteria
- ✅ All 25 existing languages continue working
- ✅ 35+ new languages added successfully
- ✅ Memory usage under 5MB
- ✅ Parse speed >125K lines/sec
- ✅ Incremental parsing <10ms
- ✅ Cache hit rate >90%
