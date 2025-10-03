# Phase 2: Performance Critical Systems (3 weeks)
## Targeting 85% Total Memory Reduction

## 🎯 STRICT SUCCESS CRITERIA - MUST ACHIEVE ALL
- [ ] **Memory Target**: < 150MB total (vs current 2GB indexing)
- [ ] **LanceDB Performance**: < 5ms semantic search queries
- [ ] **Vector Index**: < 100MB storage for 1M+ code chunks
- [ ] **Tree-sitter Speed**: > 10K lines/second parsing
- [ ] **Symbol Extraction**: < 50ms for 1K line files
- [ ] **Cache Hit Rate**: > 90% for repeated operations
- [ ] **Incremental Updates**: < 10ms for small file changes
- [ ] **Load Test**: Handle 100+ concurrent searches

**GATE**: Phase 3 starts ONLY when search performs 10x faster than current.

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED : TRANSLATION ONLY - PRESERVE YEARS OF WORK
**THIS IS BATTLE-TESTED AI - JUST CHANGE THE LANGUAGE**

**MANDATORY STUDY - TRANSLATE LINE BY LINE**:
- `/home/verma/lapce/Codex/src/` - Port EXACT logic
- `/home/verma/lapce/Codex/src` - Same algorithms
- `/home/verma/lapce/Codex/src/` - Same condensing
- `/home/verma/lapce/Codex/src/` - Same counting

**TRANSLATION RULES**:
- TypeScript → Rust syntax only
- Keep same variable names (snake_case)
- Keep same function flow
- Keep same data structures
- Keep same algorithms
- Keep same decisions
- NO "improvements" - this AI took YEARS to perfect

### Week 1: Semantic Search with LanceDB
**Current Issue:** Qdrant client + embeddings using 40MB+ RAM
**Rust Solution:** LanceDB embedded vector database (pure Rust)

```rust
use lancedb::prelude::*;
use candle_core::{Device, Tensor};
use candle_transformers::models::bert::{BertModel, Config};

pub struct SemanticIndexer {
    db: Arc<lancedb::Database>,
    encoder: Arc<BertModel>,  // Shared model instance
    device: Device,
}

impl SemanticIndexer {
    pub async fn index_code(&self, code: &str, path: &Path) -> Result<()> {
        // Generate embeddings using candle (Rust ML framework)
        let embeddings = self.encode_text(code)?;
        
        // Store in LanceDB - columnar format, minimal memory
        let table = self.db.open_table("code_index").await?;
        table.add(&[
            ("path", path.to_str()),
            ("content", code),
            ("vector", embeddings.to_vec1::<f32>()?),
        ]).await?;
        
        Ok(())
    }
    
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_embedding = self.encode_text(query)?;
        
        // Vector similarity search - extremely fast
        let table = self.db.open_table("code_index").await?;
        table.search(&query_embedding)
            .limit(limit)
            .execute()
            .await?
    }
}
```

**Memory Savings:** 40MB → 10MB
**Why LanceDB over alternatives:**
- **Pure Rust**, no external services needed
- **Columnar storage** - 10x less memory than row-based
- **Built-in versioning** - perfect for code changes
- **Zero-copy reads** - mmap-based access
- **Apache Arrow format** - industry standard

### Week 1.5: Streaming & Compression Pipeline
**Current Issue:** Multiple buffers for streaming, 20MB+ during active sessions
**Rust Solution:** Zero-allocation streaming with zstd

```rust
use zstd::stream::{Encoder, Decoder};
use tokio::io::{AsyncRead, AsyncWrite};

pub struct StreamPipeline {
    // Reusable compression contexts
    encoder: Arc<Mutex<Encoder<'static>>>,
    decoder: Arc<Mutex<Decoder<'static>>>,
}

impl StreamPipeline {
    pub fn compress_stream<R, W>(&self, input: R, output: W) 
    where 
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        // Direct streaming without intermediate buffers
        let encoder = self.encoder.lock().unwrap();
        tokio::io::copy_buf(
            &mut BufReader::new(encoder.wrap(input)),
            &mut BufWriter::new(output)
        ).await?;
    }
}

// SSE parsing without allocations
pub struct SseParser {
    buffer: BytesMut,  // Reusable buffer
}

impl SseParser {
    pub fn parse_chunk(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        
        while let Some(line_end) = self.buffer.iter().position(|&b| b == b'\n') {
            // Parse without allocating strings
            let line = &self.buffer[..line_end];
            if line.starts_with(b"data: ") {
                events.push(SseEvent::from_bytes(&line[6..]));
            }
            self.buffer.advance(line_end + 1);
        }
        
        events
    }
}
```

**Memory Savings:** 20MB → 2MB

### Week 2: Context Management with Tantivy
**Current Issue:** In-memory context windows using 30MB+
**Rust Solution:** Tantivy for full-text search + context extraction

