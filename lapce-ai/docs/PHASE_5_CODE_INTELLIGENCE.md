# Phase 5: Code Intelligence & Analysis (2 weeks)
## Deep Code Understanding with Minimal Resources

## 🎯 STRICT SUCCESS CRITERIA - MUST ACHIEVE ALL
- [ ] **Memory Target**: < 5MB for all parsers and intelligence
- [ ] **Context Accuracy**: 100% same context selection as Codex
- [ ] **Token Counting**: EXACT tiktoken.ts behavior (character-match)
- [ ] **Symbol Extraction**: < 50ms for 1K line files
- [ ] **Parse Speed**: > 10K lines/second for all languages
- [ ] **Incremental Parsing**: < 10ms for small edits
- [ ] **Context Window**: Exact same limits per model (128k, 200k, etc.)
- [ ] **Stress Test**: Parse 1M+ lines without memory growth

⚠️ **GATE**: Phase 6 starts ONLY when context management is IDENTICAL to Codex.

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED : 1:1 TRANSLATION - YEARS OF AI DEVELOPMENT
**DO NOT REWRITE - ONLY TRANSLATE TYPESCRIPT TO RUST**

**LINE-BY-LINE TRANSLATION FROM**:
- `/home/verma/lapce/lapce-ai-rust/codex-reference/context/` - Port every function
- `/home/verma/lapce/lapce-ai-rust/codex-reference/sliding-window/` - Same algorithms
- `/home/verma/lapce/lapce-ai-rust/codex-reference/tools/codebaseSearchTool.ts` - Exact search
- `/home/verma/lapce/lapce-ai-rust/codex-reference/tools/listCodeDefinitionNamesTool.ts` - Same format

**TRANSLATION MANDATE**:
- Copy each TypeScript function → Rust function
- Same logic flow, same decisions
- Same variable names (snake_case)
- Same algorithms (no "improvements")
- Same data structures
- Same ranking/scoring/prioritization
- This AI is PERFECT after years of tuning - just change syntax

### Week 1: Advanced Tree-sitter Integration
**Goal:** Complete language support with incremental parsing
**Memory Target:** < 5MB for all parsers

### Multi-Language Parser System (MATCH CODEX BEHAVIOR)
```rust
// Context extraction MUST match codex-reference/context/
// Token counting MUST match codex-reference/tiktoken.ts
// DO NOT change how context is selected or formatted
use tree_sitter::{Parser, Tree, Query, QueryCursor, Language};
use dashmap::DashMap;
use arc_swap::ArcSwap;

pub struct CodeIntelligence {
    parsers: DashMap<Language, Arc<Parser>>,
    trees: DashMap<PathBuf, ArcSwap<Tree>>,
    queries: DashMap<Language, CompiledQueries>,
    symbol_index: Arc<SymbolIndex>,
}

pub struct CompiledQueries {
    highlights: Query,
    locals: Query,
    definitions: Query,
    references: Query,
    implementations: Query,
    call_hierarchy: Query,
}

impl CodeIntelligence {
    pub fn new() -> Result<Self> {
        let mut parsers = DashMap::new();
        let mut queries = DashMap::new();
        
        // Load all language parsers (native, not WASM)
        for lang in &[
            ("rust", tree_sitter_rust::language()),
            ("typescript", tree_sitter_typescript::language_typescript()),
            ("javascript", tree_sitter_javascript::language()),
            ("python", tree_sitter_python::language()),
            ("go", tree_sitter_go::language()),
            ("java", tree_sitter_java::language()),
            ("cpp", tree_sitter_cpp::language()),
            ("c", tree_sitter_c::language()),
        ] {
            let mut parser = Parser::new();
            parser.set_language(lang.1)?;
            parsers.insert(lang.0.to_string(), Arc::new(parser));
            
            // Pre-compile queries for each language
            queries.insert(lang.0.to_string(), Self::load_queries(lang.1)?);
        }
        
        Ok(Self {
            parsers,
            trees: DashMap::new(),
            queries,
            symbol_index: Arc::new(SymbolIndex::new()),
        })
    }
    
    pub async fn parse_incrementally(&self, path: &Path, edit: Edit) -> Result<Tree> {
        let content = self.read_file_mmap(path)?;
        let language = self.detect_language(path)?;
        let parser = self.parsers.get(&language).unwrap();
        
        // Get existing tree for incremental parsing
        let new_tree = if let Some(old_tree_ref) = self.trees.get(path) {
            let old_tree = old_tree_ref.load();
            
            // Apply edit to old tree
            old_tree.edit(&edit);
            
            // Parse incrementally
            parser.parse(&content, Some(&old_tree))?
        } else {
            // Initial parse
            parser.parse(&content, None)?
        };
        
        // Store new tree
        self.trees.entry(path.to_owned())
            .and_modify(|t| t.store(Arc::new(new_tree.clone())))
            .or_insert(ArcSwap::from(Arc::new(new_tree.clone())));
            
        Ok(new_tree)
    }
}
```

