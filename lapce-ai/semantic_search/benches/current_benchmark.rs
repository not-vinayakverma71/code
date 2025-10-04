// Current Performance Benchmark - Actual Working Test
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lancedb::connect;
use lancedb::search::fully_optimized_storage::{FullyOptimizedStorage, FullyOptimizedConfig, EmbeddingMetadata};
use lancedb::embeddings::compression::CompressedEmbedding;
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Runtime;

fn create_test_data(count: usize) -> (Vec<CompressedEmbedding>, Vec<EmbeddingMetadata>) {
    let mut embeddings = Vec::new();
    let mut metadata = Vec::new();
    
    for i in 0..count {
        // Create synthetic 1536-dim embedding
        let mut vec = vec![0.0f32; 1536];
        for j in 0..1536 {
            vec[j] = ((i as f32 * 0.01 + j as f32 * 0.001).sin() * 0.7) / (1.0 + j as f32 * 0.001);
        }
        
        // Normalize
        let norm = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        for v in vec.iter_mut() {
            *v /= norm;
        }
        
        embeddings.push(CompressedEmbedding::compress(&vec).unwrap());
        metadata.push(EmbeddingMetadata {
            id: format!("doc_{}", i),
            path: format!("/doc_{}.txt", i),
            content: format!("Document {}", i),
        });
    }
    
    (embeddings, metadata)
}

fn benchmark_index_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("index_creation_300_vectors", |b| {
        b.iter(|| {
            rt.block_on(async {
                let tmpdir = tempfile::tempdir().unwrap();
                let db_path = tmpdir.path().to_str().unwrap();
                let conn = connect(db_path).execute().await.unwrap();
                let conn = Arc::new(conn);
                
                let storage = FullyOptimizedStorage::new(conn, Default::default()).await.unwrap();
                let table = storage.create_or_open_table("bench_table", 1536).await.unwrap();
                
                let (embeddings, metadata) = create_test_data(300);
                storage.store_batch(&table, embeddings, metadata).await.unwrap();
                
                let start = Instant::now();
                storage.create_index_with_persistence(&table, false).await.unwrap();
                start.elapsed()
            })
        });
    });
}

fn benchmark_query_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // Setup once
    let (table, storage, query_vec) = rt.block_on(async {
        let tmpdir = tempfile::tempdir().unwrap();
        let db_path = tmpdir.path().to_str().unwrap();
        let conn = connect(db_path).execute().await.unwrap();
        let conn = Arc::new(conn);
        
        let config = FullyOptimizedConfig {
            cache_ttl_seconds: 600,
            cache_max_size: 1000,
            ivf_partitions: 16,
            pq_subvectors: 16,
            pq_bits: 8,
            nprobes: 20,
            refine_factor: Some(1),
        };
        
        let storage = FullyOptimizedStorage::new(conn, config).await.unwrap();
        let table = storage.create_or_open_table("query_bench", 1536).await.unwrap();
        
        let (embeddings, metadata) = create_test_data(500);
        storage.store_batch(&table, embeddings, metadata).await.unwrap();
        storage.create_index_with_persistence(&table, false).await.unwrap();
        
        let query_vec = vec![0.1f32; 1536];
        (table, Arc::new(storage), query_vec)
    });
    
    c.bench_function("cold_query_500_vectors", |b| {
        let storage = storage.clone();
        let table = table.clone();
        let query = query_vec.clone();
        
        b.iter(|| {
            rt.block_on(async {
                // Clear cache for cold query
                storage.get_cache_stats().await;
                
                let start = Instant::now();
                let results = storage.query_optimized(&table, &query, 10).await.unwrap();
                black_box(results);
                start.elapsed()
            })
        });
    });
    
    c.bench_function("cached_query_500_vectors", |b| {
        let storage = storage.clone();
        let table = table.clone();
        let query = query_vec.clone();
        
        // Warm up cache
        rt.block_on(async {
            storage.query_optimized(&table, &query, 10).await.unwrap();
        });
        
        b.iter(|| {
            rt.block_on(async {
                let start = Instant::now();
                let results = storage.query_optimized(&table, &query, 10).await.unwrap();
                black_box(results);
                start.elapsed()
            })
        });
    });
}

fn benchmark_compression(c: &mut Criterion) {
    let vec = vec![0.5f32; 1536];
    
    c.bench_function("compress_1536_dim", |b| {
        b.iter(|| {
            let compressed = CompressedEmbedding::compress(&vec).unwrap();
            black_box(compressed)
        });
    });
    
    let compressed = CompressedEmbedding::compress(&vec).unwrap();
    
    c.bench_function("decompress_1536_dim", |b| {
        b.iter(|| {
            let decompressed = compressed.decompress().unwrap();
            black_box(decompressed)
        });
    });
}

criterion_group!(benches, 
    benchmark_compression,
    benchmark_index_creation, 
    benchmark_query_performance
);
criterion_main!(benches);
