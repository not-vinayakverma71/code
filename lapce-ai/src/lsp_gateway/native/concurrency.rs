/// Concurrency Model (LSP-032)
/// Parser pool integration, lock-free read paths, non-blocking operations

use std::sync::Arc;
use std::path::PathBuf;
use parking_lot::RwLock;
use dashmap::DashMap;
use anyhow::{Result, anyhow};

#[cfg(feature = "cst_integration")]
use lapce_tree_sitter::parser_pool::{ParserPool, FileType};

/// Concurrent document store with lock-free reads
pub struct ConcurrentDocumentStore {
    /// Document content indexed by URI
    documents: DashMap<String, Arc<DocumentData>>,
}

#[derive(Debug, Clone)]
pub struct DocumentData {
    pub uri: String,
    pub content: String,
    pub version: u32,
    pub language_id: String,
}

impl ConcurrentDocumentStore {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
        }
    }
    
    /// Lock-free read of document
    pub fn get(&self, uri: &str) -> Option<Arc<DocumentData>> {
        self.documents.get(uri).map(|entry| entry.value().clone())
    }
    
    /// Insert or update document
    pub fn insert(&self, uri: String, data: DocumentData) {
        self.documents.insert(uri, Arc::new(data));
    }
    
    /// Remove document
    pub fn remove(&self, uri: &str) -> Option<Arc<DocumentData>> {
        self.documents.remove(uri).map(|(_, v)| v)
    }
    
    /// Check if document exists (lock-free)
    pub fn contains(&self, uri: &str) -> bool {
        self.documents.contains_key(uri)
    }
    
    /// Get all URIs (snapshot)
    pub fn uris(&self) -> Vec<String> {
        self.documents.iter().map(|entry| entry.key().clone()).collect()
    }
    
    /// Count documents
    pub fn len(&self) -> usize {
        self.documents.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

impl Default for ConcurrentDocumentStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Concurrent parse tree cache
pub struct ParseTreeCache {
    /// Cached trees indexed by URI
    cache: DashMap<String, Arc<CachedTree>>,
    /// Maximum cache size
    max_size: usize,
}

#[derive(Debug, Clone)]
pub struct CachedTree {
    pub uri: String,
    pub version: u32,
    pub tree_json: String, // Serialized tree for lock-free access
    pub language_id: String,
}

impl ParseTreeCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: DashMap::new(),
            max_size,
        }
    }
    
    /// Lock-free read of cached tree
    pub fn get(&self, uri: &str, version: u32) -> Option<Arc<CachedTree>> {
        self.cache.get(uri).and_then(|entry| {
            let cached = entry.value();
            if cached.version == version {
                Some(cached.clone())
            } else {
                None
            }
        })
    }
    
    /// Insert cached tree with eviction
    pub fn insert(&self, cached: CachedTree) {
        // Simple eviction: if cache is full, remove random entry
        if self.cache.len() >= self.max_size {
            if let Some(entry) = self.cache.iter().next() {
                let key = entry.key().clone();
                drop(entry);
                self.cache.remove(&key);
            }
        }
        
        self.cache.insert(cached.uri.clone(), Arc::new(cached));
    }
    
    /// Invalidate cache entry
    pub fn invalidate(&self, uri: &str) {
        self.cache.remove(uri);
    }
    
    /// Clear all cache
    pub fn clear(&self) {
        self.cache.clear();
    }
    
    /// Get cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for ParseTreeCache {
    fn default() -> Self {
        Self::new(1000) // Cache up to 1000 parse trees
    }
}

/// Parser pool wrapper for LSP gateway
#[cfg(feature = "cst_integration")]
pub struct LspParserPool {
    pool: ParserPool,
}

#[cfg(feature = "cst_integration")]
impl LspParserPool {
    pub fn new(max_per_type: usize) -> Self {
        Self {
            pool: ParserPool::new(max_per_type),
        }
    }
    
