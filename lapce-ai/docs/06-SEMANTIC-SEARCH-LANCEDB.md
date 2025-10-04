# Step 6: Semantic Search with LanceDB
## Vector Database for Code Intelligence

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED : TYPESCRIPT → RUST TRANSLATION ONLY
**YEARS OF SEARCH TUNING - TRANSLATE, DON'T REDESIGN**

**TRANSLATE LINE-BY-LINE FROM**: 
- `/home/verma/lapce/Codex/tools/codebaseSearchTool.ts`
- `/home/verma/lapce/Codex/tools/searchFilesTool.ts`
- Same ranking algorithms (just Rust syntax)
- Same result format (AI depends on it)
## Achieving 10MB Memory Footprint with Sub-5ms Query Latency

## ✅ Success Criteria
- [ ] **Memory Usage**: < 10MB including embeddings
- [ ] **Query Latency**: < 5ms for top-10 results
- [ ] **Index Speed**: > 1000 files/second (Not necessary)
- [ ] **Accuracy**: > 90% relevance score
- [ ] **Incremental Indexing**: < 100ms per file update
- [ ] **Cache Hit Rate**: > 80% for repeated queries
- [ ] **Concurrent Queries**: Handle 100+ simultaneous searches
- [ ] **Test Coverage**: Index 100+ code files successfully

## Overview
LanceDB provides columnar storage with vector similarity search, offering 75% memory reduction compared to traditional vector databases while maintaining production-grade performance.

## Core Architecture

### LanceDB Integration
```rust
use lancedb::{Connection, Table, Query};
use arrow::array::{Float32Array, StringArray, StructArray};
use arrow::datatypes::{DataType, Field, Schema};
use candle_core::{Device, Tensor};
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
BERT or Small Embedding model

pub struct SemanticSearchEngine {
    // LanceDB connection
    connection: Arc<Connection>,
    
    // Embedding model
    embedder: Arc<EmbeddingModel>,
    
    // Table references
    code_table: Arc<Table>,
    doc_table: Arc<Table>,
    
    // Query cache
    query_cache: Arc<QueryCache>,
    
    // Metrics
    metrics: Arc<SearchMetrics>,
}
```

## LanceDB Setup and Configuration

### 1. Database Initialization
```rust
impl SemanticSearchEngine {
    pub async fn new(config: SearchConfig) -> Result<Self> {
        // Initialize LanceDB connection
        let connection = lancedb::connect(&config.db_path)
            .execute()
            .await?;
            
        // Create or open tables
        let code_table = Self::create_code_table(&connection).await?;
        let doc_table = Self::create_doc_table(&connection).await?;
        
        // Initialize embedding model
        let embedder = Arc::new(EmbeddingModel::new(&config.model_config)?);
        
        // Setup query cache
        let query_cache = Arc::new(QueryCache::new(
            config.cache_size,
            Duration::from_secs(config.cache_ttl),
        ));
        
        Ok(Self {
            connection: Arc::new(connection),
            embedder,
            code_table: Arc::new(code_table),
            doc_table: Arc::new(doc_table),
            query_cache,
            metrics: Arc::new(SearchMetrics::new()),
        })
    }
    
    async fn create_code_table(conn: &Connection) -> Result<Table> {
        // Define schema for code embeddings
        let schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("path", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("language", DataType::Utf8, true),
            Field::new("start_line", DataType::Int32, false),
            Field::new("end_line", DataType::Int32, false),
            Field::new("vector", DataType::FixedSizeList(
                Box::new(Field::new("item", DataType::Float32, true)),
                768, // BERT embedding dimension  //BERT or Small Embedding model
            ), false),
            Field::new("metadata", DataType::Utf8, true),
            Field::new("timestamp", DataType::Timestamp(TimeUnit::Millisecond, None), false),
        ]);
        
        // Create table with optimized settings
        conn.create_table("code_embeddings", schema)
            .with_vector_column("vector", 768)
            .with_metric("cosine")
            .with_index_type("IVF_PQ")  // Inverted File with Product Quantization
            .with_num_partitions(100)
            .with_num_sub_vectors(32)
            .execute()
            .await
    }
}
```

