// Tests for filter-aware cache keys (SEM-001)
use crate::search::improved_cache::ImprovedQueryCache;
use crate::search::semantic_search_engine::{SearchFilters, SearchResult};
use std::collections::HashMap;

#[tokio::test]
async fn test_filter_aware_cache_keys_different() {
    let cache = ImprovedQueryCache::new(100, 600);
    
    let query = "find async functions";
    
    // Test that different filters produce different cache keys
    let key1 = cache.compute_cache_key_with_filters(query, None);
    let key2 = cache.compute_cache_key_with_filters(query, Some("language:rust"));
    let key3 = cache.compute_cache_key_with_filters(query, Some("language:python"));
    let key4 = cache.compute_cache_key_with_filters(query, Some("language:rust,path:/src"));
    
    // All keys should be different
    assert_ne!(key1, key2, "Key without filter should differ from key with filter");
    assert_ne!(key2, key3, "Keys with different languages should differ");
    assert_ne!(key2, key4, "Keys with different filter combinations should differ");
    assert_ne!(key3, key4, "Keys with completely different filters should differ");
}

#[tokio::test]
async fn test_filter_aware_cache_keys_consistent() {
    let cache = ImprovedQueryCache::new(100, 600);
    
    let query = "find async functions";
    let filter = "language:rust,min_score:0.8";
    
    // Same query and filter should produce same key
    let key1 = cache.compute_cache_key_with_filters(query, Some(filter));
    let key2 = cache.compute_cache_key_with_filters(query, Some(filter));
    
    assert_eq!(key1, key2, "Same query and filter should produce identical keys");
}

#[tokio::test]
async fn test_cache_isolation_with_filters() {
    let cache = ImprovedQueryCache::new(100, 600);
    
    let query = "async function";
    
    // Create different result sets for different filters
    let result_no_filter = vec![SearchResult {
        path: "/src/main.rs".into(),
        content: "async fn main()".to_string(),
        score: 0.95,
        start_line: 1,
        end_line: 1,
        language: Some("rust".to_string()),
        metadata: HashMap::new(),
    }];
    
    let result_rust_filter = vec![SearchResult {
        path: "/src/lib.rs".into(),
        content: "async fn process()".to_string(),
        score: 0.90,
        start_line: 10,
        end_line: 10,
        language: Some("rust".to_string()),
        metadata: HashMap::new(),
    }];
    
    let result_python_filter = vec![SearchResult {
        path: "/src/main.py".into(),
        content: "async def main():".to_string(),
        score: 0.88,
        start_line: 5,
        end_line: 5,
        language: Some("python".to_string()),
        metadata: HashMap::new(),
    }];
    
    // Insert with different filter contexts
    let key1 = cache.compute_cache_key_with_filters(query, None);
    let key2 = cache.compute_cache_key_with_filters(query, Some("language:rust"));
    let key3 = cache.compute_cache_key_with_filters(query, Some("language:python"));
    
    cache.insert(key1.clone(), result_no_filter.clone()).await;
    cache.insert(key2.clone(), result_rust_filter.clone()).await;
    cache.insert(key3.clone(), result_python_filter.clone()).await;
    
    // Verify each filter gets its own results
    let cached1 = cache.get(&key1).await;
    let cached2 = cache.get(&key2).await;
    let cached3 = cache.get(&key3).await;
    
    assert!(cached1.is_some(), "Should have cached result for no filter");
    assert!(cached2.is_some(), "Should have cached result for rust filter");
    assert!(cached3.is_some(), "Should have cached result for python filter");
    
    // Verify results are different and correct
    assert_eq!(cached1.unwrap()[0].path.to_str().unwrap(), "/src/main.rs");
    assert_eq!(cached2.unwrap()[0].path.to_str().unwrap(), "/src/lib.rs");
    assert_eq!(cached3.unwrap()[0].path.to_str().unwrap(), "/src/main.py");
}

#[tokio::test]
async fn test_search_filters_serialization() {
    // Test that SearchFilters Debug implementation is deterministic
    let filters1 = SearchFilters {
        language: Some("rust".to_string()),
        path_pattern: Some("/src".to_string()),
        min_score: Some(0.8),
        max_results: Some(10),
        file_extensions: Some(vec!["rs".to_string(), "toml".to_string()]),
        exclude_patterns: None,
    };
    
    let filters2 = SearchFilters {
        language: Some("rust".to_string()),
        path_pattern: Some("/src".to_string()),
        min_score: Some(0.8),
        max_results: Some(10),
        file_extensions: Some(vec!["rs".to_string(), "toml".to_string()]),
        exclude_patterns: None,
    };
    
    let str1 = format!("{:?}", filters1);
    let str2 = format!("{:?}", filters2);
    
    assert_eq!(str1, str2, "Same filters should serialize identically");
}

#[tokio::test]
async fn test_cache_metrics_with_filter_aware_keys() {
    let cache = ImprovedQueryCache::new(100, 600);
    
    let query = "test query";
    let result = vec![SearchResult {
        path: "/test.rs".into(),
        content: "test".to_string(),
        score: 1.0,
        start_line: 1,
        end_line: 1,
        language: Some("rust".to_string()),
        metadata: HashMap::new(),
    }];
    
    // Insert with filter
    let key = cache.compute_cache_key_with_filters(query, Some("language:rust"));
    cache.insert(key.clone(), result.clone()).await;
    
    // First get should be a hit
    let _ = cache.get(&key).await;
    
    // Different filter should be a miss
    let key2 = cache.compute_cache_key_with_filters(query, Some("language:python"));
    let _ = cache.get(&key2).await;
    
    let stats = cache.get_stats().await;
    assert_eq!(stats.hits, 1, "Should have 1 cache hit");
    assert_eq!(stats.misses, 1, "Should have 1 cache miss");
    assert_eq!(stats.entries, 1, "Should have 1 cached entry");
}
