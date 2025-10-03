# Step 16: Symbol Extraction & Code Search
## Tree-sitter Integration, Symbol Indexing, Codebase Search

## ⚠️ CRITICAL: 1:1 TYPESCRIPT TO RUST PORT ONLY
**PRESERVE EXACT SYMBOL FORMATS FOR AI RECOGNITION**

**TRANSLATE LINE-BY-LINE FROM**:
- `/home/verma/lapce/lapce-ai-rust/codex-reference/tools/listCodeDefinitionNamesTool.ts`
- `/home/verma/lapce/lapce-ai-rust/codex-reference/tools/codebaseSearchTool.ts`
- `/home/verma/lapce/lapce-ai-rust/codex-reference/services/tree-sitter/`
- `/home/verma/lapce/lapce-ai-rust/codex-reference/context/`

## ✅ Success Criteria
- [ ] **Memory Usage**: < 5MB for all parsers
- [ ] **Parse Speed**: > 10K lines/second
- [ ] **Symbol Format**: EXACT match with TypeScript
- [ ] **Search Ranking**: Same algorithm as Codex
- [ ] **Language Support**: All 25+ languages
- [ ] **Incremental Parsing**: < 10ms updates
- [ ] **Cache Hit Rate**: > 90%
- [ ] **Test Coverage**: 100% symbol format match

## Overview
Symbol extraction MUST produce EXACT format that AI expects. Years of training depend on this format.

## Symbol Format (DO NOT CHANGE)

### Expected Format from TypeScript
```typescript
// From codex-reference/tools/listCodeDefinitionNamesTool.ts
// AI expects EXACTLY this format:
"class MyClass"
"function myFunction()"
"MyClass.method()"
"const myVariable"
"interface MyInterface"
"type MyType"
```

### Rust Translation
```rust
pub struct SymbolExtractor {
    parsers: HashMap<Language, Parser>,
    queries: HashMap<Language, Query>,
}

impl SymbolExtractor {
    pub fn extract_symbols(&self, code: &str, language: Language) -> Vec<Symbol> {
        // EXACT symbol format from TypeScript
        let tree = self.parsers.get(&language).unwrap().parse(code, None);
        let mut cursor = QueryCursor::new();
        let query = &self.queries[&language];
        
        let mut symbols = Vec::new();
        for match_ in cursor.matches(query, tree.root_node(), code.as_bytes()) {
            // Format EXACTLY as TypeScript does
            let symbol = self.format_symbol(match_, code);
            symbols.push(symbol);
        }
        
        symbols
    }
    
    fn format_symbol(&self, match_: Match, source: &str) -> Symbol {
        // CHARACTER-FOR-CHARACTER format
        match match_.pattern_index {
            0 => { // Class
                format!("class {}", self.get_name(match_, source))
            },
            1 => { // Function
                format!("function {}()", self.get_name(match_, source))
            },
            2 => { // Method
                format!("{}.{}()", self.get_class(match_, source), self.get_name(match_, source))
            },
            // ... EXACT formatting for each type
        }
    }
}
```

## Tree-sitter Queries (PRESERVE EXACTLY)

### Language-Specific Patterns
```rust
lazy_static! {
    static ref QUERIES: HashMap<Language, &'static str> = {
        let mut m = HashMap::new();
        
        // EXACT queries from TypeScript
        m.insert(Language::Rust, r#"
            (struct_item name: (identifier) @class.name)
            (function_item name: (identifier) @function.name)
            (impl_item type: (type_identifier) @class
                (function_item name: (identifier) @method.name))
        "#);
        
        m.insert(Language::TypeScript, r#"
            (class_declaration name: (identifier) @class.name)
            (function_declaration name: (identifier) @function.name)
            (method_definition name: (property_identifier) @method.name)
        "#);
        
        // ... ALL languages from TypeScript
        m
    };
}
```

## Codebase Search (EXACT ALGORITHM)

### Search Ranking from TypeScript
```typescript
// From codex-reference/tools/codebaseSearchTool.ts
// Ranking factors (MUST PRESERVE):
// 1. Exact match: 1.0
// 2. Start of word: 0.8
// 3. CamelCase match: 0.7
// 4. Fuzzy match: 0.5
```

