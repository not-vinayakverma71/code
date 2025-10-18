// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

//! Performance benchmarks for incremental indexing (CST-PERF01 & CST-PERF02)
//!
//! Target: <1ms per 1k nodes for canonical mapping

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;

#[cfg(feature = "cst_ts")]
use lancedb::indexing::{
    StableIdEmbeddingCache, IncrementalDetector, CachedEmbedder,
    EmbeddingModel, EmbeddingStats,
};

// Mock embedding model for benchmarking
#[cfg(feature = "cst_ts")]
struct BenchEmbeddingModel;

#[cfg(feature = "cst_ts")]
impl EmbeddingModel for BenchEmbeddingModel {
    fn embed(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        // Fast mock embedding
        Ok(vec![0.1; 384])
    }
    
    fn dimension(&self) -> usize {
        384
    }
}

#[cfg(feature = "cst_ts")]
fn benchmark_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");
    
    let cache = StableIdEmbeddingCache::new();
    let embedding = vec![0.1; 384];
    
    // Insert benchmark
    group.bench_function("insert", |b| {
        let mut id = 0u64;
        b.iter(|| {
            cache.insert(
                black_box(id),
                black_box("fn test() {}"),
                black_box(&embedding),
                black_box("function_definition"),
                black_box("/test.rs"),
            );
            id += 1;
        });
    });
    
    // Lookup benchmark (hit)
    cache.insert(12345, "fn test() {}", &embedding, "function_definition", "/test.rs");
    group.bench_function("lookup_hit", |b| {
        b.iter(|| {
            let result = cache.get(black_box(12345));
            black_box(result)
        });
    });
    
    // Lookup benchmark (miss)
    group.bench_function("lookup_miss", |b| {
        b.iter(|| {
            let result = cache.get(black_box(99999));
            black_box(result)
        });
    });
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
fn benchmark_change_detection(c: &mut Criterion) {
    use lancedb::ast::{CstNode, NodeMetadata};
    use std::collections::HashMap;
    
    let mut group = c.benchmark_group("change_detection");
    
    // Generate test nodes
    fn make_nodes(count: usize) -> Vec<CstNode> {
        (0..count).map(|i| CstNode {
            kind: "function_definition".to_string(),
            text: format!("fn test_{}() {{}}", i),
            start_byte: i * 20,
            end_byte: (i + 1) * 20,
            start_point: (i, 0),
            end_point: (i, 20),
            children: vec![],
            metadata: NodeMetadata {
                semantic_info: None,
                canonical_kind: Some("function".to_string()),
                stable_id: Some(i as u64),
            },
        }).collect()
    }
    
    for node_count in [100, 500, 1000, 5000] {
        group.throughput(Throughput::Elements(node_count as u64));
        
        let old_nodes = make_nodes(node_count);
        let mut new_nodes = old_nodes.clone();
        
        // Modify 10% of nodes
        let modify_count = node_count / 10;
        for i in 0..modify_count {
            new_nodes[i].text = format!("fn modified_{}() {{}}", i);
        }
        
        // Collect stable IDs
        let old_ids: HashMap<u64, &CstNode> = old_nodes.iter()
            .filter_map(|n| n.metadata.stable_id.map(|id| (id, n)))
            .collect();
        
        group.bench_with_input(
            BenchmarkId::from_parameter(node_count),
            &(old_ids, new_nodes),
            |b, (old_ids, new_nodes)| {
                b.iter(|| {
                    let detector = IncrementalDetector::new(old_ids.clone());
                    let changeset = detector.detect_changes(black_box(new_nodes));
                    black_box(changeset)
                });
            }
        );
    }
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
fn benchmark_cached_embedding(c: &mut Criterion) {
    use lancedb::ast::{CstNode, NodeMetadata};
    use std::sync::Arc;
    
    let mut group = c.benchmark_group("cached_embedding");
    
    let model = Arc::new(BenchEmbeddingModel);
    let embedder = Arc::new(CachedEmbedder::new(model));
    
    // Create test node
    let node = CstNode {
        kind: "function_definition".to_string(),
        text: "fn test() { println!(\"hello\"); }".to_string(),
        start_byte: 0,
        end_byte: 35,
        start_point: (0, 0),
        end_point: (0, 35),
        children: vec![],
        metadata: NodeMetadata {
            semantic_info: None,
            canonical_kind: Some("function".to_string()),
            stable_id: Some(12345),
        },
    };
    
    // Cache miss (first embed)
    group.bench_function("cache_miss", |b| {
        b.iter(|| {
            let result = embedder.embed_node(black_box(&node));
            black_box(result)
        });
    });
    
    // Cache hit (reuse)
    embedder.embed_node(&node).unwrap();
    group.bench_function("cache_hit", |b| {
        b.iter(|| {
            let result = embedder.embed_node(black_box(&node));
            black_box(result)
        });
    });
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
fn benchmark_throughput_comparison(c: &mut Criterion) {
    use lancedb::ast::{CstNode, NodeMetadata};
    use std::sync::Arc;
    
    let mut group = c.benchmark_group("throughput_comparison");
    group.measurement_time(Duration::from_secs(10));
    
    let model = Arc::new(BenchEmbeddingModel);
    
    // Generate 1000 nodes
    let nodes: Vec<CstNode> = (0..1000).map(|i| CstNode {
        kind: "function_definition".to_string(),
        text: format!("fn test_{}() {{ return {}; }}", i, i),
        start_byte: i * 30,
        end_byte: (i + 1) * 30,
        start_point: (i, 0),
        end_point: (i, 30),
        children: vec![],
        metadata: NodeMetadata {
            semantic_info: None,
            canonical_kind: Some("function".to_string()),
            stable_id: Some(i as u64),
        },
    }).collect();
    
    // Full embedding (no cache)
    group.bench_function("full_embedding", |b| {
        b.iter(|| {
            let embedder = CachedEmbedder::new(model.clone());
            for node in &nodes {
                let _ = embedder.embed_node(black_box(node));
            }
        });
    });
    
    // Incremental (90% cached)
    group.bench_function("incremental_90pct_cached", |b| {
        b.iter(|| {
            let embedder = CachedEmbedder::new(model.clone());
            
            // Populate cache with 90%
            for node in &nodes[..900] {
                let _ = embedder.embed_node(node);
            }
            
            // Re-process all (90% cache hits)
            for node in &nodes {
                let _ = embedder.embed_node(black_box(node));
            }
        });
    });
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
fn benchmark_large_file_scenarios(c: &mut Criterion) {
    use lancedb::ast::{CstNode, NodeMetadata};
    
    let mut group = c.benchmark_group("large_file_scenarios");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(15));
    
    // Simulate large file with 10k nodes
    fn make_large_cst(node_count: usize) -> Vec<CstNode> {
        (0..node_count).map(|i| CstNode {
            kind: if i % 3 == 0 { "function_definition" } else { "expression_statement" }.to_string(),
            text: format!("node_{}", i),
            start_byte: i * 10,
            end_byte: (i + 1) * 10,
            start_point: (i / 50, i % 50),
            end_point: (i / 50, (i % 50) + 10),
            children: vec![],
            metadata: NodeMetadata {
                semantic_info: None,
                canonical_kind: Some(if i % 3 == 0 { "function" } else { "statement" }.to_string()),
                stable_id: Some(i as u64),
            },
        }).collect()
    }
    
    for node_count in [1000, 5000, 10000] {
        group.throughput(Throughput::Elements(node_count as u64));
        
        let nodes = make_large_cst(node_count);
        
        group.bench_with_input(
            BenchmarkId::new("parse", node_count),
            &nodes,
            |b, nodes| {
                b.iter(|| {
                    // Simulate processing
                    let mut count = 0;
                    for node in nodes {
                        if node.metadata.stable_id.is_some() {
                            count += 1;
                        }
                    }
                    black_box(count)
                });
            }
        );
    }
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
fn benchmark_per_node_latency(c: &mut Criterion) {
    use lancedb::ast::{CstNode, NodeMetadata};
    use std::collections::HashMap;
    
    let mut group = c.benchmark_group("per_node_latency");
    
    // Target: <1μs per node for change detection
    let old_nodes: HashMap<u64, CstNode> = (0..1000).map(|i| {
        let node = CstNode {
            kind: "function_definition".to_string(),
            text: format!("fn test_{}() {{}}", i),
            start_byte: i * 20,
            end_byte: (i + 1) * 20,
            start_point: (i, 0),
            end_point: (i, 20),
            children: vec![],
            metadata: NodeMetadata {
                semantic_info: None,
                canonical_kind: Some("function".to_string()),
                stable_id: Some(i as u64),
            },
        };
        (i as u64, node)
    }).collect();
    
    let old_ids: HashMap<u64, &CstNode> = old_nodes.iter()
        .map(|(id, node)| (*id, node))
        .collect();
    
    let new_node = CstNode {
        kind: "function_definition".to_string(),
        text: "fn new_func() {}".to_string(),
        start_byte: 0,
        end_byte: 16,
        start_point: (0, 0),
        end_point: (0, 16),
        children: vec![],
        metadata: NodeMetadata {
            semantic_info: None,
            canonical_kind: Some("function".to_string()),
            stable_id: Some(500),
        },
    };
    
    group.bench_function("lookup_single_node", |b| {
        b.iter(|| {
            let result = old_ids.get(&500);
            black_box(result)
        });
    });
    
    group.bench_function("compare_single_node", |b| {
        b.iter(|| {
            if let Some(old) = old_ids.get(&500) {
                let changed = old.text != black_box(&new_node).text;
                black_box(changed)
            }
        });
    });
    
    group.finish();
}

#[cfg(feature = "cst_ts")]
criterion_group!(
    benches,
    benchmark_cache_operations,
    benchmark_change_detection,
    benchmark_cached_embedding,
    benchmark_throughput_comparison,
    benchmark_large_file_scenarios,
    benchmark_per_node_latency,
);

#[cfg(feature = "cst_ts")]
criterion_main!(benches);

#[cfg(not(feature = "cst_ts"))]
fn main() {
    eprintln!("Error: Benchmarks require the 'cst_ts' feature.");
    eprintln!("Run with: cargo bench --features cst_ts --bench incremental_indexing_bench");
}