    /// Acquire parser for language (non-blocking)
    pub fn acquire(&self, language_id: &str) -> Result<PooledParser<'_>> {
        let file_type = Self::language_to_file_type(language_id)?;
        let parser = self.pool.acquire(file_type)
            .map_err(|e| anyhow!("Failed to acquire parser: {}", e))?;
        Ok(PooledParser { inner: parser })
    }
    
    fn language_to_file_type(language_id: &str) -> Result<FileType> {
        match language_id {
            "rust" => Ok(FileType::Rust),
            "javascript" | "js" => Ok(FileType::JavaScript),
            "typescript" | "ts" => Ok(FileType::TypeScript),
            "python" | "py" => Ok(FileType::Python),
            "go" => Ok(FileType::Go),
            "cpp" | "c++" | "cxx" => Ok(FileType::Cpp),
            "java" => Ok(FileType::Java),
            _ => Ok(FileType::Other),
        }
    }
}

#[cfg(feature = "cst_integration")]
impl Default for LspParserPool {
    fn default() -> Self {
        Self::new(10) // 10 parsers per language type
    }
}

#[cfg(feature = "cst_integration")]
pub struct PooledParser<'a> {
    inner: lapce_tree_sitter::parser_pool::PooledParser<'a>,
}

/// Fallback when cst_integration is disabled
#[cfg(not(feature = "cst_integration"))]
pub struct LspParserPool;

#[cfg(not(feature = "cst_integration"))]
impl LspParserPool {
    pub fn new(_max_per_type: usize) -> Self {
        Self
    }
    
    pub fn acquire(&self, _language_id: &str) -> Result<()> {
        Err(anyhow!("CST integration not enabled"))
    }
}

#[cfg(not(feature = "cst_integration"))]
impl Default for LspParserPool {
    fn default() -> Self {
        Self
    }
}

/// Work-stealing task queue for parallel processing
pub struct TaskQueue<T> {
    queue: crossbeam::queue::SegQueue<T>,
}

