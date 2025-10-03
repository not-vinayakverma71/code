// Final Consolidated Production System with Full Fallback Support
// Integrates all optimizations: SIMD, Compression, Caching, Index Persistence
// Production-ready with AWS Titan integration

use crate::error::{Error, Result};
use crate::search::fully_optimized_storage::{
    FullyOptimizedStorage, FullyOptimizedConfig, EmbeddingMetadata, SearchResult
};
use crate::embeddings::compression::CompressedEmbedding;
use crate::embeddings::aws_titan_robust::{RobustAwsTitan, RobustConfig};
use crate::embeddings::aws_titan_production::AwsTier;
use crate::embeddings::embedder_interface::IEmbedder;
use crate::optimization::simd_kernels::SimdCapabilities;
use crate::{connect, Connection};

use std::sync::Arc;
use std::time::{Instant, Duration};
use std::path::PathBuf;
use tracing::{info, warn, debug};

/// Production system configuration
#[derive(Clone)]
pub struct ProductionConfig {
    // Database
    pub db_path: String,
    pub table_name: String,
    
    // AWS Titan
    pub aws_region: String,
    pub aws_tier: AwsTier,
    
    // Performance settings
    pub enable_simd: bool,
    pub enable_cache: bool,
    pub cache_ttl_seconds: u64,
    pub cache_max_size: usize,
    
    // Index configuration
    pub ivf_partitions: usize,
    pub pq_subvectors: usize,
    pub pq_bits: usize,
    pub nprobes: usize,
    pub refine_factor: Option<usize>,
    
    // Robust handling
    pub max_retries: usize,
    pub requests_per_second: f64,
    pub batch_size: usize,
}

impl Default for ProductionConfig {
    fn default() -> Self {
        Self {
            db_path: "./lancedb_production".to_string(),
            table_name: "semantic_embeddings".to_string(),
            aws_region: "us-east-1".to_string(),
            aws_tier: AwsTier::Standard,
            enable_simd: true,
            enable_cache: true,
            cache_ttl_seconds: 600,
            cache_max_size: 10000,
            ivf_partitions: 16,
            pq_subvectors: 16,
            pq_bits: 8,
            nprobes: 20,
            refine_factor: Some(1),
            max_retries: 3,
            requests_per_second: 2.0,
            batch_size: 5,
        }
    }
}

/// Final production system with all optimizations
pub struct ProductionSystem {
    config: ProductionConfig,
    connection: Arc<Connection>,
    storage: Arc<FullyOptimizedStorage>,
    embedder: Arc<RobustAwsTitan>,
    simd_available: bool,
}

