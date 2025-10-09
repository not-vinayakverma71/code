// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Core Semantic Search Engine Implementation - Following doc/06-SEMANTIC-SEARCH-LANCEDB.md

use crate::error::{Error, Result};
use crate::embeddings::service_factory::IEmbedder;
use crate::search::improved_cache::ImprovedQueryCache;
use crate::search::search_metrics::{SearchMetrics, SEARCH_LATENCY, CACHE_MISSES_TOTAL};
use crate::memory::profiler::{MemoryProfiler, MemoryDashboard};
use arrow_array::{Float32Array, StringArray, Int32Array, RecordBatch, ArrayRef};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use futures::TryStreamExt;
use lance::dataset::Dataset;
use crate::{Connection, Table};
use crate::index::{Index, vector::IvfPqIndexBuilder};
use crate::query::{Query as LanceQuery, ExecutableQuery, QueryBase};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Search configuration with optimizations
#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub db_path: String,
    pub cache_size: usize,
    pub cache_ttl: u64,  // seconds
    pub batch_size: usize,
    pub max_results: usize,
    pub min_score: f32,
    pub optimal_batch_size: Option<usize>, // Adaptive batch sizing
    pub max_embedding_dim: Option<usize>,  // Support high dimensions
    pub index_nprobes: Option<usize>,      // Query optimization
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            db_path: "./lancedb".to_string(),
            cache_size: 1000,
            cache_ttl: 300,  // 5 minutes
            batch_size: 100,
            max_results: 10,
            min_score: 0.5,
            optimal_batch_size: Some(50),
            max_embedding_dim: Some(1536),
            index_nprobes: Some(10),
        }
    }
}

/// Main semantic search engine - Lines 38-54 from doc
pub struct SemanticSearchEngine {
    // LanceDB connection
    connection: Arc<Connection>,
    
    // Embedding model (using our IEmbedder trait)
    pub(crate) embedder: Arc<dyn IEmbedder>,
    
    // Table references
    pub(crate) code_table: Arc<RwLock<Option<Table>>>,
    pub(crate) doc_table: Arc<RwLock<Option<Table>>>,
    
    // Query cache
    pub query_cache: Arc<ImprovedQueryCache>,
    
    // Metrics
    pub(crate) metrics: Arc<SearchMetrics>,
    
    // Configuration
    config: SearchConfig,
    
    // Memory profiling
    memory_profiler: Arc<MemoryProfiler>,
    memory_dashboard: Arc<RwLock<MemoryDashboard>>,
}

