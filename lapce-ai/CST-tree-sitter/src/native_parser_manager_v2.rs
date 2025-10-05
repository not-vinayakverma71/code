//! NATIVE PARSER MANAGER V2 - With Compact CST Support
//! Integrates dual representation for massive memory savings

use tree_sitter::{Parser, Tree, Language, Query, QueryCursor, Node};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, Instant};
use dashmap::DashMap;
use parking_lot::RwLock;
use bytes::Bytes;
use crate::dual_representation::{DualTree, DualNode, DualRepresentationConfig};
use crate::native_parser_manager::{FileType, CompiledQueries, LanguageDetector};

/// Enhanced parser manager with compact CST support
pub struct NativeParserManagerV2 {
    parsers: DashMap<FileType, Arc<RwLock<Parser>>>,
    queries: DashMap<FileType, Arc<CompiledQueries>>,
    
    // Cache using dual representation
    cache: Arc<DualTreeCache>,
    
    // Configuration for dual representation
    dual_config: DualRepresentationConfig,
    
    // Language detection
    pub detector: LanguageDetector,
    
    // Performance metrics
    metrics: Arc<ParserMetricsV2>,
}

/// Cache for dual tree representations
pub struct DualTreeCache {
    cache: moka::sync::Cache<PathBuf, CachedDualTree>,
    max_size: usize,
    
    // Statistics
    compact_count: Arc<RwLock<usize>>,
    standard_count: Arc<RwLock<usize>>,
    total_memory: Arc<RwLock<usize>>,
}

#[derive(Clone)]
pub struct CachedDualTree {
    pub tree: DualTree,
    pub source: Bytes,
    pub version: u64,
    pub last_modified: SystemTime,
    pub file_type: FileType,
}

pub struct ParseResultV2 {
    pub tree: DualTree,
    pub source: Bytes,
    pub file_type: FileType,
    pub parse_time: std::time::Duration,
    pub is_compact: bool,
}

impl NativeParserManagerV2 {
    /// Create new parser manager with dual representation support
    pub fn new(cache_size: usize) -> Self {
        Self::with_config(cache_size, DualRepresentationConfig::default())
    }
    
    /// Create with custom dual representation config
    pub fn with_config(cache_size: usize, dual_config: DualRepresentationConfig) -> Self {
        let cache = Arc::new(DualTreeCache::new(cache_size));
        
        Self {
            parsers: DashMap::new(),
            queries: DashMap::new(),
            cache,
            dual_config,
            detector: LanguageDetector::new(),
            metrics: Arc::new(ParserMetricsV2::new()),
        }
    }
    