### 2. Embedding Model Implementation - Currently disable - Use external api of aws titan 
```rust
use candle_nn::{Module, VarBuilder};
use tokenizers::tokenizer::Tokenizer;

impl EmbeddingModel {
    pub fn new(config: &ModelConfig) -> Result<Self> {
        // Load model weights
        let device = Device::cuda_if_available(0)?;
        let weights = candle_core::safetensors::load(
            &config.model_path,
            &device,
        )?;
        
        // Build BERT model /// BERT or Small Embedding model
        let var_builder = VarBuilder::from_tensors(weights, DType::F32, &device);
        let bert_config = BertConfig::from_file(&config.config_path)?;
        let model = BertModel::new(&bert_config, var_builder)?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&config.tokenizer_path)?;
        
        Ok(Self {
            model,
            tokenizer,
            device,
            pooling: config.pooling_strategy.clone(),
            batch_size: config.batch_size,
        })
    }
    
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        // Tokenize input
        let encoding = self.tokenizer.encode(text, true)?;
        let input_ids = Tensor::new(
            encoding.get_ids(),
            &self.device,
        )?;
        
        let attention_mask = Tensor::new(
            encoding.get_attention_mask(),
            &self.device,
        )?;
        
        // Forward pass
        let output = self.model.forward(&input_ids, &attention_mask)?;
        
        // Apply pooling
        let pooled = match self.pooling {
            PoolingStrategy::Mean => Self::mean_pooling(&output, &attention_mask)?,
            PoolingStrategy::CLS => output.i((.., 0, ..))?,
            PoolingStrategy::Max => Self::max_pooling(&output)?,
            PoolingStrategy::LastToken => {
                let seq_len = output.dim(1)?;
                output.i((.., seq_len - 1, ..))?
            }
        };
        
        // Normalize embeddings
        let normalized = Self::l2_normalize(&pooled)?;
        
        // Convert to Vec<f32>
        Ok(normalized.to_vec1()?)
    }
}
```

## Indexing Pipeline

### 1. Code Indexer
```rust
pub struct CodeIndexer {
    search_engine: Arc<SemanticSearchEngine>,
    parser: Arc<CodeParser>,
    batch_size: usize,
    index_queue: Arc<Mutex<VecDeque<IndexTask>>>,
}

impl CodeIndexer {
    pub async fn index_repository(&self, repo_path: &Path) -> Result<IndexStats> {
        let mut stats = IndexStats::default();
        
        // Walk repository files
        let files = self.collect_files(repo_path).await?;
        
        // Process in batches
        for chunk in files.chunks(self.batch_size) {
            let batch_results = self.process_batch(chunk).await?;
            stats.merge(batch_results);
        }
        
        // Optimize index
        self.search_engine.optimize_index().await?;
        
        Ok(stats)
    }
    
    async fn process_batch(&self, files: &[PathBuf]) -> Result<IndexStats> {
        let mut embeddings = Vec::new();
        let mut metadata = Vec::new();
        
        for file in files {
            // Parse code chunks
            let chunks = self.parser.parse_file(file).await?;
            
            for chunk in chunks {
                // Generate embedding
                let embedding = self.search_engine
                    .embedder
                    .embed_text(&chunk.content)
                    .await?;
                    
                embeddings.push(embedding);
                metadata.push(ChunkMetadata {
                    path: file.clone(),
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    language: chunk.language,
                });
            }
        }
        
        // Batch insert into LanceDB
        self.batch_insert(embeddings, metadata).await
    }
    
    async fn batch_insert(
        &self,
        embeddings: Vec<Vec<f32>>,
        metadata: Vec<ChunkMetadata>,
    ) -> Result<IndexStats> {
        // Create Arrow arrays
        let id_array = StringArray::from_iter_values(
            metadata.iter().map(|m| uuid::Uuid::new_v4().to_string())
        );
        
        let path_array = StringArray::from_iter_values(
            metadata.iter().map(|m| m.path.to_string_lossy())
        );
        
        let vector_array = Float32Array::from_iter_values(
            embeddings.into_iter().flatten()
        );
        
        // Create record batch
        let batch = RecordBatch::try_new(
            self.search_engine.code_table.schema(),
            vec![
                Arc::new(id_array),
                Arc::new(path_array),
                Arc::new(vector_array),
            ],
        )?;
        
        // Insert into LanceDB
        self.search_engine.code_table
            .add(batch)
            .execute()
            .await?;
            
        Ok(IndexStats {
            files_indexed: metadata.len(),
            chunks_created: metadata.len(),
            ..Default::default()
        })
    }
}
```

## Query Execution

