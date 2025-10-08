// Integration tests for core tools module

#[test]
fn test_core_tools_compile() {
    // Simple test to verify core tools module compiles
    use lapce_ai_rust::core::tools::{
        ToolRegistry,
        ToolContext,
        RooIgnore,
    };
    
    use std::path::PathBuf;
    use tempfile::TempDir;
    
    // Test registry creation
    let registry = ToolRegistry::new();
    assert_eq!(registry.count(), 0);
    
    // Test context creation
    let temp_dir = TempDir::new().unwrap();
    let context = ToolContext::new(
        temp_dir.path().to_path_buf(),
        "test_user".to_string()
    );
    assert_eq!(context.user_id, "test_user");
    
    // Test rooignore
    let rooignore = RooIgnore::new(temp_dir.path().to_path_buf());
    assert!(rooignore.is_allowed(&temp_dir.path().join("test.txt")));
    
    println!("Core tools module compiled and basic functionality works!");
}
