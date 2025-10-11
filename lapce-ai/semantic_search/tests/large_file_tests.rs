// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Large file tests with memory and time benchmarks (CST-B07)

#[cfg(feature = "cst_ts")]
mod large_file_tests {
    use lancedb::indexing::{
        StableIdEmbeddingCache, IncrementalDetector, CachedEmbedder,
        EmbeddingModel,
    };
    use lancedb::ast::{CstNode, NodeMetadata};
    use std::sync::Arc;
    use std::time::Instant;
    
    // Mock embedding model
    struct MockEmbeddingModel;
    
    impl EmbeddingModel for MockEmbeddingModel {
        fn embed(&self, _text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
            Ok(vec![0.1; 384])
        }
        
        fn dimension(&self) -> usize {
            384
        }
    }
    
    fn generate_large_cst(node_count: usize) -> Vec<CstNode> {
        (0..node_count).map(|i| {
            let kind = match i % 5 {
                0 => "function_definition",
                1 => "class_definition",
                2 => "expression_statement",
                3 => "assignment",
                _ => "identifier",
            };
            
            CstNode {
                kind: kind.to_string(),
                text: format!("{}_{}", kind, i),
                start_byte: i * 50,
                end_byte: (i + 1) * 50,
                start_point: (i / 80, i % 80),
                end_point: (i / 80, (i % 80) + 50),
                children: vec![],
                metadata: NodeMetadata {
                    semantic_info: None,
                    canonical_kind: Some(kind.to_string()),
                    stable_id: Some(i as u64),
                },
            }
        }).collect()
    }
    
    #[test]
    fn test_1k_nodes_performance() {
        let nodes = generate_large_cst(1000);
        
        let start = Instant::now();
        let cache = StableIdEmbeddingCache::new();
        
        for node in &nodes {
            if let Some(stable_id) = node.metadata.stable_id {
                cache.insert(
                    stable_id,
                    &node.text,
                    &vec![0.1; 384],
                    &node.kind,
                    "/test.rs",
                );
            }
        }
        
        let elapsed = start.elapsed();
        println!("1k nodes insertion: {:?}", elapsed);
        assert!(elapsed.as_millis() < 100, "Should insert 1k nodes in <100ms");
        
        let (hits, misses, size, entries) = cache.stats();
        assert_eq!(entries, 1000);
        println!("Cache size: {} bytes, {} entries", size, entries);
    }
    
    #[test]
    fn test_10k_nodes_performance() {
        let nodes = generate_large_cst(10000);
        
        let start = Instant::now();
        let cache = StableIdEmbeddingCache::new();
        
        for node in &nodes {
            if let Some(stable_id) = node.metadata.stable_id {
                cache.insert(
                    stable_id,
                    &node.text,
                    &vec![0.1; 384],
                    &node.kind,
                    "/test.rs",
                );
            }
        }
        
        let elapsed = start.elapsed();
        println!("10k nodes insertion: {:?}", elapsed);
        assert!(elapsed.as_millis() < 1000, "Should insert 10k nodes in <1s");
        
        let (_, _, size, entries) = cache.stats();
        println!("10k nodes - Cache size: {} MB, {} entries", size / 1_048_576, entries);
    }
    
    #[test]
    fn test_change_detection_1k_nodes() {
        use std::collections::HashMap;
        
        let old_nodes = generate_large_cst(1000);
        let mut new_nodes = old_nodes.clone();
        
        // Modify 10% (100 nodes)
        for i in 0..100 {
            new_nodes[i].text = format!("modified_{}", i);
        }
        
        let old_ids: HashMap<u64, &CstNode> = old_nodes.iter()
            .filter_map(|n| n.metadata.stable_id.map(|id| (id, n)))
            .collect();
        
        let start = Instant::now();
        let detector = IncrementalDetector::new(old_ids);
        let changeset = detector.detect_changes(&new_nodes);
        let elapsed = start.elapsed();
        
        println!("Change detection (1k nodes, 10% modified): {:?}", elapsed);
        assert!(elapsed.as_micros() < 1000, "Should detect changes in 1k nodes in <1ms");
        
        assert_eq!(changeset.unchanged.len(), 900);
        assert_eq!(changeset.modified.len(), 100);
        assert_eq!(changeset.added.len(), 0);
        assert_eq!(changeset.deleted.len(), 0);
    }
    
    #[test]
    fn test_change_detection_10k_nodes() {
        use std::collections::HashMap;
        
        let old_nodes = generate_large_cst(10000);
        let mut new_nodes = old_nodes.clone();
        
        // Modify 5% (500 nodes)
        for i in 0..500 {
            new_nodes[i].text = format!("modified_{}", i);
        }
        
        let old_ids: HashMap<u64, &CstNode> = old_nodes.iter()
            .filter_map(|n| n.metadata.stable_id.map(|id| (id, n)))
            .collect();
        
        let start = Instant::now();
        let detector = IncrementalDetector::new(old_ids);
        let changeset = detector.detect_changes(&new_nodes);
        let elapsed = start.elapsed();
        
        println!("Change detection (10k nodes, 5% modified): {:?}", elapsed);
        assert!(elapsed.as_millis() < 10, "Should detect changes in 10k nodes in <10ms");
        
        assert_eq!(changeset.unchanged.len(), 9500);
        assert_eq!(changeset.modified.len(), 500);
    }
    
