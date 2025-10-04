# CODEX TREE-SITTER DEEP ANALYSIS REPORT


## CRITICAL: This is the EXACT Implementation We Must Translate to Rust


## Overview
The Codex VS Code extension uses WASM-based tree-sitter parsers for code intelligence. We're moving to native Rust parsers for 10x performance improvement.


## 1. CORE ARCHITECTURE


### Files Structure
```
Codex/src/services/tree-sitter/
├── index.ts                 # Main parsing logic & symbol extraction
├── languageParser.ts        # Language-specific parser loading
├── markdownParser.ts        # Special markdown handling
└── queries/                 # Language-specific query patterns (29 files)
    ├── javascript.ts
    ├── typescript.ts
    ├── rust.ts
    ├── python.ts
    └── ... (25 more languages)
```


### WASM Files (38 languages currently supported)
- Located in: `/home/verma/lapce/Codex/src/dist/tree-sitter-*.wasm`
- Languages: bash, c, c_sharp, cpp, css, elisp, elixir, elm, embedded_template, go, html, java, javascript, json, kotlin, lua, objc, ocaml, php, python, ql, rescript, ruby, rust, scala, solidity, swift, systemrdl, tlaplus, toml, tsx, typescript, vue, yaml, zig


## 2. SYMBOL FORMAT (MUST COPY EXACTLY)


### From `listCodeDefinitionNamesTool.ts`:
The tool uses `parseSourceCodeDefinitionsForFile` and `parseSourceCodeForDefinitionsTopLevel` which return symbols in this EXACT format:


```typescript
// Format: line_start--line_end | definition_text
"1--10 | function myFunc()"
"11--20 | class MyClass"
"21--30 | MyClass.method()"
"31--40 | const myVar"
```


### Key Symbol Extraction Logic (from `index.ts`):


```typescript
// Lines 268-368: processCaptures function
function processCaptures(captures: QueryCapture[], lines: string[], language: string): string | null {
    // Filter HTML elements for JSX/TSX
    const needsHtmlFiltering = ["jsx", "tsx"].includes(language)
    
    // Minimum component lines = 4 (configurable)
    const MIN_COMPONENT_LINES = 4
    
    // Sort captures by start position
    captures.sort((a, b) => a.node.startPosition.row - b.node.startPosition.row)
    
    // Track processed lines to avoid duplicates
    const processedLines = new Set<string>()
    
    // Output format: "startLine--endLine | lines[startLine]\n"
    // Example: "1--10 | function calculateSum()"
}
```


### Symbol Categories Captured:
- **Functions**: `function myFunc()`, arrow functions, async functions
- **Classes**: `class MyClass`, abstract classes, interfaces
- **Methods**: `MyClass.method()`, static methods, getters/setters
- **Variables**: `const myVar`, `let myVar`, `var myVar`
- **Type definitions**: `type MyType`, `interface MyInterface`
- **Modules**: `module MyModule`, imports/exports
- **Special constructs**: decorators, macros, traits, impl blocks


## 3. PARSER INITIALIZATION


### From `languageParser.ts`:


```typescript
// Lines 78-90: Parser initialization
export async function loadRequiredLanguageParsers(filesToParse: string[]) {
    const { Parser, Query } = require("web-tree-sitter")
    
    if (!isParserInitialized) {
        await Parser.init()
        isParserInitialized = true
    }
    
    // Load only required parsers based on file extensions
    const extensionsToLoad = new Set(filesToParse.map(file => 
        path.extname(file).toLowerCase().slice(1)))
    
    // Create parser and query for each language
    for (const ext of extensionsToLoad) {
        const language = await loadLanguage(langName)
        const parser = new Parser()
        parser.setLanguage(language)
        parsers[ext] = { parser, query }
    }
}
```


### Extension Mapping (MUST MATCH):
```typescript
// Lines 99-221: Extension to parser mapping
switch (ext) {
    case "js":
    case "jsx":
    case "json":
        language = "javascript"
        break
    case "ts":
        language = "typescript"
        break
    case "tsx":
        language = "tsx"
        break
    case "py":
        language = "python"
        break
    case "rs":
        language = "rust"
        break
    // ... 30+ more languages
}
```