### Rust Translation
```rust
pub struct CodebaseSearcher {
    symbol_index: Arc<SymbolIndex>,
    file_index: Arc<FileIndex>,
    ranking_weights: RankingWeights,
}

impl CodebaseSearcher {
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        // EXACT ranking algorithm from TypeScript
        let mut results = Vec::new();
        
        // Search symbols
        for symbol in self.symbol_index.search(query) {
            let score = self.calculate_score(&symbol, query);
            results.push(SearchResult {
                path: symbol.file_path.clone(),
                symbol: symbol.name.clone(),
                score,
                line: symbol.line,
            });
        }
        
        // Sort EXACTLY as TypeScript does
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap()
        });
        
        results.truncate(limit);
        results
    }
    
    fn calculate_score(&self, symbol: &Symbol, query: &str) -> f32 {
        // EXACT scoring from TypeScript
        if symbol.name == query {
            1.0 // Exact match
        } else if symbol.name.starts_with(query) {
            0.8 // Prefix match
        } else if self.is_camelcase_match(&symbol.name, query) {
            0.7 // CamelCase match
        } else {
            0.5 // Fuzzy match
        }
    }
}
```

## Incremental Parsing (PRESERVE LOGIC)

```rust
pub struct IncrementalParser {
    trees: HashMap<PathBuf, Tree>,
    versions: HashMap<PathBuf, u64>,
}

impl IncrementalParser {
    pub fn update(&mut self, path: &Path, edit: InputEdit, new_text: &str) -> Tree {
        // EXACT incremental parsing from TypeScript
        if let Some(old_tree) = self.trees.get_mut(path) {
            old_tree.edit(&edit);
            let parser = self.get_parser_for_file(path);
            let new_tree = parser.parse(new_text, Some(old_tree)).unwrap();
            self.trees.insert(path.to_owned(), new_tree.clone());
            new_tree
        } else {
            self.parse_full(path, new_text)
        }
    }
}
```

## Language Detection (SAME MAPPINGS)

```rust
pub fn detect_language(path: &Path) -> Language {
    // EXACT file extension mappings from TypeScript
    match path.extension().and_then(|s| s.to_str()) {
        Some("rs") => Language::Rust,
        Some("ts") | Some("tsx") => Language::TypeScript,
        Some("js") | Some("jsx") => Language::JavaScript,
        Some("py") => Language::Python,
        Some("go") => Language::Go,
        Some("java") => Language::Java,
        Some("c") => Language::C,
        Some("cpp") | Some("cc") | Some("cxx") => Language::Cpp,
        // ... ALL mappings from TypeScript
        _ => Language::PlainText,
    }
}
```

## Symbol Index Storage

```rust
pub struct SymbolIndex {
    symbols: DashMap<PathBuf, Vec<Symbol>>,
    global_index: Arc<RwLock<HashMap<String, Vec<SymbolLocation>>>>,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub column: usize,
    pub file_path: PathBuf,
    pub scope: Option<String>,
}

#[derive(Clone, Debug)]
pub enum SymbolKind {
    Class,
    Function,
    Method,
    Variable,
    Interface,
    Type,
    // EXACT kinds from TypeScript
}
```

## Testing Requirements

```rust
#[test]
fn symbol_format_matches_typescript() {
    let code = "class MyClass { method() {} }";
    let symbols = extractor.extract_symbols(code, Language::TypeScript);
    
    // MUST match TypeScript output EXACTLY
    assert_eq!(symbols[0].to_string(), "class MyClass");
    assert_eq!(symbols[1].to_string(), "MyClass.method()");
}

#[test]
fn search_ranking_identical() {
    let results = searcher.search("getData", 10);
    let ts_results = load_typescript_results("search_getData.json");
    
    // Same order, same scores
    for (rust_result, ts_result) in results.iter().zip(ts_results.iter()) {
        assert_eq!(rust_result.symbol, ts_result.symbol);
        assert!((rust_result.score - ts_result.score).abs() < 0.001);
    }
}
```

## Implementation Checklist
- [ ] Port listCodeDefinitionNamesTool.ts exactly
- [ ] Port codebaseSearchTool.ts exactly  
- [ ] Preserve symbol format CHARACTER-FOR-CHARACTER
- [ ] Match search ranking algorithm
- [ ] Support all 25+ languages
- [ ] Implement incremental parsing
- [ ] Cache parsed trees
- [ ] Test against TypeScript output
