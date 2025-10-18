// Integration tests for semantic_search
use lancedb::types::CodeBlock;
use lancedb::database::cache_manager::CacheManager;
use lancedb::processors::scanner::{DirectoryScanner, list_files, RooIgnoreController, is_path_in_ignored_directory};
use lancedb::processors::parser::CodeParser;
use lancedb::storage::lockfree_cache::LockFreeCache;
use lancedb::storage::hierarchical_cache::HierarchicalCache;
use lancedb::embeddings::service_factory::{ICodeParser, Ignore};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use std::fs;

#[tokio::test]
async fn test_list_files_with_ignore() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();
    
    // Create test structure
    fs::create_dir_all(base.join("src")).unwrap();
    fs::create_dir_all(base.join("node_modules")).unwrap();
    fs::create_dir_all(base.join(".git")).unwrap();
    fs::write(base.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(base.join("node_modules/package.json"), "{}").unwrap();
    fs::write(base.join(".git/config"), "[core]").unwrap();
    fs::write(base.join("README.md"), "# Test").unwrap();
    
    // List files
    let files = list_files(base, true, 100).await.unwrap();
    
    // Filter with RooIgnoreController
    let controller = RooIgnoreController::new(base);
    controller.initialize().await.unwrap();
    let filtered = controller.filter_paths(files);
    
    // Should exclude node_modules and .git
    let paths: Vec<String> = filtered.iter()
        .map(|p| p.strip_prefix(base).unwrap().to_string_lossy().to_string())
        .collect();
    
    assert!(paths.iter().any(|p| p.contains("src/main.rs")));
    assert!(paths.iter().any(|p| p.contains("README.md")));
    assert!(!paths.iter().any(|p| p.contains("node_modules")));
    assert!(!paths.iter().any(|p| p.contains(".git")));
}

#[test]
fn test_cache_manager_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path();
    
    // Create and use cache
    {
        let cache = CacheManager::new(cache_path);
        cache.update_hash("file1.rs", "hash123".to_string());
        cache.update_hash("file2.rs", "hash456".to_string());
        
        assert_eq!(cache.get_hash("file1.rs"), Some("hash123".to_string()));
        assert_eq!(cache.get_hash("file2.rs"), Some("hash456".to_string()));
    }
    
    // Reload cache and verify persistence
    {
        let cache = CacheManager::new(cache_path);
        assert_eq!(cache.get_hash("file1.rs"), Some("hash123".to_string()));
        assert_eq!(cache.get_hash("file2.rs"), Some("hash456".to_string()));
        
        // Delete one
        cache.delete_hash("file1.rs");
        assert_eq!(cache.get_hash("file1.rs"), None);
        assert_eq!(cache.get_hash("file2.rs"), Some("hash456".to_string()));
    }
}

#[test]
fn test_lockfree_cache() {
    use lancedb::storage::lockfree_cache::{LockFreeCache, LockFreeCacheEntry};
    use std::sync::Arc;
    use std::time::Instant;
    
    let cache = LockFreeCache::new(3, 1000);
    
    // Insert items
    for i in 0..5 {
        let entry = LockFreeCacheEntry {
            embedding: Some(Arc::from(vec![i as f32; 10].into_boxed_slice())),
            compressed: None,
            size_bytes: 40,
            access_count: 0,
            last_access: Instant::now(),
        };
        cache.insert(i as u128, entry).unwrap();
    }
    
    // LRU should evict oldest
    assert!(cache.get(&0).is_none()); // Evicted
    assert!(cache.get(&1).is_none()); // Evicted
    assert!(cache.get(&2).is_some());
    assert!(cache.get(&3).is_some());
    assert!(cache.get(&4).is_some());
    
    let stats = cache.stats();
    assert_eq!(stats.entries, 3);
}

#[test]
fn test_code_parser_fallback_chunking() {
    let parser = CodeParser::new();
    let content = "fn main() {\n    println!(\"Hello\");\n}\n\nfn test() {\n    // Test\n}";
    
    let blocks = parser.parse(content);
    assert!(!blocks.is_empty());
    
    for block in &blocks {
        assert!(!block.content.is_empty());
        assert!(block.start_line > 0);
        assert!(block.end_line >= block.start_line);
        assert!(!block.segment_hash.is_empty());
    }
}

#[test]
fn test_path_ignore_filtering() {
    use std::path::PathBuf;
    
    let paths_to_test = vec![
        PathBuf::from("/project/src/main.rs"),
        PathBuf::from("/project/node_modules/lib.js"),
        PathBuf::from("/project/.git/config"),
        PathBuf::from("/project/target/debug/app"),
        PathBuf::from("/project/dist/bundle.js"),
    ];
    
    let results: Vec<bool> = paths_to_test.iter()
        .map(|p| is_path_in_ignored_directory(p))
        .collect();
    
    assert!(!results[0]); // src/main.rs - not ignored
    assert!(results[1]);  // node_modules - ignored
    assert!(results[2]);  // .git - ignored
    assert!(results[3]);  // target - ignored
    assert!(results[4]);  // dist - ignored
}

#[tokio::test]
async fn test_hierarchical_cache() {
    use lancedb::storage::hierarchical_cache::{HierarchicalCache, CacheConfig};
    
    let temp_dir = TempDir::new().unwrap();
    let config = CacheConfig {
        l1_max_size_mb: 0.01,
        l1_max_entries: 2,
        l2_max_size_mb: 0.02,
        l2_max_entries: 4,
        ..Default::default()
    };
    
    let cache = HierarchicalCache::new(config, temp_dir.path()).unwrap();
    
    // Add embeddings
    for i in 0..3 {
        let embedding = vec![i as f32; 128];
        cache.put(&format!("id_{}", i), embedding).unwrap();
    }
    
    // Retrieve and verify
    for i in 0..3 {
        let result = cache.get(&format!("id_{}", i)).unwrap();
        assert!(result.is_some());
    }
    
    let stats = cache.get_stats();
    assert!(stats.l1_entries > 0 || stats.l2_entries > 0);
}
