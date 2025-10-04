# Step 7: Tree-sitter Integration - Native Parsing for 30-50+ Languages
## Replace WASM with Native Parsers

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED :  1:1 TYPESCRIPT TO RUST TRANSLATION ONLY
**YEARS OF PERFECTED PARSING LOGIC - TRANSLATE EXACTLY**

**STUDY CODEX REFERENCE**:  `/home/verma/lapce/Codex`
- Symbol format took YEARS to perfect - copy exactly
- Context extraction is battle-tested - preserve ALL logic
- Just change TypeScript → Rust syntax, nothing else

## ✅ Success Criteria
- [ ] **Memory Usage**: < 5MB for all language parsers
- [ ] **Parse Speed**: > 10K lines/second
- [ ] **Language Support**: 100+ programming languages
- [ ] **Incremental Parsing**: < 10ms for small edits
- [ ] **Symbol Extraction**: < 50ms for 1K line file
- [ ] **Cache Hit Rate**: > 90% for unchanged files
- [ ] **Query Performance**: < 1ms for syntax queries
- [ ] **Test Coverage**: Parse 1M+ lines without errors

## Overview
Native tree-sitter integration replaces 38 WASM modules, providing 10x faster parsing with 90% memory reduction through direct FFI bindings and shared parser instances.

## CRITICAL: Symbol Format from Codex
```typescript
// From codex-reference - AI expects this EXACT format:
// Classes: "class MyClass"
// Functions: "function myFunc()"
// Methods: "MyClass.method()"
// Variables: "const myVar"
```

## Core Architecture

### Native Parser Manager
```rust
use tree_sitter::{Parser, Tree, Language, Query, QueryCursor};
use std::collections::HashMap;
use arc_swap::ArcSwap;

pub struct NativeParserManager {
    // Language parsers (shared instances)
    parsers: DashMap<FileType, Arc<Parser>>,
    
    // Compiled queries for each language
    queries: DashMap<FileType, Arc<CompiledQueries>>,
    
    // Tree cache with incremental parsing
    tree_cache: Arc<TreeCache>,
    
    // Language detection
    detector: LanguageDetector,
    
    // Metrics
    metrics: Arc<ParserMetrics>,
}

pub struct CompiledQueries {
    highlights: Query,
    locals: Query,
    injections: Query,
    tags: Query,
    folds: Query,
}

pub struct TreeCache {
    cache: moka::sync::Cache<PathBuf, CachedTree>,
    max_size: usize,
}

pub struct CachedTree {
    tree: Tree,
    source: Bytes,
    version: u64,
    last_modified: SystemTime,
}
```

## Language Loading