impl SemanticSearchEngine {
    /// Initialize engine - Lines 62-89 from doc
    pub async fn new(
        config: SearchConfig,
        embedder: Arc<dyn IEmbedder>,
    ) -> Result<Self> {
        // Initialize LanceDB connection  
        let connection = crate::connect(&config.db_path)
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to connect to LanceDB: {}", e)
            })?;
            
        // Setup query cache
        let query_cache = Arc::new(ImprovedQueryCache::new(
            config.cache_ttl as u64,
            config.cache_size,
        ));
        
        // Initialize memory profiling
        let memory_profiler = Arc::new(MemoryProfiler::new());
        let memory_dashboard = Arc::new(RwLock::new(MemoryDashboard::new(memory_profiler.clone())));
        
        let engine = Self {
            connection: Arc::new(connection),
            embedder,
            code_table: Arc::new(RwLock::new(None)),
            doc_table: Arc::new(RwLock::new(None)),
            query_cache,
            config: config.clone(),
            metrics: Arc::new(SearchMetrics::new()),
            memory_profiler,
            memory_dashboard,
        };
        
        // Create or open tables
        engine.initialize_tables().await?;
        
        // Create or refresh vector indices
        engine.ensure_vector_indices().await?;
        
        Ok(engine)
    }
    
    /// Create or open tables - Lines 91-118 from doc
    async fn initialize_tables(&self) -> Result<()> {
        // Try to open existing tables first
        let existing_tables = self.connection
            .table_names()
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to list tables: {}", e)
            })?;
        
        // Create or open code table
        if existing_tables.contains(&"code_embeddings".to_string()) {
            let table = self.connection
                .open_table("code_embeddings")
                .execute()
                .await
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to open code table: {}", e)
                })?;
            *self.code_table.write().await = Some(table);
        } else {
            self.create_code_table().await?;
        }
        
        // Create or open doc table
        if existing_tables.contains(&"doc_embeddings".to_string()) {
            let table = self.connection
                .open_table("doc_embeddings")
                .execute()
                .await
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to open doc table: {}", e)
                })?;
            *self.doc_table.write().await = Some(table);
        } else {
            self.create_doc_table().await?;
        }
        
        Ok(())
    }
    
    /// Create document embeddings table
    async fn create_doc_table(&self) -> Result<()> {
        // Define schema for document embeddings
        let vector_dim = self.config.max_embedding_dim.unwrap_or(768) as i32;
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("doc_path", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("doc_type", DataType::Utf8, true),  // markdown, txt, etc
            Field::new("section", DataType::Utf8, true),   // heading/section name
            Field::new("vector", DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, false)),
                vector_dim,
            ), false),
            Field::new("metadata", DataType::Utf8, true),
            Field::new("timestamp", DataType::Timestamp(TimeUnit::Millisecond, None), false),
        ]));
        
        // Create empty initial batch for doc table
        let id_array = StringArray::from(vec![] as Vec<&str>);
        let doc_path_array = StringArray::from(vec![] as Vec<&str>);
        let content_array = StringArray::from(vec![] as Vec<&str>);
        let doc_type_array = StringArray::from(vec![] as Vec<Option<&str>>);
        let section_array = StringArray::from(vec![] as Vec<Option<&str>>);
        
        // Create empty vector array
        let vector_values = Float32Array::from(vec![] as Vec<f32>);
        let vector_field = Arc::new(Field::new("item", DataType::Float32, false));
        let vector_array = arrow_array::FixedSizeListArray::new(
            vector_field, vector_dim, Arc::new(vector_values), None
        );
        
        let metadata_array = StringArray::from(vec![] as Vec<Option<&str>>);
        let timestamp_array = arrow_array::TimestampMillisecondArray::from(vec![] as Vec<i64>);
        
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array) as ArrayRef,
                Arc::new(doc_path_array) as ArrayRef,
                Arc::new(content_array) as ArrayRef,
                Arc::new(doc_type_array) as ArrayRef,
                Arc::new(section_array) as ArrayRef,
                Arc::new(vector_array) as ArrayRef,
                Arc::new(metadata_array) as ArrayRef,
                Arc::new(timestamp_array) as ArrayRef,
            ],
        ).map_err(|e| Error::Runtime {
            message: format!("Failed to create doc batch: {}", e)
        })?;
        
        // Create table - use RecordBatchIterator
        let batches = vec![batch];
        let reader = arrow_array::RecordBatchIterator::new(
            batches.into_iter().map(Ok),
            schema.clone()
        );
        let table = self.connection
            .create_table("doc_embeddings", Box::new(reader))
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to create doc table: {}", e)
            })?;
            
        *self.doc_table.write().await = Some(table);
        Ok(())
    }
    
    /// Create code embeddings table - Lines 91-117 from doc
    async fn create_code_table(&self) -> Result<()> {
        // Define schema for code embeddings
        let vector_dim = self.config.max_embedding_dim.unwrap_or(768) as i32;
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("path", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("language", DataType::Utf8, true),
            Field::new("start_line", DataType::Int32, false),
            Field::new("end_line", DataType::Int32, false),
            Field::new("vector", DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, false)),
                vector_dim, // Match configured embedding dimension
            ), false),
            Field::new("metadata", DataType::Utf8, true),
            Field::new("timestamp", DataType::Timestamp(TimeUnit::Millisecond, None), false),
        ]));
        
        // Create empty initial batch
        let id_array = StringArray::from(vec![] as Vec<&str>);
        let path_array = StringArray::from(vec![] as Vec<&str>);
        let content_array = StringArray::from(vec![] as Vec<&str>);
        let language_array = StringArray::from(vec![] as Vec<Option<&str>>);
        let start_line_array = Int32Array::from(vec![] as Vec<i32>);
        let end_line_array = Int32Array::from(vec![] as Vec<i32>);
        
        // Create empty vector array with proper shape
        let vector_values = Float32Array::from(vec![] as Vec<f32>);
        let vector_field = Arc::new(Field::new("item", DataType::Float32, false));
        let vector_array = arrow_array::FixedSizeListArray::new(
            vector_field, vector_dim, Arc::new(vector_values), None
        );
        
        let metadata_array = StringArray::from(vec![] as Vec<Option<&str>>);
        let timestamp_array = arrow_array::TimestampMillisecondArray::from(vec![] as Vec<i64>);
        
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array) as ArrayRef,
                Arc::new(path_array) as ArrayRef,
                Arc::new(content_array) as ArrayRef,
                Arc::new(language_array) as ArrayRef,
                Arc::new(start_line_array) as ArrayRef,
                Arc::new(end_line_array) as ArrayRef,
                Arc::new(vector_array) as ArrayRef,
                Arc::new(metadata_array) as ArrayRef,
                Arc::new(timestamp_array) as ArrayRef,
            ],
        ).map_err(|e| Error::Runtime {
            message: format!("Failed to create record batch: {}", e)
        })?;
        
        // Create table with optimized settings
        let batches = vec![batch];
        let reader = arrow_array::RecordBatchIterator::new(
            batches.into_iter().map(Ok),
            schema.clone()
        );
        let table = self.connection
            .create_table("code_embeddings", Box::new(reader))
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to create code table: {}", e)
            })?;
            
        *self.code_table.write().await = Some(table);
        Ok(())
    }
    
    /// Optimize index for better performance - Lines 213 from doc
    pub async fn optimize_index(&self) -> Result<()> {
        let code_table_guard = self.code_table.read().await;
        if let Some(table) = code_table_guard.as_ref() {
            // Compact fragments to optimize storage
            table.optimize(crate::table::OptimizeAction::Compact { 
                options: Default::default(),
                remap_options: None
            }).await.map_err(|e| Error::Runtime {
                message: format!("Failed to optimize table: {}", e)
            })?;
            
            // Reindex if needed
            let indices = table.list_indices().await.map_err(|e| Error::Runtime {
                message: format!("Failed to list indices: {}", e)
            })?;
            
            if !indices.is_empty() {
                log::info!("Table optimized with {} indices", indices.len());
            }
        }
        
        let doc_table_guard = self.doc_table.read().await;
        if let Some(table) = doc_table_guard.as_ref() {
            table.optimize(crate::table::OptimizeAction::Compact {
                options: Default::default(),
                remap_options: None
            }).await.map_err(|e| Error::Runtime {
                message: format!("Failed to optimize doc table: {}", e)
            })?;
        }
        
        Ok(())
    }
    
    /// Convert LanceDB results to SearchResult format - Lines 332 from doc
    fn convert_results(&self, batch: &RecordBatch) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        
        // Extract columns
        let ids = batch.column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let paths = batch.column_by_name("path")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let contents = batch.column_by_name("content")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let scores = batch.column_by_name("_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
        let start_lines = batch.column_by_name("start_line")
            .and_then(|c| c.as_any().downcast_ref::<Int32Array>());
        let end_lines = batch.column_by_name("end_line")
            .and_then(|c| c.as_any().downcast_ref::<Int32Array>());
        
        let num_rows = batch.num_rows();
        for i in 0..num_rows {
            results.push(SearchResult {
                id: ids.and_then(|a| a.value(i).parse().ok()).unwrap_or_default(),
                path: paths.map(|a| a.value(i).to_string()).unwrap_or_default(),
                content: contents.map(|a| a.value(i).to_string()).unwrap_or_default(),
                score: scores.map(|a| a.value(i)).unwrap_or(0.0),
                start_line: start_lines.map(|a| a.value(i) as usize).unwrap_or(0),
                end_line: end_lines.map(|a| a.value(i) as usize).unwrap_or(0),
                language: None,  // Can be extracted from file extension if needed
                metadata: HashMap::new(),
            });
        }
        
        Ok(results)
    }
    
    
    /// Ensure vector indices exist on all tables
    async fn ensure_vector_indices(&self) -> Result<()> {
        // Create index on code table if it exists and has data
        let code_table_guard = self.code_table.read().await;
        if let Some(table) = code_table_guard.as_ref() {
            let row_count = table.count_rows(None).await.unwrap_or(0);
            if row_count > 0 {
                drop(code_table_guard);
                self.create_vector_index_with_params(256, 48, 10).await?;
            }
        }
        
        // Create index on doc table if it exists and has data
        let doc_table_guard = self.doc_table.read().await;
        if let Some(table) = doc_table_guard.as_ref() {
            let row_count = table.count_rows(None).await.unwrap_or(0);
            if row_count > 0 {
                // Create index for doc table too
                let indices = table.list_indices().await.map_err(|e| Error::Runtime {
                    message: format!("Failed to list doc indices: {}", e)
                })?;
                
                if !indices.iter().any(|idx| idx.name == "doc_vector_idx") {
                    table.create_index(&["vector"], Index::IvfPq(
                        IvfPqIndexBuilder::default()
                            .distance_type(crate::DistanceType::Cosine)
                            .num_partitions(256)
                            .num_sub_vectors(48)
                    )).name("doc_vector_idx".to_string()).execute().await.map_err(|e| Error::Runtime {
                        message: format!("Failed to create doc vector index: {}", e)
                    })?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Create optimized vector index with configurable parameters
    pub async fn create_vector_index(&self) -> Result<()> {
        self.create_vector_index_with_params(256, 48, 10).await
    }
    
    /// Create vector index with custom optimization parameters
    pub async fn create_vector_index_with_params(
        &self, 
        num_partitions: usize,
        num_sub_vectors: usize,
        _nprobes: usize
    ) -> Result<()> {
        let code_table_guard = self.code_table.read().await;
        if let Some(table) = code_table_guard.as_ref() {
            // Check if index already exists
            let indices = table.list_indices().await.map_err(|e| Error::Runtime {
                message: format!("Failed to list indices: {}", e)
            })?;
            
            // Create optimized IVF PQ index if not exists
            if !indices.iter().any(|idx| idx.name == "vector_idx") {
                println!("Creating IVF_PQ index with {} partitions and {} subvectors", 
                    num_partitions, num_sub_vectors);
                    
                table.create_index(&["vector"], Index::IvfPq(
                    IvfPqIndexBuilder::default()
                        .distance_type(crate::DistanceType::Cosine)
                        .num_partitions(num_partitions as u32)
                        .num_sub_vectors(num_sub_vectors as u32)
                )).name("vector_idx".to_string()).execute().await.map_err(|e| Error::Runtime {
                    message: format!("Failed to create vector index: {}", e)
                })?;
                
                println!("✅ Vector index created successfully");
            }
        }
        Ok(())
    }
    
    /// Main search method - Lines 295-341 from doc
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        let start = Instant::now();
        
        // Check cache with filter-aware key
        let filters_string = filters.as_ref().map(|f| format!("{:?}", f));
        let cache_key = self.query_cache.compute_cache_key_with_filters(
            query, 
            filters_string.as_deref()
        );
        let cache_start = std::time::Instant::now();
        if let Some(cached) = self.query_cache.get(&cache_key).await {
            self.metrics.record_cache_hit(cache_start.elapsed());
            return Ok(cached);
        }
        
        // Generate query embedding
        let embeddings = self.embedder.create_embeddings(vec![query.to_string()], None).await
            .map_err(|_| Error::Runtime {
                message: "Failed to generate query embedding".to_string()
            })?;
        
        let query_embedding = embeddings.embeddings.into_iter().next()
            .ok_or_else(|| Error::Runtime {
                message: "No embeddings returned".to_string()
            })?;
        
        // Perform vector search
        let code_table_guard = self.code_table.read().await;
        if let Some(table) = code_table_guard.as_ref() {
            // Build LanceDB query
            let query_builder = table
                .vector_search(query_embedding)
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to create vector search: {}", e)
                })?
                .limit(limit);
            
            // Execute query
            let mut results_stream = query_builder
                .execute()
                .await
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to execute search: {}", e)
                })?;
            
            let mut search_results = Vec::new();
            
            // Process results
            use futures::TryStreamExt;
            while let Some(batch) = results_stream.try_next().await.map_err(|e| Error::Runtime {
                message: format!("Failed to read results: {}", e)
            })? {
                // Use the new convert_results method
                let batch_results = self.convert_results(&batch)?;
                search_results.extend(batch_results);
            }
            
            // Sort by score (highest first)
            search_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            
            // Cache the results with filter-aware key
            self.query_cache.put(cache_key, search_results.clone()).await;
        
            // Record metrics
            let duration = start.elapsed();
            self.metrics.record_search(duration, search_results.len());
            SEARCH_LATENCY.with_label_values(&["search"]).observe(duration.as_secs_f64());
            CACHE_MISSES_TOTAL.inc();
            
            Ok(search_results)
        } else {
            Err(Error::Runtime {
                message: "Code table not initialized".to_string()
            })
        }
    }
    
    /// Batch insert for indexing - Lines 247-286 from doc
    pub async fn batch_insert(
        &self,
        embeddings: Vec<Vec<f32>>,
        metadata: Vec<ChunkMetadata>,
    ) -> Result<IndexStats> {
        // Validate embedding dimensions
        let expected_dim = self.config.max_embedding_dim.unwrap_or(1536);
        for (idx, embedding) in embeddings.iter().enumerate() {
            if embedding.len() != expected_dim {
                return Err(Error::Runtime {
                    message: format!(
                        "Embedding dimension mismatch at index {}: expected {}, got {}",
                        idx, expected_dim, embedding.len()
                    )
                });
            }
        }
        
        let code_table_guard = self.code_table.read().await;
        if let Some(table) = code_table_guard.as_ref() {
            // Create Arrow arrays
            let id_array = StringArray::from_iter_values(
                metadata.iter().map(|_| Uuid::new_v4().to_string())
            );
            
            let path_array = StringArray::from_iter_values(
                metadata.iter().map(|m| m.path.to_string_lossy())
            );
            
            let content_array = StringArray::from_iter_values(
                metadata.iter().map(|m| &m.content)
            );
            
            let language_array = StringArray::from(
                metadata.iter()
                    .map(|m| m.language.as_deref())
                    .collect::<Vec<_>>()
            );
            
            let start_line_array = Int32Array::from_iter_values(
                metadata.iter().map(|m| m.start_line as i32)
            );
            
            let end_line_array = Int32Array::from_iter_values(
                metadata.iter().map(|m| m.end_line as i32)
            );
            
            // Get embedding dimension from first embedding
            let embedding_dim = if !embeddings.is_empty() {
                embeddings[0].len()
            } else {
                1536 // Default dimension
            };
            
            // Create vector array
            let flat_vectors: Vec<f32> = embeddings.into_iter().flatten().collect();
            let vector_values = Float32Array::from(flat_vectors);
            let vector_field = Arc::new(Field::new("item", DataType::Float32, false));
            let vector_array = arrow_array::FixedSizeListArray::new(
                vector_field, embedding_dim as i32, Arc::new(vector_values), None
            );
            
            let metadata_array = StringArray::from(
                metadata.iter()
                    .map(|m| serde_json::to_string(&m.metadata).ok())
                    .collect::<Vec<_>>()
            );
            
            let timestamp = chrono::Utc::now().timestamp_millis();
            let timestamp_array = arrow_array::TimestampMillisecondArray::from(
                vec![timestamp; metadata.len()]
            );
            
            // Get the schema for the batch
            let schema = Arc::new(arrow_schema::Schema::new(vec![
                arrow_schema::Field::new("id", arrow_schema::DataType::Utf8, false),
                arrow_schema::Field::new("path", arrow_schema::DataType::Utf8, false),
                arrow_schema::Field::new("content", arrow_schema::DataType::Utf8, false),
                arrow_schema::Field::new("language", arrow_schema::DataType::Utf8, true),
                arrow_schema::Field::new("start_line", arrow_schema::DataType::Int32, false),
                arrow_schema::Field::new("end_line", arrow_schema::DataType::Int32, false),
                arrow_schema::Field::new("vector", arrow_schema::DataType::FixedSizeList(
                    Arc::new(arrow_schema::Field::new("item", arrow_schema::DataType::Float32, false)),
                    embedding_dim as i32
                ), false),
                arrow_schema::Field::new("metadata", arrow_schema::DataType::Utf8, true),
                arrow_schema::Field::new("timestamp", arrow_schema::DataType::Timestamp(
                    arrow_schema::TimeUnit::Millisecond,
                    None
                ), false),
            ]));
            
            // Create record batch
            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(id_array) as ArrayRef,
                    Arc::new(path_array) as ArrayRef,
                    Arc::new(content_array) as ArrayRef,
                    Arc::new(language_array) as ArrayRef,
                    Arc::new(start_line_array) as ArrayRef,
                    Arc::new(end_line_array) as ArrayRef,
                    Arc::new(vector_array) as ArrayRef,
                    Arc::new(metadata_array) as ArrayRef,
                    Arc::new(timestamp_array) as ArrayRef,
                ],
            ).map_err(|e| Error::Runtime {
                message: format!("Failed to create record batch: {}", e)
            })?;
            
            // Insert into LanceDB
            let batches = vec![batch];
            let schema = batches[0].schema();
            let reader = arrow_array::RecordBatchIterator::new(
                batches.into_iter().map(Ok),
                schema
            );
            table.add(Box::new(reader))
                .execute()
                .await
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to insert batch: {}", e)
                })?;
            
            // Drop the table guard before calling ensure_vector_indices
            drop(code_table_guard);
            
            // Optimize table after large batch (>1000 chunks)
            if metadata.len() > 1000 {
                self.optimize_index().await?;
            }
            
            // Refresh index after batch insert
            self.ensure_vector_indices().await?;
            
            Ok(IndexStats {
                files_indexed: metadata.len(),
                chunks_created: metadata.len(),
                time_elapsed: Duration::from_secs(0),
            })
        } else {
            Err(Error::Runtime {
                message: "Code table not initialized".to_string()
            })
        }
    }
    
    /// Pre-warm the query cache for better performance
    pub async fn prewarm_cache(&self, num_queries: usize) -> Result<()> {
        println!("Pre-warming query cache with {} queries...", num_queries);
        
        // Common query patterns to warm up
        let warmup_queries = vec![
            "function", "impl", "struct", "async", "trait",
            "pub fn", "use std", "Result", "Error", "Vec",
            "HashMap", "Arc", "tokio", "serde", "clone"
        ];
        
        for i in 0..num_queries.min(warmup_queries.len()) {
            let query = warmup_queries[i];
            
            // Execute query to populate cache
            let _ = self.search(query, 10, None).await?;
            
            if i % 5 == 0 {
                println!("  Warmed up {} queries", i + 1);
            }
        }
        
        println!("✅ Query cache pre-warming complete");
        Ok(())
    }
    
    /// Delete entries by file path
    pub async fn delete_by_path(&self, path: &Path) -> Result<()> {
        let code_table_guard = self.code_table.read().await;
        if let Some(table) = code_table_guard.as_ref() {
            table.delete(&format!("path = '{}'", path.display()))
                .await
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to delete entries: {}", e)
                })?;
        }
        Ok(())
    }
    
    /// Get memory profiling report
    pub fn get_memory_report(&self) -> crate::memory::profiler::MemoryReport {
        self.memory_profiler.get_memory_report()
    }
    
    /// Detect memory leaks
    pub fn detect_memory_leaks(&self) -> Vec<crate::memory::profiler::LeakCandidate> {
        self.memory_profiler.detect_leaks()
    }
    
    /// Get hot allocation paths
    pub fn get_hot_paths(&self, top_n: usize) -> Vec<crate::memory::profiler::HotPath> {
        self.memory_profiler.get_hot_paths(top_n)
    }
    
    /// Print memory dashboard
    pub async fn print_memory_dashboard(&self) {
        let mut dashboard = self.memory_dashboard.write().await;
        dashboard.print_dashboard();
    }
    
    /// Check if steady state memory target is achieved
    pub fn is_steady_state(&self) -> bool {
        crate::memory::profiler::is_steady_state()
    }
}

/// Search result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub path: String,
    pub content: String,
    pub language: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub score: f32,
    pub metadata: HashMap<String, String>,
}

/// Search filters - Lines 299-361 from doc
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub language: Option<String>,
    pub path_pattern: Option<String>,
    pub min_score: Option<f32>,
}

/// Chunk metadata for indexing
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub path: std::path::PathBuf,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub language: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Index statistics
#[derive(Debug, Default)]
pub struct IndexStats {
    pub files_indexed: usize,
    pub chunks_created: usize,
    pub time_elapsed: Duration,
}