```rust
use tantivy::{Index, IndexWriter, Document};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;

pub struct ContextManager {
    index: Index,
    writer: Arc<Mutex<IndexWriter>>,
    searcher_pool: SearcherPool,  // Reuse searchers
}

impl ContextManager {
    pub fn extract_context(&self, file: &Path, range: Range<usize>) -> Context {
        // Memory-mapped file access
        let mmap = unsafe { Mmap::map(&File::open(file)?)? };
        
        // Extract context without copying
        let context_slice = &mmap[range];
        
        // Parse AST incrementally
        let tree = self.parse_incrementally(context_slice);
        
        Context {
            content: Cow::Borrowed(context_slice),
            symbols: self.extract_symbols(&tree),
            imports: self.extract_imports(&tree),
        }
    }
    
    pub async fn search_context(&self, query: &str) -> Vec<ContextMatch> {
        let searcher = self.searcher_pool.acquire().await;
        let query = self.query_parser.parse_query(query)?;
        
        // Fast BM25 scoring
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;
        
        top_docs.into_iter()
            .map(|(score, doc_address)| {
                let doc = searcher.doc(doc_address)?;
                ContextMatch::from_doc(doc, score)
            })
            .collect()
    }
}
```

**Memory Savings:** 30MB → 5MB

### Week 2.5: Cache Optimization
**Current Issue:** Multiple caching layers, redundant data
**Rust Solution:** Unified cache with eviction policies

```rust
use moka::future::Cache;
use blake3::Hasher;

pub struct UnifiedCache {
    // Single cache for all data types
    cache: Cache<CacheKey, CacheValue>,
    
    // Bloom filter for fast negative lookups
    bloom: Arc<RwLock<BloomFilter>>,
}

impl UnifiedCache {
    pub fn new(max_memory_mb: usize) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_memory_mb * 1024 * 1024)
            .weigher(|_key, value: &CacheValue| value.size())
            .time_to_live(Duration::from_secs(300))
            .build();
            
        Self {
            cache,
            bloom: Arc::new(RwLock::new(BloomFilter::new(10000, 0.01))),
        }
    }
    
    pub async fn get_or_compute<F, Fut>(&self, key: &str, compute: F) -> CacheValue
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = CacheValue>,
    {
        // Fast path: bloom filter check
        if !self.bloom.read().unwrap().contains(key) {
            let value = compute().await;
            self.insert(key, value.clone()).await;
            return value;
        }
        
        // Check cache
        self.cache.get_or_insert_with(key, compute).await
    }
}
```

**Memory Savings:** 15MB → 3MB

## Advanced Optimizations

### 1. SIMD-Accelerated Text Processing
```rust
use packed_simd::u8x32;

pub fn find_newlines_simd(text: &[u8]) -> Vec<usize> {
    let mut positions = Vec::new();
    let newline = u8x32::splat(b'\n');
    
    let chunks = text.chunks_exact(32);
    let remainder = chunks.remainder();
    
    for (i, chunk) in chunks.enumerate() {
        let vector = u8x32::from_slice_unaligned(chunk);
        let matches = vector.eq(newline);
        
        for j in 0..32 {
            if matches.extract(j) {
                positions.push(i * 32 + j);
            }
        }
    }
    
    // Handle remainder
    for (i, &byte) in remainder.iter().enumerate() {
        if byte == b'\n' {
            positions.push(text.len() - remainder.len() + i);
        }
    }
    
    positions
}
```

### 2. Lock-Free Statistics Collection
```rust
use atomic_float::AtomicF64;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct PerformanceStats {
    request_count: AtomicU64,
    total_latency: AtomicF64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl PerformanceStats {
    pub fn record_request(&self, latency_ms: f64) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        // Lock-free float addition
        let mut current = self.total_latency.load(Ordering::Relaxed);
        loop {
            let new = current + latency_ms;
            match self.total_latency.compare_exchange_weak(
                current,
                new,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current = x,
            }
        }
    }
}
```

## Dependencies to Add
```toml
[dependencies]
# Vector database
lancedb = "0.10"
arrow = "52.0"

# ML for embeddings
candle-core = "0.6"
candle-transformers = "0.6"

# Full-text search
tantivy = "0.22"

# Caching
moka = { version = "0.12", features = ["future"] }
blake3 = "1.5"
bloom = "0.3"

# Compression
zstd = "0.13"

# SIMD
packed_simd = { version = "0.3", package = "packed_simd_2" }

# Lock-free data structures
crossbeam-skiplist = "0.1"
atomic_float = "0.1"
```

## Expected Results - Phase 2
- **Memory:** 40MB → 20MB (additional 50% reduction)
- **Search Latency:** 100ms → 5ms (95% reduction)
- **Indexing Speed:** 10x faster
- **Cache Hit Rate:** 85%+ 

## Total Progress After Phase 2
- **Total Memory:** 135MB → 20MB (85% reduction)
- **Performance:** 5-10x faster across all operations
- **Scalability:** Can handle 100K+ files efficiently