### 1. Dynamic Language Loading
```rust
impl NativeParserManager {
    pub fn new() -> Result<Self> {
        let mut parsers = DashMap::new();
        let mut queries = DashMap::new();
        
        // Load all supported languages
        for lang_type in FileType::iter() {
            let parser = Self::load_language(lang_type)?;
            parsers.insert(lang_type, Arc::new(parser));
            
            let compiled_queries = Self::load_queries(lang_type)?;
            queries.insert(lang_type, Arc::new(compiled_queries));
        }
        
        Ok(Self {
            parsers,
            queries,
            tree_cache: Arc::new(TreeCache::new(100)), // Cache 100 trees
            detector: LanguageDetector::new(),
            metrics: Arc::new(ParserMetrics::new()),
        })
    }
    
    fn load_language(file_type: FileType) -> Result<Parser> {
        let language = match file_type {
            FileType::Rust => tree_sitter_rust::language(),
            FileType::JavaScript => tree_sitter_javascript::language(),
            FileType::TypeScript => tree_sitter_typescript::language_typescript(),
            FileType::Python => tree_sitter_python::language(),
            FileType::Go => tree_sitter_go::language(),
            FileType::Cpp => tree_sitter_cpp::language(),
            FileType::Java => tree_sitter_java::language(),
            FileType::CSharp => tree_sitter_c_sharp::language(),
            FileType::Ruby => tree_sitter_ruby::language(),
            FileType::Php => tree_sitter_php::language(),
            FileType::Swift => tree_sitter_swift::language(),
            FileType::Kotlin => tree_sitter_kotlin::language(),
            FileType::Scala => tree_sitter_scala::language(),
            FileType::Haskell => tree_sitter_haskell::language(),
            FileType::Elixir => tree_sitter_elixir::language(),
            FileType::Clojure => tree_sitter_clojure::language(),
            FileType::Zig => tree_sitter_zig::language(),
            FileType::Lua => tree_sitter_lua::language(),
            FileType::Bash => tree_sitter_bash::language(),
            FileType::Html => tree_sitter_html::language(),
            FileType::Css => tree_sitter_css::language(),
            FileType::Json => tree_sitter_json::language(),
            FileType::Yaml => tree_sitter_yaml::language(),
            FileType::Toml => tree_sitter_toml::language(),
            FileType::Markdown => tree_sitter_markdown::language(),
            _ => return Err(Error::UnsupportedLanguage(file_type)),
        };
        
        let mut parser = Parser::new();
        parser.set_language(language)?;
        Ok(parser)
    }
    
    fn load_queries(file_type: FileType) -> Result<CompiledQueries> {
        // Load query files from embedded resources
        let highlights_query = Self::load_query_file(file_type, "highlights.scm")?;
        let locals_query = Self::load_query_file(file_type, "locals.scm")?;
        let injections_query = Self::load_query_file(file_type, "injections.scm")?;
        let tags_query = Self::load_query_file(file_type, "tags.scm")?;
        let folds_query = Self::load_query_file(file_type, "folds.scm")?;
        
        let language = Self::get_language(file_type)?;
        
        Ok(CompiledQueries {
            highlights: Query::new(language, &highlights_query)?,
            locals: Query::new(language, &locals_query)?,
            injections: Query::new(language, &injections_query)?,
            tags: Query::new(language, &tags_query)?,
            folds: Query::new(language, &folds_query)?,
        })
    }
}
```

### 2. Incremental Parsing
```rust
impl NativeParserManager {
    pub async fn parse_file(&self, path: &Path) -> Result<ParseResult> {
        let start = Instant::now();
        
        // Detect language
        let file_type = self.detector.detect(path)?;
        
        // Get parser for language
        let parser = self.parsers
            .get(&file_type)
            .ok_or(Error::NoParserForLanguage(file_type))?
            .clone();
            
        // Read file content
        let content = tokio::fs::read(path).await?;
        let content_bytes = Bytes::from(content);
        
        // Check cache
        if let Some(cached) = self.tree_cache.get(path).await {
            if cached.is_valid(&content_bytes) {
                self.metrics.record_cache_hit();
                return Ok(ParseResult::from_cached(cached));
            }
        }
        
        // Parse with incremental parsing if possible
        let tree = if let Some(old_tree) = self.tree_cache.get_tree(path).await {
            self.parse_incremental(parser, &content_bytes, old_tree)?
        } else {
            self.parse_full(parser, &content_bytes)?
        };
        
        // Cache the tree
        self.tree_cache.insert(path.to_owned(), CachedTree {
            tree: tree.clone(),
            source: content_bytes.clone(),
            version: self.compute_version(&content_bytes),
            last_modified: SystemTime::now(),
        }).await;
        
        // Record metrics
        self.metrics.record_parse(start.elapsed(), content_bytes.len());
        
        Ok(ParseResult {
            tree,
            source: content_bytes,
            file_type,
            parse_time: start.elapsed(),
        })
    }
    
    fn parse_incremental(
        &self,
        mut parser: Arc<Parser>,
        content: &[u8],
        old_tree: Tree,
    ) -> Result<Tree> {
        // Get mutable parser
        let parser = Arc::make_mut(&mut parser);
        
        // Parse with old tree for incremental parsing
        parser.parse(content, Some(&old_tree))
            .ok_or(Error::ParseFailed)
    }
    
    fn parse_full(&self, mut parser: Arc<Parser>, content: &[u8]) -> Result<Tree> {
        let parser = Arc::make_mut(&mut parser);
        parser.parse(content, None)
            .ok_or(Error::ParseFailed)
    }
}
```