### Symbol Extraction & Indexing (EXACT FORMAT FROM CODEX)
```rust
// READ: codex-reference/tools/listCodeDefinitionNamesTool.ts
// Symbol format MUST match exactly for AI to recognize
pub struct SymbolIndex {
    symbols: DashMap<PathBuf, Vec<Symbol>>,
    global_index: Arc<RwLock<HashMap<String, Vec<SymbolLocation>>>>,
    type_hierarchy: DashMap<String, TypeInfo>,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    name: String,
    kind: SymbolKind,
    range: Range,
    children: Vec<Symbol>,
    signature: Option<String>,
    doc_comment: Option<String>,
}

impl SymbolIndex {
    pub async fn extract_symbols(&self, path: &Path, tree: &Tree) -> Result<Vec<Symbol>> {
        let query = self.get_symbol_query(path)?;
        let mut cursor = QueryCursor::new();
        let source = self.read_source(path)?;
        
        let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
        let mut symbols = Vec::new();
        
        for match_ in matches {
            for capture in match_.captures {
                let node = capture.node;
                let symbol_name = node.utf8_text(source.as_bytes())?;
                
                // Extract symbol with context
                let symbol = Symbol {
                    name: symbol_name.to_string(),
                    kind: Self::node_to_symbol_kind(node),
                    range: Self::node_to_range(node),
                    children: self.extract_children(node, source.as_bytes())?,
                    signature: self.extract_signature(node, source.as_bytes()),
                    doc_comment: self.extract_doc_comment(node, source.as_bytes()),
                };
                
                symbols.push(symbol);
            }
        }
        
        // Update global index
        self.update_global_index(path, &symbols).await;
        
        Ok(symbols)
    }
    
    pub async fn find_references(&self, symbol: &str) -> Vec<SymbolLocation> {
        let index = self.global_index.read().await;
        index.get(symbol).cloned().unwrap_or_default()
    }
}
```

### Week 1.5: Semantic Analysis
**Goal:** Type inference, call graphs, dependency analysis
**Memory Target:** < 3MB

```rust
pub struct SemanticAnalyzer {
    type_cache: Arc<DashMap<String, ResolvedType>>,
    call_graph: Arc<CallGraph>,
    import_graph: Arc<ImportGraph>,
}

pub struct CallGraph {
    edges: DashMap<FunctionId, Vec<FunctionId>>,
    reverse_edges: DashMap<FunctionId, Vec<FunctionId>>,
}

impl SemanticAnalyzer {
    pub async fn analyze_file(&self, path: &Path, tree: &Tree) -> SemanticInfo {
        let mut analyzer = FileAnalyzer::new(tree);
        
        // Extract type information
        let types = analyzer.extract_types();
        
        // Build call graph
        let calls = analyzer.extract_function_calls();
        for (caller, callees) in calls {
            self.call_graph.add_edges(caller, callees);
        }
        
        // Extract imports/dependencies
        let imports = analyzer.extract_imports();
        self.import_graph.add_file_imports(path, imports);
        
        SemanticInfo {
            types,
            call_sites: analyzer.call_sites,
            imports: analyzer.imports,
            exports: analyzer.exports,
        }
    }
    
    pub async fn infer_type(&self, expr: &Expression) -> Option<ResolvedType> {
        // Check cache first
        if let Some(cached) = self.type_cache.get(&expr.id()) {
            return Some(cached.clone());
        }
        
        // Perform type inference
        let inferred = match expr {
            Expression::Literal(lit) => self.infer_literal_type(lit),
            Expression::Variable(var) => self.lookup_variable_type(var),
            Expression::FunctionCall(call) => self.infer_return_type(call),
            Expression::MemberAccess(obj, member) => {
                let obj_type = self.infer_type(obj).await?;
                self.lookup_member_type(&obj_type, member)
            }
            _ => None,
        };
        
        // Cache result
        if let Some(ref type_) = inferred {
            self.type_cache.insert(expr.id(), type_.clone());
        }
        
        inferred
    }
}
```

### Week 2: Code Completion & Diagnostics
**Goal:** Intelligent completions, real-time error detection
**Memory Target:** < 2MB

