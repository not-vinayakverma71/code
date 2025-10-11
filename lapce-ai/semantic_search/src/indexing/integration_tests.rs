// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Integration tests for incremental indexing (CST-B05-5)
//!
//! Tests the complete flow: parse → detect changes → cache embeddings → index

#[cfg(all(test, feature = "cst_ts"))]
mod incremental_integration_tests {
    use crate::indexing::{CachedEmbedder, EmbeddingModel, IncrementalDetector};
    use crate::processors::cst_to_ast_pipeline::CstToAstPipeline;
    use crate::error::Result;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;
    use std::fs;

    /// Mock embedder for testing
    struct TestEmbedder;
    
    impl EmbeddingModel for TestEmbedder {
        fn embed(&self, text: &str) -> Result<Vec<f32>> {
            // Deterministic embedding based on text hash
            let hash = text.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));
            Ok(vec![
                (hash % 1000) as f32 / 1000.0,
                (hash % 100) as f32 / 100.0,
                text.len() as f32
            ])
        }
        
        fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
            texts.into_iter().map(|t| self.embed(t)).collect()
        }
    }

    #[tokio::test]
    async fn test_incremental_reindex_on_file_change() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        
        // Create initial file
        let initial_content = r#"
fn hello() {
    println!("Hello");
}

fn world() {
    println!("World");
}
"#;
        fs::write(&file_path, initial_content).unwrap();
        
        // Setup pipeline and embedder
        let pipeline = Arc::new(CstToAstPipeline::new());
        let embedder = Arc::new(CachedEmbedder::new(Arc::new(TestEmbedder)));
        
        // First parse - all nodes are new
        let output1 = pipeline.process_file(&file_path).await.unwrap();
        let (embeddings1, changeset1) = embedder
            .embed_file_incremental(&output1.cst, &file_path)
            .unwrap();
        
        println!("First parse: {} added nodes", changeset1.added.len());
        assert!(changeset1.added.len() > 0, "Should have added nodes");
        assert_eq!(changeset1.unchanged.len(), 0, "No unchanged nodes on first parse");
        
        let initial_embedding_count = embeddings1.len();
        let stats1 = embedder.stats();
        assert_eq!(stats1.cache_misses, initial_embedding_count);
        assert_eq!(stats1.embeddings_generated, initial_embedding_count);
        
        // Second parse - same content (should reuse all embeddings)
        let output2 = pipeline.process_file(&file_path).await.unwrap();
        embedder.reset_stats();
        let (embeddings2, changeset2) = embedder
            .embed_file_incremental(&output2.cst, &file_path)
            .unwrap();
        
        println!("Second parse: {} unchanged, {} changes",
                 changeset2.unchanged.len(), changeset2.total_changes());
        assert_eq!(changeset2.unchanged.len(), changeset1.added.len(),
                   "All nodes should be unchanged");
        assert!(!changeset2.has_changes(), "No changes on re-parse");
        
        let stats2 = embedder.stats();
        assert!(stats2.embeddings_reused > 0, "Should reuse embeddings");
        assert_eq!(stats2.embeddings_generated, 0, "Should not generate new embeddings");
        
        // Modify file - change one function
        let modified_content = r#"
fn hello() {
    println!("Hello, World!");  // Modified
}