## Symbol Extraction

### 1. Fast Symbol Extraction
```rust
pub struct SymbolExtractor {
    parser_manager: Arc<NativeParserManager>,
    symbol_cache: Arc<SymbolCache>,
}

impl SymbolExtractor {
    pub async fn extract_symbols(&self, path: &Path) -> Result<Vec<Symbol>> {
        // Parse file
        let parse_result = self.parser_manager.parse_file(path).await?;
        
        // Get queries for language
        let queries = self.parser_manager
            .queries
            .get(&parse_result.file_type)
            .ok_or(Error::NoQueriesForLanguage)?;
            
        // Extract symbols using tree-sitter queries
        let mut cursor = QueryCursor::new();
        let mut symbols = Vec::new();
        
        let matches = cursor.matches(
            &queries.tags,
            parse_result.tree.root_node(),
            parse_result.source.as_ref(),
        );
        
        for match_ in matches {
            for capture in match_.captures {
                let node = capture.node;
                let symbol = self.create_symbol(node, &parse_result.source)?;
                symbols.push(symbol);
            }
        }
        
        // Cache symbols
        self.symbol_cache.insert(path.to_owned(), symbols.clone()).await;
        
        Ok(symbols)
    }
    
    fn create_symbol(&self, node: Node, source: &[u8]) -> Result<Symbol> {
        let name = node.utf8_text(source)?;
        let kind = self.determine_symbol_kind(node);
        
        Ok(Symbol {
            name: name.to_string(),
            kind,
            range: Range {
                start: Position {
                    line: node.start_position().row,
                    column: node.start_position().column,
                },
                end: Position {
                    line: node.end_position().row,
                    column: node.end_position().column,
                },
            },
            children: Vec::new(),
        })
    }
    
    fn determine_symbol_kind(&self, node: Node) -> SymbolKind {
        match node.kind() {
            "function_declaration" | "method_definition" => SymbolKind::Function,
            "class_declaration" | "struct_item" => SymbolKind::Class,
            "interface_declaration" | "trait_item" => SymbolKind::Interface,
            "variable_declaration" | "let_declaration" => SymbolKind::Variable,
            "const_declaration" | "const_item" => SymbolKind::Constant,
            "enum_declaration" | "enum_item" => SymbolKind::Enum,
            "module" | "mod_item" => SymbolKind::Module,
            _ => SymbolKind::Unknown,
        }
    }
}
```

## Syntax Highlighting

### 1. Efficient Highlighting
```rust
pub struct SyntaxHighlighter {
    parser_manager: Arc<NativeParserManager>,
    theme: Arc<Theme>,
}

impl SyntaxHighlighter {
    pub async fn highlight(&self, path: &Path) -> Result<Vec<HighlightedRange>> {
        let parse_result = self.parser_manager.parse_file(path).await?;
        
        let queries = self.parser_manager
            .queries
            .get(&parse_result.file_type)
            .ok_or(Error::NoQueriesForLanguage)?;
            
        let mut cursor = QueryCursor::new();
        let mut highlights = Vec::new();
        
        // Use highlights query
        let matches = cursor.matches(
            &queries.highlights,
            parse_result.tree.root_node(),
            parse_result.source.as_ref(),
        );
        
        for match_ in matches {
            for capture in match_.captures {
                let capture_name = queries.highlights
                    .capture_names()[capture.index as usize];
                    
                let style = self.theme.get_style(capture_name);
                
                highlights.push(HighlightedRange {
                    start: capture.node.start_byte(),
                    end: capture.node.end_byte(),
                    style,
                });
            }
        }
        
        // Sort and merge overlapping ranges
        highlights.sort_by_key(|h| h.start);
        self.merge_overlapping(&mut highlights);
        
        Ok(highlights)
    }
    
    fn merge_overlapping(&self, highlights: &mut Vec<HighlightedRange>) {
        if highlights.len() < 2 {
            return;
        }
        
        let mut write_idx = 0;
        for read_idx in 1..highlights.len() {
            if highlights[write_idx].end >= highlights[read_idx].start {
                // Merge overlapping ranges
                highlights[write_idx].end = highlights[write_idx].end
                    .max(highlights[read_idx].end);
            } else {
                write_idx += 1;
                highlights[write_idx] = highlights[read_idx].clone();
            }
        }
        
        highlights.truncate(write_idx + 1);
    }
}
```