impl<T> TaskQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: crossbeam::queue::SegQueue::new(),
        }
    }
    
    /// Push task (non-blocking, lock-free)
    pub fn push(&self, task: T) {
        self.queue.push(task);
    }
    
    /// Try to pop task (non-blocking, lock-free)
    pub fn try_pop(&self) -> Option<T> {
        self.queue.pop()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl<T> Default for TaskQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Concurrent symbol index with lock-free reads
pub struct ConcurrentSymbolIndex {
    /// Symbol definitions indexed by symbol name
    definitions: DashMap<String, Vec<SymbolLocation>>,
    /// Symbol references indexed by symbol name
    references: DashMap<String, Vec<SymbolLocation>>,
}

#[derive(Debug, Clone)]
pub struct SymbolLocation {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}

impl ConcurrentSymbolIndex {
    pub fn new() -> Self {
        Self {
            definitions: DashMap::new(),
            references: DashMap::new(),
        }
    }
    
    /// Lock-free read of symbol definitions
    pub fn get_definitions(&self, symbol: &str) -> Vec<SymbolLocation> {
        self.definitions
            .get(symbol)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }
    
    /// Lock-free read of symbol references
    pub fn get_references(&self, symbol: &str) -> Vec<SymbolLocation> {
        self.references
            .get(symbol)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }
    
    /// Add symbol definition
    pub fn add_definition(&self, symbol: String, location: SymbolLocation) {
        self.definitions
            .entry(symbol)
            .or_insert_with(Vec::new)
            .push(location);
    }
    
    /// Add symbol reference
    pub fn add_reference(&self, symbol: String, location: SymbolLocation) {
        self.references
            .entry(symbol)
            .or_insert_with(Vec::new)
            .push(location);
    }
    
    /// Clear index for URI
    pub fn clear_uri(&self, uri: &str) {
        // Remove all entries for this URI
        for mut entry in self.definitions.iter_mut() {
            entry.value_mut().retain(|loc| loc.uri != uri);
        }
        for mut entry in self.references.iter_mut() {
            entry.value_mut().retain(|loc| loc.uri != uri);
        }
    }
    
    /// Get all symbols
    pub fn all_symbols(&self) -> Vec<String> {
        self.definitions.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for ConcurrentSymbolIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_concurrent_document_store() {
        let store = ConcurrentDocumentStore::new();
        
        let data = DocumentData {
            uri: "file:///test.rs".to_string(),
            content: "fn main() {}".to_string(),
            version: 1,
            language_id: "rust".to_string(),
        };
        
        store.insert(data.uri.clone(), data.clone());
        
        let retrieved = store.get("file:///test.rs").unwrap();
        assert_eq!(retrieved.content, "fn main() {}");
        assert_eq!(retrieved.version, 1);
        
        assert!(store.contains("file:///test.rs"));
        assert_eq!(store.len(), 1);
    }
    
    #[test]
    fn test_parse_tree_cache() {
        let cache = ParseTreeCache::new(2);
        
        let cached1 = CachedTree {
            uri: "file:///test1.rs".to_string(),
            version: 1,
            tree_json: "{}".to_string(),
            language_id: "rust".to_string(),
        };
        
        cache.insert(cached1.clone());
        
        let retrieved = cache.get("file:///test1.rs", 1).unwrap();
        assert_eq!(retrieved.uri, "file:///test1.rs");
        
        // Wrong version should return None
        assert!(cache.get("file:///test1.rs", 2).is_none());
    }
    
    #[test]
    fn test_parse_tree_cache_eviction() {
        let cache = ParseTreeCache::new(2);
        
        cache.insert(CachedTree {
            uri: "file:///test1.rs".to_string(),
            version: 1,
            tree_json: "{}".to_string(),
            language_id: "rust".to_string(),
        });
        
        cache.insert(CachedTree {
            uri: "file:///test2.rs".to_string(),
            version: 1,
            tree_json: "{}".to_string(),
            language_id: "rust".to_string(),
        });
        
        // Cache should be full
        assert_eq!(cache.len(), 2);
        
        // Adding third should evict one
        cache.insert(CachedTree {
            uri: "file:///test3.rs".to_string(),
            version: 1,
            tree_json: "{}".to_string(),
            language_id: "rust".to_string(),
        });
        
        assert_eq!(cache.len(), 2);
    }
    
    #[test]
    fn test_task_queue() {
        let queue = TaskQueue::new();
        
        queue.push(1);
        queue.push(2);
        queue.push(3);
        
        assert_eq!(queue.try_pop(), Some(1));
        assert_eq!(queue.try_pop(), Some(2));
        assert_eq!(queue.try_pop(), Some(3));
        assert_eq!(queue.try_pop(), None);
        assert!(queue.is_empty());
    }
    
    #[test]
    fn test_concurrent_symbol_index() {
        let index = ConcurrentSymbolIndex::new();
        
        index.add_definition(
            "main".to_string(),
            SymbolLocation {
                uri: "file:///test.rs".to_string(),
                line: 10,
                character: 5,
            },
        );
        
        index.add_reference(
            "main".to_string(),
            SymbolLocation {
                uri: "file:///other.rs".to_string(),
                line: 20,
                character: 10,
            },
        );
        
        let defs = index.get_definitions("main");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].uri, "file:///test.rs");
        
        let refs = index.get_references("main");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].uri, "file:///other.rs");
    }
    
    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;
        
        let store = Arc::new(ConcurrentDocumentStore::new());
        let mut handles = vec![];
        
        // Spawn multiple threads writing concurrently
        for i in 0..10 {
            let store = store.clone();
            let handle = thread::spawn(move || {
                let data = DocumentData {
                    uri: format!("file:///test{}.rs", i),
                    content: format!("content {}", i),
                    version: i as u32,
                    language_id: "rust".to_string(),
                };
                store.insert(data.uri.clone(), data);
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        assert_eq!(store.len(), 10);
    }
}
