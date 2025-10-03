//! Basic test to verify tree-sitter integration is working

use lapce_tree_sitter::NativeParserManager;
use std::sync::Arc;

#[test]
fn test_parser_manager_creation() {
    let parser_manager = NativeParserManager::new();
    assert!(parser_manager.is_ok(), "Parser manager should be created successfully");
}

#[test] 
fn test_supported_languages() {
    use lapce_tree_sitter::FileType;
    
    // Test that we can iterate over supported file types
    let mut count = 0;
    for _file_type in FileType::iter() {
        count += 1;
    }
    assert!(count > 30, "Should support 30+ languages");
}

#[test]
fn test_language_detection() {
    use lapce_tree_sitter::FileType;
    use std::path::Path;
    
    // Test file extension detection
    assert_eq!(FileType::from_extension("rs"), Some(FileType::Rust));
    assert_eq!(FileType::from_extension("js"), Some(FileType::JavaScript));
    assert_eq!(FileType::from_extension("py"), Some(FileType::Python));
    assert_eq!(FileType::from_extension("go"), Some(FileType::Go));
    assert_eq!(FileType::from_extension("ts"), Some(FileType::TypeScript));
}
