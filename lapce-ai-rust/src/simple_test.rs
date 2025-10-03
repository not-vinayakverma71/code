// Simple test to verify semantic search is working

#[cfg(test)]
mod tests {
    use super::super::semantic_search::*;
    use tempfile::tempdir;
    use std::time::Instant;
    
    #[tokio::test]
    async fn test_basic_search() -> anyhow::Result<()> {
        let engine = SemanticSearchEngine::new().await?;
        
        // Create test files
        let dir = tempdir()?;
        for i in 0..10 {
            let path = dir.path().join(format!("test{}.rs", i));
            std::fs::write(&path, format!("fn function_{}() {{ }}", i))?;
        }
        
        // Index
        let stats = engine.index_directory(dir.path()).await?;
        println!("Indexed {} files", stats.files_indexed);
        
        // Search
        let results = engine.search("function", 5).await?;
        println!("Found {} results", results.len());
        
        assert!(stats.files_indexed > 0);
        assert!(!results.is_empty());
        
        Ok(())
    }
}
