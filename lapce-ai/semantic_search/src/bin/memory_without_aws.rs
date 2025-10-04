// Test memory usage WITHOUT AWS SDK - using mock embedder instead
use lancedb::error::Result;
use lancedb::embeddings::embedder_interface::{IEmbedder, EmbeddingResponse, EmbedderInfo, AvailableEmbedders};
use lancedb::search::semantic_search_engine::{SearchConfig, SemanticSearchEngine, ChunkMetadata};
use std::sync::Arc;
use async_trait::async_trait;
use std::any::Any;

// Mock embedder that doesn't use AWS SDK
struct MockEmbedder {
    dimension: usize,
}

#[async_trait]
impl IEmbedder for MockEmbedder {
    async fn create_embeddings(
        &self,
        texts: Vec<String>,
        _model: Option<&str>,
    ) -> Result<EmbeddingResponse> {
        // Generate fake embeddings
        let embeddings: Vec<Vec<f32>> = texts
            .iter()
            .map(|text| {
                // Generate deterministic embedding based on text hash
                let hash = text.len() as f32 / 100.0;
                vec![hash; self.dimension]
            })
            .collect();
        
        Ok(EmbeddingResponse {
            embeddings,
            usage: None,
        })
    }
    
    async fn validate_configuration(&self) -> Result<(bool, Option<String>)> {
        Ok((true, None))
    }
    
    fn embedder_info(&self) -> EmbedderInfo {
        EmbedderInfo {
            name: AvailableEmbedders::OpenAi,  // Just use a dummy value
        }
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn get_memory_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║          MEMORY BENCHMARK WITHOUT AWS SDK                         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");
    
    // Phase 1: Baseline
    println!("Phase 1: Baseline Memory");
    println!("═══════════════════════════════════════════════════");
    let baseline_mem = get_memory_mb();
    println!("  Initial process memory: {:.2} MB", baseline_mem);
    
    // Phase 2: Create engine with mock embedder
    println!("\nPhase 2: Initialize Engine (No AWS SDK)");
    println!("═══════════════════════════════════════════════════");
    
    let config = SearchConfig {
        db_path: "/tmp/no_aws_test".to_string(),
        cache_size: 100,
        cache_ttl: 300,
        batch_size: 10,
        max_results: 10,
        min_score: 0.5,
        optimal_batch_size: Some(10),
        max_embedding_dim: Some(384),  // Smaller dimension
        index_nprobes: Some(5),
    };
    
    let embedder = Arc::new(MockEmbedder { dimension: 384 });
    let engine = Arc::new(SemanticSearchEngine::new(config, embedder).await?);
    
    let after_init = get_memory_mb();
    println!("  After engine init: {:.2} MB", after_init);
    println!("  Delta from baseline: +{:.2} MB", after_init - baseline_mem);
    
    // Phase 3: Index some data
    println!("\nPhase 3: Index Sample Data");
    println!("═══════════════════════════════════════════════════");
    
    let mut embeddings = Vec::new();
    let mut metadata = Vec::new();
    
    // Create 100 sample documents
    for i in 0..100 {
        embeddings.push(vec![i as f32 / 100.0; 384]);
        metadata.push(ChunkMetadata {
            path: format!("/test/file_{}.rs", i).into(),
            content: format!("Sample content for document {}", i),
            start_line: i * 10,
            end_line: (i + 1) * 10,
            language: Some("rust".to_string()),
        });
    }
    
    engine.batch_insert(embeddings, metadata).await?;
    
    let after_index = get_memory_mb();
    println!("  After indexing 100 docs: {:.2} MB", after_index);
    println!("  Delta from init: +{:.2} MB", after_index - after_init);
    
    // Phase 4: Perform searches
    println!("\nPhase 4: Search Operations");
    println!("═══════════════════════════════════════════════════");
    
    for i in 0..10 {
        let query = format!("test query {}", i);
        let results = engine.search(&query, 10, None).await?;
        if i == 0 {
            println!("  First search: {} results", results.len());
        }
    }
    
    let after_search = get_memory_mb();
    println!("  After 10 searches: {:.2} MB", after_search);
    println!("  Delta from index: +{:.2} MB", after_search - after_index);
    
    // Phase 5: Test our new compression/cache components
    println!("\nPhase 5: Test Compression + Cache Components");
    println!("═══════════════════════════════════════════════════");
    
    // Create and use ZSTD compressor
    use lancedb::embeddings::zstd_compression::{ZstdCompressor, CompressionConfig};
    let mut compressor = ZstdCompressor::new(CompressionConfig::default());
    
    // Compress some embeddings
    for i in 0..50 {
        let embedding = vec![i as f32 / 50.0; 384];
        let _ = compressor.compress_embedding(&embedding, &format!("id_{}", i))?;
    }
    
    let after_compression = get_memory_mb();
    println!("  After compression tests: {:.2} MB", after_compression);
    
    // Use hierarchical cache
    use lancedb::storage::hierarchical_cache::{HierarchicalCache, CacheConfig};
    use tempfile::TempDir;
    
    let temp_dir = TempDir::new()?;
    let cache = HierarchicalCache::new(
        CacheConfig {
            l1_max_size_mb: 0.5,
            l2_max_size_mb: 1.5,
            l3_max_size_mb: 10.0,
            ..Default::default()
        },
        temp_dir.path()
    )?;
    
    // Add to cache
    for i in 0..50 {
        let embedding = vec![i as f32 / 50.0; 384];
        cache.put(&format!("cache_{}", i), embedding)?;
    }
    
    let after_cache = get_memory_mb();
    println!("  After cache operations: {:.2} MB", after_cache);
    
    // Phase 6: Summary
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                    MEMORY SUMMARY (NO AWS SDK)                    ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    
    println!("\n📊 Memory Usage Breakdown:");
    println!("  Baseline process:            {:.2} MB", baseline_mem);
    println!("  Engine initialization:      +{:.2} MB", after_init - baseline_mem);
    println!("  Indexing 100 documents:     +{:.2} MB", after_index - after_init);
    println!("  10 search operations:       +{:.2} MB", after_search - after_index);
    println!("  Compression tests:          +{:.2} MB", after_compression - after_search);
    println!("  Hierarchical cache:         +{:.2} MB", after_cache - after_compression);
    println!("  ─────────────────────────────────────");
    println!("  TOTAL WITHOUT AWS SDK:       {:.2} MB", after_cache);
    
    println!("\n📈 Comparison:");
    println!("  With AWS SDK:     ~70 MB (from real benchmark)");
    println!("  Without AWS SDK:  {:.2} MB", after_cache);
    println!("  AWS SDK overhead: ~{:.2} MB", 70.0 - after_cache);
    
    println!("\n✅ Key Findings:");
    println!("  • Core engine uses only {:.2} MB", after_init - baseline_mem);
    println!("  • LanceDB + indexing adds {:.2} MB", after_index - after_init);
    println!("  • Search cache adds {:.2} MB", after_search - after_index);
    println!("  • Total core system: {:.2} MB", after_cache);
    
    if after_cache < 10.0 {
        println!("\n🎯 SUCCESS: Memory usage WITHOUT AWS SDK is < 10MB target!");
    }
    
    Ok(())
}