    #[test]
    fn test_cached_embedding_throughput() {
        let nodes = generate_large_cst(1000);
        let model = Arc::new(MockEmbeddingModel);
        let embedder = Arc::new(CachedEmbedder::new(model));
        
        // First pass - all misses
        let start = Instant::now();
        for node in &nodes {
            let _ = embedder.embed_node(node);
        }
        let elapsed_cold = start.elapsed();
        
        println!("Cold embedding (1k nodes): {:?}", elapsed_cold);
        
        // Second pass - all hits
        let start = Instant::now();
        for node in &nodes {
            let _ = embedder.embed_node(node);
        }
        let elapsed_hot = start.elapsed();
        
        println!("Hot embedding (1k nodes): {:?}", elapsed_hot);
        
        let speedup = elapsed_cold.as_micros() as f64 / elapsed_hot.as_micros() as f64;
        println!("Speedup: {:.2}x", speedup);
        
        assert!(speedup > 10.0, "Cache should provide >10x speedup");
        
        let stats = embedder.stats();
        assert_eq!(stats.embeddings_generated, 1000);
        assert_eq!(stats.embeddings_reused, 1000);
    }
    
    #[test]
    fn test_memory_usage_scaling() {
        // Test memory scaling with different cache sizes
        let sizes = [100, 500, 1000, 5000];
        
        for size in sizes {
            let cache = StableIdEmbeddingCache::new();
            let embedding = vec![0.1; 384];
            
            for i in 0..size {
                cache.insert(
                    i as u64,
                    &format!("node_{}", i),
                    &embedding,
                    "function",
                    "/test.rs",
                );
            }
            
            let (_, _, bytes, entries) = cache.stats();
            let bytes_per_entry = bytes / entries as u64;
            
            println!("{} entries: {} bytes total, {} bytes/entry", 
                     entries, bytes, bytes_per_entry);
            
            // Expect ~100 bytes per entry (embedding + metadata)
            assert!(bytes_per_entry > 50 && bytes_per_entry < 200,
                    "Memory per entry should be reasonable");
        }
    }
    
    #[test]
    fn test_incremental_speedup_realistic() {
        use std::collections::HashMap;
        
        // Simulate editing a large file
        let initial_nodes = generate_large_cst(5000);
        let model = Arc::new(MockEmbeddingModel);
        
        // Initial index (full embedding)
        let embedder = Arc::new(CachedEmbedder::new(model.clone()));
        let start = Instant::now();
        for node in &initial_nodes {
            let _ = embedder.embed_node(node);
        }
        let full_time = start.elapsed();
        
        println!("Full indexing (5k nodes): {:?}", full_time);
        
        // Simulate edit: modify 50 nodes
        let mut edited_nodes = initial_nodes.clone();
        for i in 0..50 {
            edited_nodes[i].text = format!("edited_{}", i);
            // Clear stable_id to simulate new content
            edited_nodes[i].metadata.stable_id = Some((i + 10000) as u64);
        }
        
        // Incremental re-index
        let old_ids: HashMap<u64, &CstNode> = initial_nodes.iter()
            .filter_map(|n| n.metadata.stable_id.map(|id| (id, n)))
            .collect();
        
        let start = Instant::now();
        let detector = IncrementalDetector::new(old_ids);
        let changeset = detector.detect_changes(&edited_nodes);
        
        // Embed only changed/new nodes
        for node in changeset.modified.iter().chain(changeset.added.iter()) {
            let _ = embedder.embed_node(node);
        }
        let incremental_time = start.elapsed();
        
        println!("Incremental re-index (50 changes): {:?}", incremental_time);
        
        let speedup = full_time.as_micros() as f64 / incremental_time.as_micros() as f64;
        println!("Incremental speedup: {:.2}x", speedup);
        
        assert!(speedup > 5.0, "Incremental should be >5x faster for small edits");
    }
    
    #[test]
    fn test_cache_hit_rate_tracking() {
        let nodes = generate_large_cst(1000);
        let model = Arc::new(MockEmbeddingModel);
        let embedder = Arc::new(CachedEmbedder::new(model));
        
        // Populate cache with 800 nodes
        for node in &nodes[..800] {
            let _ = embedder.embed_node(node);
        }
        
        // Process all 1000 (80% hit rate expected)
        for node in &nodes {
            let _ = embedder.embed_node(node);
        }
        
        let stats = embedder.stats();
        let hit_rate = stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64;
        
        println!("Hit rate: {:.2}%", hit_rate * 100.0);
        assert!(hit_rate > 0.75, "Should achieve >75% hit rate");
    }
    
    #[test]
    #[ignore] // Run with --ignored for stress testing
    fn test_stress_50k_nodes() {
        let nodes = generate_large_cst(50000);
        
        let start = Instant::now();
        let cache = StableIdEmbeddingCache::new();
        
        for node in &nodes {
            if let Some(stable_id) = node.metadata.stable_id {
                cache.insert(
                    stable_id,
                    &node.text,
                    &vec![0.1; 384],
                    &node.kind,
                    "/large.rs",
                );
            }
        }
        
        let elapsed = start.elapsed();
        let (_, _, size, entries) = cache.stats();
        
        println!("Stress test (50k nodes):");
        println!("  Time: {:?}", elapsed);
        println!("  Memory: {} MB", size / 1_048_576);
        println!("  Entries: {}", entries);
        println!("  Throughput: {} nodes/sec", 
                 50000 * 1000 / elapsed.as_millis().max(1));
    }
}