impl ProductionSystem {
    /// Initialize production system
    pub async fn new(config: ProductionConfig) -> Result<Self> {
        info!("Initializing Production Semantic Search System");
        
        // Detect SIMD capabilities
        let capabilities = SimdCapabilities::detect();
        let simd_available = config.enable_simd && (capabilities.has_avx2 || capabilities.has_avx512);
        
        info!("System capabilities:");
        info!("  SIMD: {}", if simd_available { 
            format!("✅ Enabled (AVX2: {}, AVX-512: {})", capabilities.has_avx2, capabilities.has_avx512)
        } else {
            "⚠️ Falling back to scalar".to_string()
        });
        info!("  Cache: {}", if config.enable_cache { "✅ Enabled" } else { "❌ Disabled" });
        
        // Connect to database
        let connection = connect(&config.db_path)
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to connect: {}", e)
            })?;
        
        // Initialize AWS Titan with robust handling
        let robust_config = RobustConfig {
            max_retries: config.max_retries,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 5000,
            max_concurrent_requests: 3,
            requests_per_second: config.requests_per_second,
            batch_size: config.batch_size,
            request_timeout_secs: 30,
            enable_cache_fallback: true,
        };
        
        let embedder = RobustAwsTitan::new(
            &config.aws_region,
            config.aws_tier.clone(),
            robust_config
        ).await?;
        
        // Create optimized storage
        let storage_config = FullyOptimizedConfig {
            cache_ttl_seconds: config.cache_ttl_seconds,
            cache_max_size: config.cache_max_size,
            ivf_partitions: config.ivf_partitions,
            pq_subvectors: config.pq_subvectors,
            pq_bits: config.pq_bits,
            nprobes: config.nprobes,
            refine_factor: config.refine_factor,
        };
        
        let storage = FullyOptimizedStorage::new(
            Arc::new(connection.clone()),
            storage_config
        ).await?;
        
        Ok(Self {
            config,
            connection: Arc::new(connection),
            storage: Arc::new(storage),
            embedder: Arc::new(embedder),
            simd_available,
        })
    }
    
    /// Initialize or open table
    pub async fn initialize(&self) -> Result<()> {
        let table = self.storage.create_or_open_table(
            &self.config.table_name,
            1536  // AWS Titan dimension
        ).await?;
        
        info!("Table '{}' ready", self.config.table_name);
        
        // Check for existing index
        let has_index = self.storage.create_index_with_persistence(&table, false).await?;
        info!("Index status: {:?}", has_index);
        
        Ok(())
    }
    
    /// Add documents to the system
    pub async fn add_documents(&self, file_paths: Vec<PathBuf>) -> Result<usize> {
        let start = Instant::now();
        info!("Processing {} documents", file_paths.len());
        
        // Extract text and prepare for embedding
        let mut texts = Vec::new();
        let mut metadata = Vec::new();
        
        for (idx, path) in file_paths.iter().enumerate() {
            let content = tokio::fs::read_to_string(path)
                .await
                .unwrap_or_else(|_| String::new());
            
            if content.len() > 100 {
                // Take first 2000 chars for embedding
                let chunk: String = content.chars().take(2000).collect();
                texts.push(chunk.clone());
                
                metadata.push(EmbeddingMetadata {
                    id: format!("doc_{}", idx),
                    path: path.to_str().unwrap_or("unknown").to_string(),
                    content: chunk,
                });
            }
        }
        
        if texts.is_empty() {
            warn!("No valid documents to process");
            return Ok(0);
        }
        
        // Generate embeddings
        info!("Generating embeddings for {} documents", texts.len());
        let embeddings = self.embedder
            .create_embeddings(texts, None)
            .await?;
        
        // Compress embeddings
        let mut compressed = Vec::new();
        for embedding in embeddings.embeddings {
            compressed.push(CompressedEmbedding::compress(&embedding)
                .map_err(|e| Error::Runtime {
                    message: format!("Compression failed: {}", e)
                })?);
        }
        
        // Store in database
        let table = self.storage.create_or_open_table(
            &self.config.table_name,
            1536
        ).await?;
        
        let doc_count = compressed.len();
        self.storage.store_batch(&table, compressed, metadata).await?;
        
        // Update index
        self.storage.create_index_with_persistence(&table, false).await?;
        
        info!("Added {} documents in {:?}", doc_count, start.elapsed());
        Ok(doc_count)
    }
    
    /// Search for similar documents
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let start = Instant::now();
        
        // Generate query embedding
        let query_response = self.embedder
            .create_embeddings(vec![query.to_string()], None)
            .await?;
        
        if query_response.embeddings.is_empty() {
            return Err(Error::Runtime {
                message: "Failed to generate query embedding".to_string()
            });
        }
        
        let query_embedding = &query_response.embeddings[0];
        
        // Open table and perform search
        let table = self.storage.create_or_open_table(
            &self.config.table_name,
            1536
        ).await?;
        
        let results = self.storage.query_optimized(
            &table,
            query_embedding,
            limit
        ).await?;
        
        debug!("Search completed in {:?}", start.elapsed());
        
        Ok(results)
    }
    
    /// Get system statistics
    pub async fn get_stats(&self) -> Result<SystemStats> {
        let cache_stats = self.storage.get_cache_stats().await;
        
        let table = self.storage.create_or_open_table(
            &self.config.table_name,
            1536
        ).await?;
        
        let doc_count = table.count_rows(None).await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to count rows: {}", e)
            })?;
        
        Ok(SystemStats {
            total_documents: doc_count,
            cache_hit_rate: cache_stats.hit_rate,
            cache_size: cache_stats.size,
            simd_enabled: self.simd_available,
        })
    }
    
    /// Run performance benchmark
    pub async fn benchmark(&self) -> Result<BenchmarkResults> {
        info!("Running production benchmark...");
        
        // Generate test query
        let query = "semantic search optimization performance";
        
        let mut latencies = Vec::new();
        
        // Warm up
        let _ = self.search(query, 10).await?;
        
        // Run multiple queries
        for i in 0..20 {
            let start = Instant::now();
            let results = self.search(query, 10).await?;
            let elapsed = start.elapsed();
            latencies.push(elapsed);
            
            if i == 0 {
                info!("First query: {:?}, {} results", elapsed, results.len());
            }
        }
        
        // Calculate percentiles
        latencies.sort();
        let p50 = latencies[latencies.len() / 2];
        let p95 = latencies[latencies.len() * 95 / 100];
        let p99 = latencies[(latencies.len() * 99 / 100).min(latencies.len() - 1)];
        
        let stats = self.get_stats().await?;
        
        Ok(BenchmarkResults {
            p50_latency: p50,
            p95_latency: p95,
            p99_latency: p99,
            cache_hit_rate: stats.cache_hit_rate,
            simd_enabled: stats.simd_enabled,
            total_documents: stats.total_documents,
        })
    }
}

#[derive(Debug)]
pub struct SystemStats {
    pub total_documents: usize,
    pub cache_hit_rate: f64,
    pub cache_size: usize,
    pub simd_enabled: bool,
}

#[derive(Debug)]
pub struct BenchmarkResults {
    pub p50_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
    pub cache_hit_rate: f64,
    pub simd_enabled: bool,
    pub total_documents: usize,
}
