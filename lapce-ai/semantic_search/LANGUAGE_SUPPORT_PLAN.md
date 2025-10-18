# CST-to-AST Pipeline Language Support Plan

## Current Status
- **CST System**: Supports 67 languages (30 core + 37 external grammars)
- **AST Pipeline**: Only supports 9 languages (rust, javascript, typescript, python, go, java, cpp, c, c_sharp)
- **Gap**: 58 languages need transformer implementation

## Languages from CST-tree-sitter/Cargo.toml

### Core Languages (30) - From crates.io
1. rust
2. python
3. go
4. java
5. c
6. cpp
7. c-sharp
8. ruby
9. php
10. lua
11. bash
12. css
13. json
14. swift
15. scala
16. elixir
17. html
18. ocaml
19. nix
20. make
21. cmake
22. verilog
23. erlang
24. d
25. pascal
26. commonlisp
27. objc
28. groovy
29. embedded-template

### External Grammar Languages (37)
30. javascript (external)
31. typescript (external)
32. toml
33. dockerfile
34. elm
35. kotlin
36. yaml
37. r
38. matlab
39. perl
40. dart
41. julia
42. haskell
43. graphql
44. sql
45. zig
46. vim
47. abap
48. nim
49. clojure
50. crystal
51. fortran
52. vhdl
53. racket
54. ada
55. prolog
56. gradle
57. xml
58. markdown
59. svelte
60. scheme
61. fennel
62. gleam
63. hcl
64. solidity
65. fsharp
66. cobol
67. systemverilog

## Implementation Plan

### Phase 1: Generic Transformer (All 67 languages)
Create a `GenericTransformer` that works for all languages using basic CST node mapping.

### Phase 2: Language Detection
Update `detect_language()` with all file extensions for 67 languages.

### Phase 3: Language Getter
Update `get_language()` to return tree-sitter::Language for all 67 languages.

### Phase 4: Transformer Registration
Register transformers for all 67 languages in `new()` method.

### Phase 5: Specialized Transformers
Create specialized transformers for common languages with unique syntax.

### Phase 6: Testing
Add comprehensive tests for all 67 languages.