    /// Parse a file with automatic representation selection
    pub fn parse_file(&self, path: &Path) -> anyhow::Result<ParseResultV2> {
        let start = Instant::now();
        
        // Read file
        let source = std::fs::read(path)?;
        let source_len = source.len();
        let source_bytes = Bytes::from(source);
        
        // Detect language
        let file_type = self.detector.detect(path)
            .map_err(|e| anyhow::anyhow!("Language detection failed: {}", e))?;
        
        // Get or create parser
        let parser = self.get_or_create_parser(file_type)?;
        
        // Parse with Tree-sitter
        let ts_tree = parser.write()
            .parse(&source_bytes, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse"))?;
        
        // Decide on representation
        let tree = if self.should_use_compact(source_len, file_type) {
            DualTree::from_tree_sitter_compact(&ts_tree, &source_bytes)
        } else {
            DualTree::from_tree_sitter(ts_tree)
        };
        
        let is_compact = tree.is_compact();
        let parse_time = start.elapsed();
        
        // Update metrics
        self.metrics.record_parse(parse_time, source_len);
        
        // Cache the result
        let cached = CachedDualTree {
            tree: tree.clone(),
            source: source_bytes.clone(),
            version: 1,
            last_modified: SystemTime::now(),
            file_type,
        };
        
        self.cache.insert(path.to_path_buf(), cached);
        
        Ok(ParseResultV2 {
            tree,
            source: source_bytes,
            file_type,
            parse_time,
            is_compact,
        })
    }
    
    /// Determine if compact representation should be used
    fn should_use_compact(&self, source_len: usize, _file_type: FileType) -> bool {
        if !self.dual_config.auto_compact {
            return false;
        }
        
        // Use compact for files larger than threshold
        source_len > self.dual_config.compact_threshold
    }
    
    /// Get or create parser for file type
    fn get_or_create_parser(&self, file_type: FileType) -> anyhow::Result<Arc<RwLock<Parser>>> {
        if let Some(parser) = self.parsers.get(&file_type) {
            return Ok(parser.clone());
        }
        
        let language = self.get_language(file_type)?;
        let mut parser = Parser::new();
        parser.set_language(&language)?;
        
        let parser = Arc::new(RwLock::new(parser));
        self.parsers.insert(file_type, parser.clone());
        
        Ok(parser)
    }
    
    /// Get language for file type
    fn get_language(&self, file_type: FileType) -> anyhow::Result<Language> {
        let language = match file_type {
            FileType::Rust => tree_sitter_rust::LANGUAGE.into(),
            FileType::Python => tree_sitter_python::LANGUAGE.into(),
            FileType::JavaScript => tree_sitter_javascript::language(),
            FileType::TypeScript => tree_sitter_typescript::language_typescript(),
            FileType::Go => tree_sitter_go::LANGUAGE.into(),
            FileType::C => tree_sitter_c::LANGUAGE.into(),
            FileType::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            FileType::Java => tree_sitter_java::LANGUAGE.into(),
            _ => return Err(anyhow::anyhow!("Language not supported")),
        };
        Ok(language)
    }
    
    /// Compact all cached trees to save memory
    pub fn compact_all(&self) {
        let entries: Vec<_> = self.cache.cache.iter()
            .map(|(k, v)| ((*k).clone(), v.clone()))
            .collect();
        
        for (path, mut cached) in entries {
            if !cached.tree.is_compact() {
                cached.tree = cached.tree.to_compact(&cached.source);
                self.cache.insert(path, cached);
            }
        }
    }
    
    /// Get memory statistics
    pub fn memory_stats(&self) -> MemoryStats {
        let compact = *self.cache.compact_count.read();
        let standard = *self.cache.standard_count.read();
        let total_memory = *self.cache.total_memory.read();
        
        MemoryStats {
            compact_trees: compact,
            standard_trees: standard,
            total_memory_bytes: total_memory,
            average_bytes_per_tree: if compact + standard > 0 {
                total_memory / (compact + standard)
            } else {
                0
            },
        }
    }
}

impl DualTreeCache {
    fn new(max_size: usize) -> Self {
        let cache = moka::sync::Cache::builder()
            .max_capacity(max_size as u64)
            .build();
        
        Self {
            cache,
            max_size,
            compact_count: Arc::new(RwLock::new(0)),
            standard_count: Arc::new(RwLock::new(0)),
            total_memory: Arc::new(RwLock::new(0)),
        }
    }
    
    fn insert(&self, path: PathBuf, tree: CachedDualTree) {
        // Update statistics
        {
            let mut compact = self.compact_count.write();
            let mut standard = self.standard_count.write();
            let mut memory = self.total_memory.write();
            
            if tree.tree.is_compact() {
                *compact += 1;
            } else {
                *standard += 1;
            }
            
            *memory += tree.tree.memory_bytes();
        }
        
        self.cache.insert(path, tree);
    }
    
    pub fn get(&self, path: &Path) -> Option<CachedDualTree> {
        self.cache.get(path)
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub compact_trees: usize,
    pub standard_trees: usize,
    pub total_memory_bytes: usize,
    pub average_bytes_per_tree: usize,
}

/// Performance metrics for V2 parser
pub struct ParserMetricsV2 {
    cache_hits: Arc<RwLock<u64>>,
    cache_misses: Arc<RwLock<u64>>,
    parse_times: Arc<RwLock<Vec<std::time::Duration>>>,
    bytes_parsed: Arc<RwLock<u64>>,
}

impl ParserMetricsV2 {
    fn new() -> Self {
        Self {
            cache_hits: Arc::new(RwLock::new(0)),
            cache_misses: Arc::new(RwLock::new(0)),
            parse_times: Arc::new(RwLock::new(Vec::new())),
            bytes_parsed: Arc::new(RwLock::new(0)),
        }
    }
    
    fn record_parse(&self, time: std::time::Duration, bytes: usize) {
        let mut times = self.parse_times.write();
        times.push(time);
        
        let mut total_bytes = self.bytes_parsed.write();
        *total_bytes += bytes as u64;
    }
}