### 1. Semantic Search
```rust
impl SemanticSearchEngine {
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        let start = Instant::now();
        
        // Check cache
        let cache_key = self.compute_cache_key(query, &filters);
        if let Some(cached) = self.query_cache.get(&cache_key).await {
            self.metrics.record_cache_hit();
            return Ok(cached);
        }
        
        // Generate query embedding
        let query_embedding = self.embedder.embed_text(query).await?;
        
        // Build LanceDB query
        let mut lance_query = self.code_table
            .search(&query_embedding)
            .limit(limit)
            .metric("cosine");
            
        // Apply filters
        if let Some(filters) = filters {
            lance_query = self.apply_filters(lance_query, filters);
        }
        
        // Execute query
        let results = lance_query
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
            
        // Convert to search results
        let search_results = self.convert_results(results)?;
        
        // Update cache
        self.query_cache.insert(cache_key, search_results.clone()).await;
        
        // Record metrics
        self.metrics.record_search(start.elapsed(), search_results.len());
        
        Ok(search_results)
    }
    
    fn apply_filters(
        &self,
        mut query: Query,
        filters: SearchFilters,
    ) -> Query {
        if let Some(language) = filters.language {
            query = query.filter(format!("language = '{}'", language));
        }
        
        if let Some(path_pattern) = filters.path_pattern {
            query = query.filter(format!("path LIKE '%{}%'", path_pattern));
        }
        
        if let Some(min_score) = filters.min_score {
            query = query.filter(format!("_distance >= {}", min_score));
        }
        
        query
    }
}
```

### 2. Hybrid Search (Keyword + Semantic)
```rust
pub struct HybridSearcher {
    semantic_engine: Arc<SemanticSearchEngine>,
    keyword_index: Arc<TantivyIndex>,
    fusion_weight: f32,
}

impl HybridSearcher {
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Run both searches in parallel
        let (semantic_results, keyword_results) = tokio::join!(
            self.semantic_engine.search(query, limit * 2, None),
            self.keyword_index.search(query, limit * 2)
        );
        
        let semantic_results = semantic_results?;
        let keyword_results = keyword_results?;
        
        // Reciprocal Rank Fusion
        self.fuse_results(semantic_results, keyword_results, limit)
    }
    
    fn fuse_results(
        &self,
        semantic: Vec<SearchResult>,
        keyword: Vec<SearchResult>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut scores = HashMap::new();
        let k = 60.0; // RRF constant
        
        // Score semantic results
        for (rank, result) in semantic.iter().enumerate() {
            let score = self.fusion_weight / (k + rank as f32 + 1.0);
            scores.entry(&result.id)
                .and_modify(|s| *s += score)
                .or_insert(score);
        }
        
        // Score keyword results
        for (rank, result) in keyword.iter().enumerate() {
            let score = (1.0 - self.fusion_weight) / (k + rank as f32 + 1.0);
            scores.entry(&result.id)
                .and_modify(|s| *s += score)
                .or_insert(score);
        }
        
        // Sort by fused score
        let mut fused: Vec<_> = scores.into_iter().collect();
        fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Return top results
        Ok(fused.into_iter()
            .take(limit)
            .filter_map(|(id, _score)| {
                semantic.iter()
                    .chain(keyword.iter())
                    .find(|r| &r.id == id)
                    .cloned()
            })
            .collect())
    }
}
```

## Performance Optimizations

### 1. Query Cache
```rust
use moka::future::Cache;

pub struct QueryCache {
    cache: Cache<String, Vec<SearchResult>>,
    hasher: blake3::Hasher,
}

impl QueryCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_size)
            .time_to_live(ttl)
            .build();
            
        Self {
            cache,
            hasher: blake3::Hasher::new(),
        }
    }
    
    pub fn compute_cache_key(&self, query: &str, filters: &Option<SearchFilters>) -> String {
        let mut hasher = self.hasher.clone();
        hasher.update(query.as_bytes());
        
        if let Some(filters) = filters {
            hasher.update(format!("{:?}", filters).as_bytes());
        }
        
        hasher.finalize().to_hex().to_string()
    }
}
```

### 2. Incremental Indexing
```rust
pub struct IncrementalIndexer {
    search_engine: Arc<SemanticSearchEngine>,
    file_watcher: FileWatcher,
    change_buffer: Arc<Mutex<Vec<FileChange>>>,
}

impl IncrementalIndexer {
    pub async fn start(&self) -> Result<()> {
        let mut rx = self.file_watcher.subscribe();
        
        loop {
            tokio::select! {
                Some(change) = rx.recv() => {
                    self.handle_change(change).await?;
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    self.flush_changes().await?;
                }
            }
        }
    }
    
    async fn handle_change(&self, change: FileChange) -> Result<()> {
        match change.kind {
            ChangeKind::Create | ChangeKind::Modify => {
                // Re-index file
                let chunks = self.parse_file(&change.path).await?;
                self.update_embeddings(chunks).await?;
            }
            ChangeKind::Delete => {
                // Remove from index
                self.search_engine.code_table
                    .delete()
                    .filter(format!("path = '{}'", change.path.display()))
                    .execute()
                    .await?;
            }
        }
        
        Ok(())
    }
}
```
**YOU CAN USE ANY OTHER MODELS**
## Memory Profile
- **LanceDB connection**: 2MB
- **Query cache**: 1MB
- **Index metadata**: 2MB
- **Total**: ~10MB (vs 40MB with Qdrant)