## 4. QUERY PATTERNS (CRITICAL)


### JavaScript Query Pattern Example:
```typescript
// From queries/javascript.ts
`
(method_definition
    name: (property_identifier) @name) @definition.method
    
(class_declaration
    name: (_) @name) @definition.class
    
(function_declaration
    name: (identifier) @name) @definition.function
    
(lexical_declaration
    (variable_declarator
        name: (identifier) @name
        value: [(arrow_function) (function_expression)]) @definition.function)
`
```


### Rust Query Pattern:
```typescript
// From queries/rust.ts
`
(function_item
    name: (identifier) @name.definition.function) @definition.function
    
(struct_item
    name: (type_identifier) @name.definition.struct) @definition.struct
    
(impl_item
    type: (type_identifier) @name.definition.impl) @definition.impl
    
(mod_item
    name: (identifier) @name.definition.module) @definition.module
`
```


## 5. CONTEXT EXTRACTION LOGIC


### From `index.ts` parseFile function:
```typescript
// Lines 378-415
async function parseFile(filePath: string, languageParsers: LanguageParser): Promise<string | null> {
    // 1. Read file content
    const fileContent = await fs.readFile(filePath, "utf8")
    
    // 2. Parse into AST
    const tree = parser.parse(fileContent)
    
    // 3. Apply queries to get captures
    const captures = query.captures(tree.rootNode)
    
    // 4. Process captures into formatted output
    return processCaptures(captures, lines, language)
}
```


### Directory-level parsing:
```typescript
// Lines 152-227: parseSourceCodeForDefinitionsTopLevel
// - Lists files (max 50 files)
// - Filters by supported extensions
// - Respects .gitignore via rooIgnoreController
// - Processes markdown separately
// - Returns concatenated results with relative paths
```


## 6. SPECIAL HANDLING