```rust
pub struct CompletionEngine {
    symbol_index: Arc<SymbolIndex>,
    type_analyzer: Arc<SemanticAnalyzer>,
    snippet_store: Arc<SnippetStore>,
    frequency_tracker: Arc<FrequencyTracker>,
}

impl CompletionEngine {
    pub async fn get_completions(&self, position: Position) -> Vec<CompletionItem> {
        let context = self.extract_context(position).await;
        let mut completions = Vec::new();
        
        // 1. Local scope completions (highest priority)
        let local_symbols = self.symbol_index.get_local_symbols(&context).await;
        for symbol in local_symbols {
            completions.push(self.symbol_to_completion(symbol, CompletionPriority::Local));
        }
        
        // 2. Type-based completions
        if let Some(receiver_type) = context.receiver_type {
            let members = self.type_analyzer.get_type_members(&receiver_type).await;
            for member in members {
                completions.push(self.member_to_completion(member, CompletionPriority::TypeMember));
            }
        }
        
        // 3. Import completions
        let available_imports = self.symbol_index.get_importable_symbols(&context).await;
        for import in available_imports.iter().take(20) {
            completions.push(self.import_to_completion(import, CompletionPriority::Import));
        }
        
        // 4. Snippet completions
        let snippets = self.snippet_store.get_relevant_snippets(&context).await;
        for snippet in snippets {
            completions.push(self.snippet_to_completion(snippet));
        }
        
        // Sort by relevance
        self.rank_completions(&mut completions, &context);
        
        completions
    }
    
    fn rank_completions(&self, completions: &mut Vec<CompletionItem>, context: &Context) {
        completions.sort_by_cached_key(|item| {
            let mut score = 0i32;
            
            // Priority bonus
            score -= item.priority as i32 * 1000;
            
            // Frequency bonus
            score -= self.frequency_tracker.get_score(&item.label) * 100;
            
            // Prefix match bonus
            if item.label.starts_with(&context.prefix) {
                score -= 500;
            }
            
            // Type match bonus
            if let Some(expected_type) = &context.expected_type {
                if item.detail.as_ref() == Some(expected_type) {
                    score -= 300;
                }
            }
            
            score
        });
    }
}
```

### Real-time Diagnostics
```rust
pub struct DiagnosticEngine {
    parsers: Arc<CodeIntelligence>,
    type_checker: Arc<TypeChecker>,
    linters: Vec<Box<dyn Linter>>,
}

impl DiagnosticEngine {
    pub async fn check_file(&self, path: &Path) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        // 1. Syntax errors from tree-sitter
        let tree = self.parsers.parse_file(path).await?;
        if tree.root_node().has_error() {
            diagnostics.extend(self.extract_syntax_errors(&tree));
        }
        
        // 2. Type errors
        let type_errors = self.type_checker.check_file(path, &tree).await;
        diagnostics.extend(type_errors);
        
        // 3. Linting
        for linter in &self.linters {
            if linter.applies_to(path) {
                let lint_diagnostics = linter.lint(&tree, path).await;
                diagnostics.extend(lint_diagnostics);
            }
        }
        
        // Deduplicate and sort
        diagnostics.sort_by_key(|d| (d.range.start, d.severity));
        diagnostics.dedup();
        
        diagnostics
    }
}

// Example linter
pub struct UnusedVariableLinter;

impl Linter for UnusedVariableLinter {
    async fn lint(&self, tree: &Tree, path: &Path) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut declared_vars = HashSet::new();
        let mut used_vars = HashSet::new();
        
        // Walk tree to find declarations and usages
        let mut cursor = tree.walk();
        self.walk_tree(&mut cursor, &mut declared_vars, &mut used_vars);
        
        // Report unused variables
        for var in declared_vars.difference(&used_vars) {
            diagnostics.push(Diagnostic {
                range: var.range,
                severity: DiagnosticSeverity::Warning,
                message: format!("Variable '{}' is never used", var.name),
                code: Some("unused_variable".to_string()),
            });
        }
        
        diagnostics
    }
}
```

### Code Navigation
```rust
pub struct NavigationEngine {
    symbol_index: Arc<SymbolIndex>,
    call_graph: Arc<CallGraph>,
}

impl NavigationEngine {
    pub async fn goto_definition(&self, position: Position) -> Option<Location> {
        let symbol = self.symbol_at_position(position).await?;
        self.symbol_index.find_definition(&symbol).await
    }
    
    pub async fn find_implementations(&self, symbol: &Symbol) -> Vec<Location> {
        match symbol.kind {
            SymbolKind::Interface | SymbolKind::Trait => {
                self.symbol_index.find_implementations(&symbol.name).await
            }
            SymbolKind::Method => {
                self.symbol_index.find_overrides(&symbol.name).await
            }
            _ => vec![],
        }
    }
    
    pub async fn call_hierarchy(&self, function: &str) -> CallHierarchy {
        let callers = self.call_graph.get_callers(function).await;
        let callees = self.call_graph.get_callees(function).await;
        
        CallHierarchy {
            item: self.symbol_index.get_function(function).await,
            callers,
            callees,
        }
    }
}
```

## Memory Optimizations
1. **Shared Parsers**: One parser instance per language
2. **Incremental Parsing**: Only reparse changed portions
3. **Tree Caching**: Keep parsed trees in memory with LRU eviction
4. **Query Compilation**: Pre-compile all queries at startup
5. **Symbol Deduplication**: Intern all symbol names

## Dependencies
```toml
[dependencies]
# Tree-sitter parsers
tree-sitter = "0.23"
tree-sitter-rust = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-python = "0.23"
tree-sitter-go = "0.23"
tree-sitter-java = "0.23"
tree-sitter-cpp = "0.23"
tree-sitter-c = "0.23"

# Data structures
arc-swap = "1.7"
dashmap = "6.0"
```

## Expected Results - Phase 5
- **Total Memory**: < 10MB for all code intelligence
- **Parse Time**: < 10ms for 1000-line file
- **Incremental Parse**: < 1ms for small edits
- **Symbol Extraction**: < 5ms per file
- **Completion Latency**: < 20ms
- **Find References**: < 50ms in 100K LOC project
