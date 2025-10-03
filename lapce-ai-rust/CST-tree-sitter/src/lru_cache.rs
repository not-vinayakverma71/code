//! LRU Cache for Parse Trees
//! Keeps only recently used files in memory

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use tree_sitter::Tree;

#[derive(Debug, Clone)]
pub struct CachedParseTree {
    pub tree: Tree,
    pub source: Vec<u8>,
    pub node_count: usize,
    pub access_count: usize,
    pub last_access: std::time::Instant,
}

pub struct LRUParseCache {
    cache: Arc<RwLock<HashMap<PathBuf, CachedParseTree>>>,
    max_entries: usize,
    max_memory_bytes: usize,
    current_memory: Arc<RwLock<usize>>,
}

impl LRUParseCache {
    pub fn new(max_entries: usize, max_memory_mb: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            max_memory_bytes: max_memory_mb * 1024 * 1024,
            current_memory: Arc::new(RwLock::new(0)),
        }
    }

    /// Get parse tree from cache
    pub fn get(&self, path: &PathBuf) -> Option<CachedParseTree> {
        let mut cache = self.cache.write();
        
        if let Some(entry) = cache.get_mut(path) {
            // Update access info
            entry.access_count += 1;
            entry.last_access = std::time::Instant::now();
            Some(entry.clone())
        } else {
            None
        }
    }

    /// Insert parse tree into cache
    pub fn insert(&self, path: PathBuf, tree: Tree, source: Vec<u8>) {
        let node_count = count_nodes(tree.root_node());
        let entry_size = source.len() + (node_count * 50); // 50 bytes per node
        
        // Check if we need to evict
        self.maybe_evict(entry_size);
        
        let cached = CachedParseTree {
            tree,
            source,
            node_count,
            access_count: 1,
            last_access: std::time::Instant::now(),
        };
        
        let mut cache = self.cache.write();
        cache.insert(path, cached);
        
        *self.current_memory.write() += entry_size;
    }

    /// Evict entries if needed
    fn maybe_evict(&self, new_entry_size: usize) {
        let mut cache = self.cache.write();
        let mut current_mem = self.current_memory.write();
        
        // Evict by count
        while cache.len() >= self.max_entries {
            if let Some(lru_path) = self.find_lru(&cache) {
                if let Some(removed) = cache.remove(&lru_path) {
                    let size = removed.source.len() + (removed.node_count * 50);
                    *current_mem = current_mem.saturating_sub(size);
                }
            } else {
                break;
            }
        }
        
        // Evict by memory
        while *current_mem + new_entry_size > self.max_memory_bytes {
            if let Some(lru_path) = self.find_lru(&cache) {
                if let Some(removed) = cache.remove(&lru_path) {
                    let size = removed.source.len() + (removed.node_count * 50);
                    *current_mem = current_mem.saturating_sub(size);
                }
            } else {
                break;
            }
        }
    }

    /// Find least recently used entry
    fn find_lru(&self, cache: &HashMap<PathBuf, CachedParseTree>) -> Option<PathBuf> {
        cache.iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(path, _)| path.clone())
    }

    /// Clear cache
    pub fn clear(&self) {
        self.cache.write().clear();
        *self.current_memory.write() = 0;
    }

    /// Remove specific entry
    pub fn remove(&self, path: &PathBuf) -> bool {
        let mut cache = self.cache.write();
        if let Some(removed) = cache.remove(path) {
            let size = removed.source.len() + (removed.node_count * 50);
            *self.current_memory.write() = self.current_memory.read().saturating_sub(size);
            true
        } else {
            false
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let current_mem = *self.current_memory.read();
        
        CacheStats {
            entries: cache.len(),
            memory_bytes: current_mem,
            memory_mb: current_mem as f64 / (1024.0 * 1024.0),
            max_entries: self.max_entries,
            max_memory_mb: self.max_memory_bytes / (1024 * 1024),
            utilization: (current_mem as f64 / self.max_memory_bytes as f64) * 100.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub memory_bytes: usize,
    pub memory_mb: f64,
    pub max_entries: usize,
    pub max_memory_mb: usize,
    pub utilization: f64,
}

fn count_nodes(node: tree_sitter::Node) -> usize {
    let mut count = 1;
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            count += count_nodes(cursor.node());
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    #[test]
    fn test_lru_eviction() {
        let cache = LRUParseCache::new(2, 1); // 2 entries max, 1 MB max
        
        let mut parser = Parser::new();
        parser.set_language(unsafe { tree_sitter_rust::LANGUAGE }.into()).unwrap();
        
        // Add 3 files
        let code1 = "fn a() {}";
        let code2 = "fn b() {}";
        let code3 = "fn c() {}";
        
        let tree1 = parser.parse(code1, None).unwrap();
        let tree2 = parser.parse(code2, None).unwrap();
        let tree3 = parser.parse(code3, None).unwrap();
        
        cache.insert(PathBuf::from("a.rs"), tree1, code1.as_bytes().to_vec());
        cache.insert(PathBuf::from("b.rs"), tree2, code2.as_bytes().to_vec());
        
        let stats = cache.stats();
        assert_eq!(stats.entries, 2);
        
        // This should evict oldest (a.rs)
        cache.insert(PathBuf::from("c.rs"), tree3, code3.as_bytes().to_vec());
        
        let stats = cache.stats();
        assert_eq!(stats.entries, 2);
        assert!(cache.get(&PathBuf::from("a.rs")).is_none());
        assert!(cache.get(&PathBuf::from("b.rs")).is_some());
        assert!(cache.get(&PathBuf::from("c.rs")).is_some());
    }
}