### Markdown Parser (markdownParser.ts):
- Extracts headers (ATX: #, ##, ### and Setext: ===, ---)
- Returns section ranges (header start to next header)
- Minimum section lines: 4
- Output format: `startLine--endLine | # Header Text`


### HTML Filtering (JSX/TSX):
```typescript
// Lines 273-279 in index.ts
const isNotHtmlElement = (line: string): boolean => {
    if (!needsHtmlFiltering) return true
    const HTML_ELEMENTS = /^[^A-Z]*<\/?(?:div|span|button|input|h[1-6]|p|a|img|ul|li|form)\b/
    return !HTML_ELEMENTS.test(line.trim())
}
```


## 7. PERFORMANCE OPTIMIZATIONS


### From Current Implementation:
1. **Lazy Loading**: Only load parsers for files being processed
2. **Parser Reuse**: Single parser instance per language
3. **Query Compilation**: Pre-compile queries, reuse across files
4. **Line Deduplication**: Track processed lines to avoid duplicates
5. **Minimum Lines Filter**: Skip components < 4 lines


## 8. CRITICAL RULES FOR RUST TRANSLATION


### MUST PRESERVE:
1. **Exact Symbol Format**: `startLine--endLine | definition_text`
2. **Line Numbering**: 1-indexed (not 0-indexed)
3. **Query Patterns**: All 29 language query patterns must be identical
4. **Extension Mapping**: Exact same file extension to language mapping
5. **Minimum Component Lines**: Default 4 lines
6. **HTML Filtering**: For JSX/TSX files
7. **Markdown Support**: Special handling for .md files
8. **Directory Traversal**: Max 50 files, respect gitignore


### MUST CHANGE:
1. **WASM → Native**: Replace web-tree-sitter with native Rust parsers
2. **Async/Await → Tokio**: Use Rust async runtime
3. **Node FS → Rust std::fs**: File system operations
4. **JavaScript Regex → Rust regex**: Pattern matching
5. **Dynamic require() → Static imports**: Language loading


## 9. LANGUAGE SUPPORT REQUIREMENTS


### Current 38 Languages (from WASM files):
bash, c, c_sharp, cpp, css, elisp, elixir, elm, embedded_template, go, html, java, javascript, json, kotlin, lua, objc, ocaml, php, python, ql, rescript, ruby, rust, scala, solidity, swift, systemrdl, tlaplus, toml, tsx, typescript, vue, yaml, zig


### Extension Support (from index.ts lines 30-94):
```
.tla, .js, .jsx, .ts, .vue, .tsx, .py, .rs, .go, .c, .h, .cpp, .hpp,
.cs, .rb, .java, .php, .swift, .sol, .kt, .kts, .ex, .exs, .el,
.html, .htm, .md, .markdown, .json, .css, .rdl, .ml, .mli, .lua,
.scala, .toml, .zig, .elm, .ejs, .erb, .vb
```


## 10. TEST FIXTURES


The Codex has extensive test fixtures in:
`/home/verma/lapce/Codex/src/services/tree-sitter/__tests__/fixtures/`


Each fixture contains sample code for testing symbol extraction. We MUST ensure our Rust implementation passes all these test cases with identical output.


## IMPLEMENTATION CHECKLIST


- [ ] Create native parser manager with same interface
- [ ] Implement all 29 query patterns identically  
- [ ] Preserve exact symbol format: `line--line | text`
- [ ] Implement markdown special handling
- [ ] Add HTML filtering for JSX/TSX
- [ ] Support all 38 languages
- [ ] Match extension mappings exactly
- [ ] Implement directory traversal (max 50 files)
- [ ] Add gitignore support
- [ ] Ensure 1-indexed line numbers
- [ ] Default 4-line minimum for components
- [ ] Test against all Codex fixtures


## CONCLUSION


This is a 1:1 translation task. The logic has been perfected over years. We must:
1. Keep ALL business logic identical
2. Only change TypeScript syntax to Rust
3. Replace WASM with native parsers
4. Maintain exact same output format
5. Pass all existing test cases


The symbol format and extraction logic is battle-tested and MUST NOT be modified.




# LAPCE-TREE-SITTER: IMPLEMENTATION vs REQUIREMENTS ANALYSIS

## DEEP ANALYSIS OF ACTUAL IMPLEMENTATION

### ✅ WHAT'S IMPLEMENTED

1. **Basic Parser Infrastructure**
   - `native_parser_manager.rs`: Basic parser manager with 32 FileTypes defined
   - `parser_pool.rs`: Parser pooling for performance
   - `cache_impl.rs`: Tree caching with moka cache
   - `incremental_parser.rs`: Basic incremental parsing support
   - 17 working language parsers (confirmed by test_each_parser binary)

2. **Symbol Extraction (BUT WRONG FORMAT)**
   - `symbol_extraction.rs`: Basic symbol extraction for 11 languages
   - `codex_symbol_format.rs`: Attempts to use Codex format BUT outputs wrong format
   - Current output: `Symbol { name: "myFunc", kind: Function }` 
   - NOT the required: `"1--10 | function myFunc()"`

3. **Query Files**
   - `/queries/` folder has 29 language query files
   - BUT they are NOT the exact queries from Codex
   - Missing the specific patterns for definition extraction

4. **Performance Features**
   - Cache hit rate works (>90%)
   - Incremental parsing works (<10ms)
   - Parse speed varies (Java slow at 2.8K lines/sec)

### ❌ WHAT'S MISSING (CRITICAL)

1. **EXACT CODEX SYMBOL FORMAT** ⚠️ MOST CRITICAL
   - Required: `"startLine--endLine | definition_text"` (e.g., `"1--10 | function myFunc()"`)
   - Current: Returns struct with separate fields
   - Line numbers should be 1-indexed, not 0-indexed
   - No implementation of the exact format anywhere

2. **processCaptures Function** ⚠️ CORE LOGIC MISSING
   - The heart of Codex's extraction logic
   - Filters components < 4 lines (MIN_COMPONENT_LINES)
   - Deduplicates overlapping ranges
   - Sorts captures by position
   - NOT IMPLEMENTED AT ALL

3. **Markdown Parser**
   - Codex has special markdown handling for .md files
   - Extracts headers (ATX and Setext styles)
   - Returns section ranges
   - NOT IMPLEMENTED

4. **HTML Filtering for JSX/TSX**
   - Should filter out HTML elements in JSX/TSX files
   - Pattern: `/^[^A-Z]*<\/?(?:div|span|button|input|h[1-6]|p|a|img|ul|li|form)\b/`
   - NOT IMPLEMENTED

5. **Directory Traversal Logic**
   - Should limit to 50 files max
   - Should respect .gitignore
   - Should separate markdown from other files
   - NOT IMPLEMENTED

6. **Language Support Gap**
   - Required: 38+ languages (from WASM files)
   - Actual: Only 17 working
   - Missing: lua, yaml, swift, kotlin, haskell, elixir, erlang, clojure, zig, html, vue, markdown, julia, nim, dart, and more

7. **Extension Mapping**
   - Incomplete mapping (e.g., .jsx, .vue, .ex, .exs, .kt, .kts, etc.)
   - Should handle 40+ extensions
   - Currently handles ~20

8. **Query Loading from Files**
   - Codex loads queries from separate .ts files
   - Our queries are embedded strings, not loaded from files
   - Missing the exact query patterns from Codex

### 📊 COMPLETION STATUS

**Based on Requirements from 07-TREE-SITTER-INTEGRATION.md:**

| Requirement | Status | Notes |
|------------|--------|-------|
| Memory < 5MB | ❓ Untested | Memory test crashes |
| Parse Speed > 10K lines/sec | ⚠️ Partial | Most work, Java slow (2.8K) |
| 100+ Languages | ❌ Failed | Only 17/100+ working |
| Incremental Parsing < 10ms | ✅ Works | ~0.02ms achieved |
| Symbol Extraction < 50ms | ❌ Wrong Format | Works but wrong output format |
| Cache Hit Rate > 90% | ✅ Works | 90% achieved |
| Query Performance < 1ms | ✅ Works | <1ms achieved |
| Parse 1M+ lines | ⚠️ Partial | Works for some languages |

**Overall: ~30% Complete** (Structure exists but core logic missing)

### 🔴 CRITICAL MISSING PIECES

1. **THE EXACT SYMBOL FORMAT IS NOT IMPLEMENTED**
   - This is the #1 requirement from Codex
   - AI expects: `"1--10 | function myFunc()"`
   - We return: Rust structs

2. **processCaptures LOGIC NOT IMPLEMENTED**
   - This is the core of symbol extraction
   - Handles filtering, deduplication, formatting
   - Must be translated exactly from TypeScript

3. **NO MARKDOWN SUPPORT**
   - Critical for documentation parsing
   - Special handling required

4. **MISSING 21+ LANGUAGES**
   - We have 17, need 38+
   - Many parsers disabled due to version conflicts

### 📝 WHAT NEEDS TO BE DONE

1. **IMMEDIATE (Must Have)**:
   - [ ] Implement exact `"line--line | text"` format
   - [ ] Port processCaptures function from Codex
   - [ ] Fix 1-indexed line numbers
   - [ ] Add MIN_COMPONENT_LINES = 4 filtering
   - [ ] Implement markdown parser

2. **HIGH PRIORITY**:
   - [ ] Add HTML filtering for JSX/TSX
   - [ ] Implement directory traversal with 50-file limit
   - [ ] Port exact query patterns from Codex
   - [ ] Fix Java performance (>10K lines/sec)

3. **MEDIUM PRIORITY**:
   - [ ] Add missing 21+ languages
   - [ ] Complete extension mappings
   - [ ] Add gitignore support
   - [ ] Test memory usage

### CONCLUSION

The current implementation has the **structure** but is missing the **core logic**. The most critical issue is that we're not outputting symbols in the exact format that the AI expects. Without the `"startLine--endLine | definition_text"` format, the integration WILL NOT WORK with the AI system.

**Estimated completion: 30%**
**Estimated work remaining: 40-60 hours** to implement all missing pieces correctly.
