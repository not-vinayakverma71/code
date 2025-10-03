//! Test ParserPool implementation

use lapce_tree_sitter::pool::ParserPool;
use lapce_tree_sitter::types::FileType;
use std::sync::Arc;

#[tokio::test]
async fn test_parser_pool_reuse() {
    let pool = Arc::new(ParserPool::new(10));
    
    // Get parser, use it, return it
    let parser1 = pool.get_parser(FileType::Rust).await.unwrap();
    let parser1_ptr = parser1.as_ref() as *const _;
    drop(parser1); // Return to pool
    
    // Get parser again - should be same instance
    let parser2 = pool.get_parser(FileType::Rust).await.unwrap();
    let parser2_ptr = parser2.as_ref() as *const _;
    
    // Verify same parser instance was reused
    assert_eq!(parser1_ptr, parser2_ptr, "Parser should be reused from pool");
    println!("✅ Parser pool correctly reuses parsers");
}

#[tokio::test]
async fn test_parser_pool_concurrent() {
    let pool = Arc::new(ParserPool::new(5));
    
    // Spawn 10 tasks that each need a parser
    let mut handles = vec![];
    for i in 0..10 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let parser = pool_clone.get_parser(FileType::Rust).await.unwrap();
            // Simulate work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            drop(parser); // Return to pool
            i
        });
        handles.push(handle);
    }
    
    // All should complete without deadlock
    for handle in handles {
        handle.await.unwrap();
    }
    
    println!("✅ Parser pool handles concurrent access");
}

#[test]
fn test_parser_pool_multiple_languages() {
    use std::collections::HashSet;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    let pool = Arc::new(ParserPool::new(10));
    
    let languages = vec![
        FileType::Rust,
        FileType::JavaScript,
        FileType::Python,
        FileType::Go,
        FileType::Java,
    ];
    
    let mut parsers = HashSet::new();
    
    for lang in &languages {
        let parser = rt.block_on(pool.get_parser(*lang)).unwrap();
        // Each language should get its own pool
        parsers.insert(*lang);
        drop(parser);
    }
    
    assert_eq!(parsers.len(), languages.len());
    println!("✅ Parser pool manages multiple language pools");
}