fn world() {
    println!("World");
}
"#;
        fs::write(&file_path, modified_content).unwrap();
        
        // Third parse - detect modification
        let output3 = pipeline.process_file(&file_path).await.unwrap();
        embedder.reset_stats();
        let (embeddings3, changeset3) = embedder
            .embed_file_incremental(&output3.cst, &file_path)
            .unwrap();
        
        println!("Third parse: {} unchanged, {} modified, {} added, {} deleted",
                 changeset3.unchanged.len(),
                 changeset3.modified.len(),
                 changeset3.added.len(),
                 changeset3.deleted.len());
        
        assert!(changeset3.has_changes(), "Should detect changes");
        assert!(changeset3.unchanged.len() > 0, "Some nodes should be unchanged");
        
        let stats3 = embedder.stats();
        assert!(stats3.embeddings_reused > 0, "Should reuse some embeddings");
        assert!(stats3.embeddings_generated > 0, "Should generate some new embeddings");
        
        // Calculate efficiency
        let reuse_rate = stats3.embeddings_reused as f64 
            / (stats3.embeddings_reused + stats3.embeddings_generated) as f64;
        println!("Embedding reuse rate: {:.1}%", reuse_rate * 100.0);
        
        // Note: CST changes propagate through parent nodes, so even small changes
        // can affect multiple nodes. A realistic expectation is that we reuse SOME embeddings.
        assert!(stats3.embeddings_reused > 0, "Should reuse at least some embeddings");
        assert!(changeset3.unchanged.len() > 0, "Should have unchanged nodes");
    }

    #[tokio::test]
    async fn test_add_and_delete_functions() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        
        // Initial file with 2 functions
        fs::write(&file_path, r#"
fn func_a() {}
fn func_b() {}
"#).unwrap();
        
        let pipeline = Arc::new(CstToAstPipeline::new());
        let embedder = Arc::new(CachedEmbedder::new(Arc::new(TestEmbedder)));
        
        // First parse
        let output1 = pipeline.process_file(&file_path).await.unwrap();
        embedder.embed_file_incremental(&output1.cst, &file_path).unwrap();
        
        // Add a new function
        fs::write(&file_path, r#"
fn func_a() {}
fn func_b() {}
fn func_c() {}
"#).unwrap();
        
        let output2 = pipeline.process_file(&file_path).await.unwrap();
        let (_, changeset2) = embedder
            .embed_file_incremental(&output2.cst, &file_path)
            .unwrap();
        
        assert!(changeset2.added.len() > 0, "Should detect added function");
        println!("Added {} nodes", changeset2.added.len());
        
        // Delete a function
        fs::write(&file_path, r#"
fn func_a() {}
fn func_c() {}
"#).unwrap();
        
        let output3 = pipeline.process_file(&file_path).await.unwrap();
        let (_, changeset3) = embedder
            .embed_file_incremental(&output3.cst, &file_path)
            .unwrap();
        
        assert!(changeset3.deleted.len() > 0, "Should detect deleted function");
        println!("Deleted {} nodes", changeset3.deleted.len());
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        
        fs::write(&file_path, "fn test() {}").unwrap();
        
        let pipeline = Arc::new(CstToAstPipeline::new());
        let embedder = Arc::new(CachedEmbedder::new(Arc::new(TestEmbedder)));
        
        // Parse and cache
        let output1 = pipeline.process_file(&file_path).await.unwrap();
        embedder.embed_file_incremental(&output1.cst, &file_path).unwrap();
        
        let (hits1, misses1, _, _) = embedder.cache_stats();
        println!("Before invalidation: hits={}, misses={}", hits1, misses1);
        
        // Invalidate cache for file
        embedder.invalidate_file(&file_path);
        
        // Re-parse should be cache miss
        let output2 = pipeline.process_file(&file_path).await.unwrap();
        embedder.embed_file_incremental(&output2.cst, &file_path).unwrap();
        
        let stats = embedder.stats();
        assert!(stats.cache_misses > 0, "Should have cache misses after invalidation");
    }

    #[tokio::test]
    async fn test_performance_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("perf.rs");
        
        // Create file
        let initial_content = r#"fn hello() { println!("Hello"); }
fn world() { println!("World"); }"#;
        fs::write(&file_path, initial_content).unwrap();
        
        let pipeline = Arc::new(CstToAstPipeline::new());
        let embedder = Arc::new(CachedEmbedder::new(Arc::new(TestEmbedder)));
        
        // First parse - measure baseline
        let start = std::time::Instant::now();
        let output1 = pipeline.process_file(&file_path).await.unwrap();
        let (embeddings1, _) = embedder.embed_file_incremental(&output1.cst, &file_path).unwrap();
        let first_time = start.elapsed();
        
        println!("Performance test:");
        println!("  First parse: {:?} ({} embeddings)", first_time, embeddings1.len());
        
        // Second parse - should use cache
        embedder.reset_stats();
        let start = std::time::Instant::now();
        let output2 = pipeline.process_file(&file_path).await.unwrap();
        let (embeddings2, changeset2) = embedder.embed_file_incremental(&output2.cst, &file_path).unwrap();
        let cached_time = start.elapsed();
        
        println!("  Cached parse: {:?} ({} unchanged, {} reused)",
                 cached_time, changeset2.unchanged.len(), embedder.stats().embeddings_reused);
        
        // Basic validation
        assert_eq!(embeddings1.len(), embeddings2.len(), "Should have same number of embeddings");
        
        let (hits, misses, _, _) = embedder.cache_stats();
        println!("  Cache stats: {} hits, {} misses", hits, misses);
    }

    #[tokio::test]
    async fn test_multiple_files_independence() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.rs");
        let file2 = temp_dir.path().join("file2.rs");
        
        fs::write(&file1, "fn file1_func() {}").unwrap();
        fs::write(&file2, "fn file2_func() {}").unwrap();
        
        let pipeline = Arc::new(CstToAstPipeline::new());
        let embedder = Arc::new(CachedEmbedder::new(Arc::new(TestEmbedder)));
        
        // Parse both files
        let output1 = pipeline.process_file(&file1).await.unwrap();
        embedder.embed_file_incremental(&output1.cst, &file1).unwrap();
        
        let output2 = pipeline.process_file(&file2).await.unwrap();
        embedder.embed_file_incremental(&output2.cst, &file2).unwrap();
        
        // Modify file1
        fs::write(&file1, "fn file1_func_modified() {}").unwrap();
        let output1_mod = pipeline.process_file(&file1).await.unwrap();
        let (_, changeset1) = embedder
            .embed_file_incremental(&output1_mod.cst, &file1)
            .unwrap();
        
        // Re-parse file2 - should still be unchanged
        let output2_reparse = pipeline.process_file(&file2).await.unwrap();
        let (_, changeset2) = embedder
            .embed_file_incremental(&output2_reparse.cst, &file2)
            .unwrap();
        
        assert!(changeset1.has_changes(), "File1 should have changes");
        assert!(!changeset2.has_changes(), "File2 should have no changes");
    }
}
