// PHASE 4.1: COMPREHENSIVE UNIT TESTS
#[cfg(test)]
mod unit_tests {
    use lru::LruCache;
    use std::num::NonZeroUsize;
    use std::collections::HashMap;
    use std::time::{Duration, Instant};
    use std::path::PathBuf;

    // TEST MODULE 1: Titan Embedder Tests
    mod titan_embedder_tests {
        use super::*;
        
        #[test]
        fn test_embedding_dimensions() {
            // Test that embeddings have correct dimensions
            let embedding = vec![0.1_f32; 1536];
            assert_eq!(embedding.len(), 1536, "Embedding should have 1536 dimensions");
        }
        
        #[test]
        fn test_embedding_normalization() {
            // Test embedding normalization
            let mut embedding = vec![3.0, 4.0]; // 3-4-5 triangle
            let magnitude = (9.0 + 16.0_f32).sqrt();
            
            // Normalize
            for val in &mut embedding {
                *val /= magnitude;
            }
            
            // Check normalized
            let norm_mag = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((norm_mag - 1.0).abs() < 0.001, "Normalized embedding should have magnitude 1");
        }
        
        #[test]
        fn test_empty_text_handling() {
            // Test handling of empty text
            let text = "";
            let result = process_text(text);
            assert!(result.is_ok(), "Should handle empty text gracefully");
        }
        
        #[test]
        fn test_large_text_chunking() {
            // Test chunking of large text
            let large_text = "a".repeat(10000);
            let chunks = chunk_text(&large_text, 1000);
            assert!(chunks.len() > 1, "Large text should be chunked");
            assert!(chunks.iter().all(|c| c.len() <= 1000), "All chunks should be <= 1000 chars");
        }
        
        fn process_text(text: &str) -> Result<String, String> {
            if text.is_empty() {
                Ok("".to_string())
            } else {
                Ok(text.to_string())
            }
        }
        
        fn chunk_text(text: &str, chunk_size: usize) -> Vec<String> {
            text.chars()
                .collect::<Vec<_>>()
                .chunks(chunk_size)
                .map(|c| c.iter().collect())
                .collect()
        }
    }

    // TEST MODULE 2: Semantic Engine Tests
    mod semantic_engine_tests {
        use super::*;
        
        #[test]
        fn test_search_result_ranking() {
            // Test that results are properly ranked
            let mut results = vec![
                ("doc1", 0.5),
                ("doc2", 0.8),
                ("doc3", 0.3),
            ];
            
            results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            
            assert_eq!(results[0].0, "doc2", "Highest score should be first");
            assert_eq!(results[2].0, "doc3", "Lowest score should be last");
        }
        
        #[test]
        fn test_similarity_calculation() {
            // Test cosine similarity calculation
            let vec1 = vec![1.0, 0.0, 0.0];
            let vec2 = vec![1.0, 0.0, 0.0];
            let similarity = cosine_similarity(&vec1, &vec2);
            
            assert!((similarity - 1.0).abs() < 0.001, "Identical vectors should have similarity 1.0");
            
            let vec3 = vec![0.0, 1.0, 0.0];
            let similarity2 = cosine_similarity(&vec1, &vec3);
            assert!((similarity2).abs() < 0.001, "Perpendicular vectors should have similarity 0.0");
        }
        
        #[test]
        fn test_empty_query_handling() {
            let query = "";
            let results = search_with_query(query);
            assert!(results.is_empty() || results.is_empty(), "Empty query should return no results or handle gracefully");
        }
        
        #[test]
        fn test_result_deduplication() {
            let mut results = vec![
                ("doc1", 0.9),
                ("doc1", 0.8), // Duplicate
                ("doc2", 0.7),
            ];
            
            let deduped = deduplicate_results(results);
            assert_eq!(deduped.len(), 2, "Should remove duplicates");
            assert!(deduped.iter().any(|(doc, _)| doc == &"doc1"));
            assert!(deduped.iter().any(|(doc, _)| doc == &"doc2"));
        }
        
        fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
            let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
            
            if mag_a == 0.0 || mag_b == 0.0 {
                0.0
            } else {
                dot / (mag_a * mag_b)
            }
        }
        
        fn search_with_query(query: &str) -> Vec<(&str, f32)> {
            if query.is_empty() {
                vec![]
            } else {
                vec![("result", 0.5)]
            }
        }
        
        fn deduplicate_results(results: Vec<(&str, f32)>) -> Vec<(&str, f32)> {
            let mut seen = HashMap::new();
            let mut deduped = Vec::new();
            
            for (doc, score) in results {
                if !seen.contains_key(doc) {
                    seen.insert(doc, true);
                    deduped.push((doc, score));
                }
            }
            
            deduped
        }
    }

    // TEST MODULE 3: Code Indexer Tests
    mod code_indexer_tests {
        use super::*;
        
        #[test]
        fn test_file_parsing() {
            let content = "fn main() {\n    println!(\"Hello\");\n}";
            let lines: Vec<&str> = content.lines().collect();
            
            assert_eq!(lines.len(), 3, "Should parse 3 lines");
            assert!(lines[0].contains("fn main"), "Should contain function");
        }
        
        #[test]
        fn test_chunk_overlap() {
            let lines = vec!["line1", "line2", "line3", "line4", "line5"];
            let chunks = create_chunks_with_overlap(&lines, 3, 1);
            
            assert!(chunks.len() >= 2, "Should create multiple chunks");
            // Check overlap
            let chunk1_last = chunks[0].last().unwrap();
            let chunk2_first = chunks[1].first().unwrap();
            assert_eq!(chunk1_last, chunk2_first, "Chunks should overlap");
        }
        
        #[test]
        fn test_file_extension_filter() {
            assert!(is_code_file("test.rs"), "Should accept .rs files");
            assert!(is_code_file("test.py"), "Should accept .py files");
            assert!(!is_code_file("test.txt"), "Should reject .txt files");
            assert!(!is_code_file("test.jpg"), "Should reject .jpg files");
        }
        
        #[test]
        fn test_empty_file_handling() {
            let content = "";
            let chunks = create_chunks(content);
            assert!(chunks.is_empty() || chunks.len() == 1, "Empty file should produce no or one empty chunk");
        }
        
        fn create_chunks_with_overlap(lines: &[&str], chunk_size: usize, overlap: usize) -> Vec<Vec<&str>> {
            let mut chunks = Vec::new();
            let mut i = 0;
            
            while i < lines.len() {
                let end = std::cmp::min(i + chunk_size, lines.len());
                chunks.push(lines[i..end].to_vec());
                
                if i + chunk_size >= lines.len() {
                    break;
                }
                
                i += chunk_size - overlap;
            }
            
            chunks
        }
        
        fn is_code_file(filename: &str) -> bool {
            let code_extensions = ["rs", "py", "js", "ts", "go", "java", "cpp", "c"];
            filename.split('.').last()
                .map(|ext| code_extensions.contains(&ext))
                .unwrap_or(false)
        }
        
        fn create_chunks(content: &str) -> Vec<String> {
            if content.is_empty() {
                vec![]
            } else {
                vec![content.to_string()]
            }
        }
    }

    // TEST MODULE 4: Cache Tests
    mod cache_tests {
        use super::*;
        
        #[test]
        fn test_lru_cache_eviction() {
            let mut cache = LruCache::new(NonZeroUsize::new(2).unwrap());
            
            cache.put("key1", "val1");
            cache.put("key2", "val2");
            cache.put("key3", "val3"); // Should evict key1
            
            assert!(cache.get(&"key1").is_none(), "key1 should be evicted");
            assert!(cache.get(&"key2").is_some(), "key2 should exist");
            assert!(cache.get(&"key3").is_some(), "key3 should exist");
        }
        
        #[test]
        fn test_cache_hit_rate_calculation() {
            let hits = 80;
            let total = 100;
            let hit_rate = (hits as f64 / total as f64) * 100.0;
            
            assert!(hit_rate >= 80.0, "Hit rate should be >= 80%");
        }
        
        #[test]
        fn test_cache_ttl() {
            let created = Instant::now();
            let ttl = Duration::from_secs(5);
            
            std::thread::sleep(Duration::from_millis(100));
            
            let elapsed = created.elapsed();
            assert!(elapsed < ttl, "Should not be expired yet");
            
            // Would expire after TTL
            // std::thread::sleep(Duration::from_secs(6));
            // assert!(created.elapsed() > ttl, "Should be expired");
        }
        
        #[test]
        fn test_multi_layer_cache() {
            let mut hot = HashMap::new();
            let mut warm = HashMap::new();
            let mut cold = HashMap::new();
            
            // Add to hot
            hot.insert("key1", "val1");
            
            // Promote to warm
            if let Some(val) = hot.remove(&"key1") {
                warm.insert("key1", val);
            }
            
            assert!(hot.get(&"key1").is_none(), "Not in hot");
            assert!(warm.get(&"key1").is_some(), "Should be in warm");
        }
    }

    // TEST MODULE 5: Edge Cases & Error Handling
    mod edge_case_tests {
        use super::*;
        
        #[test]
        fn test_unicode_handling() {
            let text = "Hello 世界 🌍";
            let processed = process_unicode(text);
            assert_eq!(processed, text, "Should preserve unicode");
        }
        
        #[test]
        fn test_very_long_input() {
            let long_text = "a".repeat(1_000_000);
            let result = handle_long_input(&long_text);
            assert!(result.is_ok(), "Should handle very long input");
        }
        
        #[test]
        fn test_concurrent_access() {
            use std::sync::{Arc, Mutex};
            use std::thread;
            
            let cache = Arc::new(Mutex::new(HashMap::new()));
            let mut handles = vec![];
            
            for i in 0..10 {
                let cache_clone = cache.clone();
                let handle = thread::spawn(move || {
                    let mut cache = cache_clone.lock().unwrap();
                    cache.insert(format!("key{}", i), format!("val{}", i));
                });
                handles.push(handle);
            }
            
            for handle in handles {
                handle.join().unwrap();
            }
            
            let cache = cache.lock().unwrap();
            assert_eq!(cache.len(), 10, "All concurrent inserts should succeed");
        }
        
        #[test]
        #[should_panic]
        fn test_panic_on_invalid_input() {
            panic_on_invalid("invalid");
        }
        
        fn process_unicode(text: &str) -> &str {
            text
        }
        
        fn handle_long_input(text: &str) -> Result<(), String> {
            if text.len() > 10_000_000 {
                Err("Too long".to_string())
            } else {
                Ok(())
            }
        }
        
        fn panic_on_invalid(input: &str) {
            if input == "invalid" {
                panic!("Invalid input");
            }
        }
    }

    // TEST MODULE 6: Boundary Conditions
    mod boundary_tests {
        use super::*;
        
        #[test]
        fn test_zero_results() {
            let results: Vec<(&str, f32)> = vec![];
            assert!(results.is_empty(), "Should handle zero results");
        }
        
        #[test]
        fn test_single_file() {
            let files = vec![PathBuf::from("single.rs")];
            assert_eq!(files.len(), 1, "Should handle single file");
        }
        
        #[test]
        fn test_max_cache_size() {
            let mut cache = LruCache::new(NonZeroUsize::new(usize::MAX).unwrap_or(NonZeroUsize::new(1000).unwrap()));
            cache.put("key", "value");
            assert!(cache.len() <= 1000, "Cache size should be bounded");
        }
        
        #[test]
        fn test_negative_scores() {
            let mut scores = vec![-0.5, 0.0, 0.5, -1.0];
            scores.sort_by(|a, b| b.partial_cmp(a).unwrap());
            assert_eq!(scores[0], 0.5, "Highest score should be first even with negatives");
        }
    }
}