## Code Intelligence

### 1. Go to Definition
```rust
pub struct CodeIntelligence {
    parser_manager: Arc<NativeParserManager>,
    symbol_index: Arc<SymbolIndex>,
}

impl CodeIntelligence {
    pub async fn goto_definition(
        &self,
        path: &Path,
        position: Position,
    ) -> Result<Option<Location>> {
        let parse_result = self.parser_manager.parse_file(path).await?;
        
        // Find node at position
        let node = self.find_node_at_position(
            parse_result.tree.root_node(),
            position,
        )?;
        
        // Get symbol name
        let symbol_name = node.utf8_text(parse_result.source.as_ref())?;
        
        // Search for definition in symbol index
        let definition = self.symbol_index
            .find_definition(symbol_name)
            .await?;
            
        Ok(definition)
    }
    
    fn find_node_at_position(&self, root: Node, position: Position) -> Result<Node> {
        let mut cursor = root.walk();
        
        loop {
            let node = cursor.node();
            let start = node.start_position();
            let end = node.end_position();
            
            if position.line >= start.row && position.line <= end.row {
                if position.column >= start.column && position.column <= end.column {
                    // Found node containing position
                    if cursor.goto_first_child() {
                        continue; // Try to find more specific node
                    } else {
                        return Ok(node);
                    }
                }
            }
            
            if !cursor.goto_next_sibling() {
                if cursor.goto_parent() {
                    cursor.goto_next_sibling();
                } else {
                    break;
                }
            }
        }
        
        Err(Error::NoNodeAtPosition)
    }
}
```

## Performance Optimizations

### 1. Parser Pooling
```rust
pub struct ParserPool {
    pools: DashMap<FileType, Vec<Parser>>,
    max_per_type: usize,
}

impl ParserPool {
    pub fn acquire(&self, file_type: FileType) -> Result<PooledParser> {
        let mut pool = self.pools.entry(file_type).or_insert_with(Vec::new);
        
        let parser = if let Some(parser) = pool.pop() {
            parser
        } else {
            Self::create_parser(file_type)?
        };
        
        Ok(PooledParser {
            parser,
            file_type,
            pool: self,
        })
    }
    
    pub fn release(&self, file_type: FileType, parser: Parser) {
        let mut pool = self.pools.get_mut(&file_type).unwrap();
        
        if pool.len() < self.max_per_type {
            pool.push(parser);
        }
    }
}

pub struct PooledParser<'a> {
    parser: Parser,
    file_type: FileType,
    pool: &'a ParserPool,
}

impl<'a> Drop for PooledParser<'a> {
    fn drop(&mut self) {
        let parser = std::mem::replace(&mut self.parser, Parser::new());
        self.pool.release(self.file_type, parser);
    }
}
```

### 2. Query Result Caching
```rust
pub struct QueryCache {
    cache: moka::sync::Cache<QueryKey, Vec<QueryMatch>>,
}

#[derive(Hash, Eq, PartialEq)]
struct QueryKey {
    file_path: PathBuf,
    query_type: QueryType,
    file_hash: u64,
}

impl QueryCache {
    pub fn get_or_compute<F>(
        &self,
        key: QueryKey,
        compute: F,
    ) -> Result<Vec<QueryMatch>>
    where
        F: FnOnce() -> Result<Vec<QueryMatch>>,
    {
        self.cache.get_or_insert_with(key, compute)
    }
}
```

## Memory Profile
- **Parser instances**: 1MB (shared across files)
- **Compiled queries**: 500KB
- **Tree cache**: 2MB (100 trees)
- **Symbol cache**: 1MB
- **Query cache**: 500KB
- **Total**: ~5MB (vs 50MB WASM)
