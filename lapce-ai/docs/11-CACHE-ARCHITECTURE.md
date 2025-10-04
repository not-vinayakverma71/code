# Step 9: Cache Architecture - Multi-Layer Intelligent Caching
## LRU, Query, and Embedding Cache with 3MB Footprint

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED :  TYPESCRIPT → RUST TRANSLATION ONLY - NO REDESIGN
**YEARS OF CACHE OPTIMIZATION - TRANSLATE LINE BY LINE**

**STUDY**: `/home/verma/lapce/Codex`
- Claude's multi-point caching - copy exactly (just Rust syntax)
- Cache keys and invalidation - same logic, different language
- Token counting for cache - identical algorithms
- This cache logic is PRODUCTION PROVEN - only change syntax

## ✅ Success Criteria
- [ ] **Memory Usage**: < 3MB total cache overhead
- [ ] **Cache Hit Rate**: > 85% for L1 cache
- [ ] **Query Latency**: < 1ms for cached items
- [ ] **L1 Performance**: 100K ops/second
- [ ] **L2 Disk Cache**: < 100MB disk usage
- [ ] **Eviction Efficiency**: LRU/LFU with < 1ms overhead
- [ ] **Bloom Filter**: 99% accuracy with 1% false positive
- [ ] **Test Coverage**: Cache 1M items without degradation

## Overview
Our multi-layer cache architecture combines in-memory, disk, and distributed caching with intelligent eviction policies to minimize latency while maintaining a tiny memory footprint.

## Core Cache System

### Unified Cache Manager
```rust
use moka::future::Cache as MokaCache;
use sled::Db;
use redis::aio::ConnectionManager;

pub struct CacheSystem {
    // L1: In-memory cache (hot data)
    l1_cache: Arc<L1Cache>,
    
    // L2: Disk cache (warm data)
    l2_cache: Arc<L2Cache>,
    
    // L3: Distributed cache (shared data)
    l3_cache: Option<Arc<L3Cache>>,
    
    // Cache coordinator
    coordinator: Arc<CacheCoordinator>,
    
    // Metrics
    metrics: Arc<CacheMetrics>,
}

pub struct L1Cache {
    cache: MokaCache<CacheKey, CacheValue>,
    bloom_filter: Arc<RwLock<BloomFilter>>,
    access_counter: Arc<AccessCounter>,
}

pub struct L2Cache {
    db: Db,
    compression: CompressionStrategy,
    max_size: usize,
}

pub struct L3Cache {
    redis: ConnectionManager,
    serializer: Arc<Serializer>,
}
```

## L1: In-Memory Cache

### 1. Smart Memory Cache
```rust
impl L1Cache {
    pub fn new(config: L1Config) -> Self {
        let cache = MokaCache::builder()
            .max_capacity(config.max_entries)
            .time_to_live(config.ttl)
            .time_to_idle(config.idle_time)
            .weigher(|_key, value: &CacheValue| value.size())
            .eviction_listener(|key, value, cause| {
                tracing::debug!("Evicted {:?}: {:?}", key, cause);
            })
            .build();
            
        let bloom_filter = Arc::new(RwLock::new(
            BloomFilter::new(config.bloom_size, config.bloom_fp_rate)
        ));
        
        Self {
            cache,
            bloom_filter,
            access_counter: Arc::new(AccessCounter::new()),
        }
    }
    
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // Check bloom filter first
        if !self.bloom_filter.read().await.contains(key) {
            self.metrics.record_bloom_filter_hit();
            return None;
        }
        
        // Get from cache
        if let Some(value) = self.cache.get(key).await {
            self.access_counter.record(key);
            self.metrics.record_l1_hit();
            Some(value)
        } else {
            self.metrics.record_l1_miss();
            None
        }
    }
    
    pub async fn put(&self, key: CacheKey, value: CacheValue) {
        // Update bloom filter
        self.bloom_filter.write().await.insert(&key);
        
        // Check if we should cache based on access patterns
        if self.should_cache(&key, &value).await {
            self.cache.insert(key.clone(), value).await;
            self.access_counter.record(&key);
        }
    }
    
    async fn should_cache(&self, key: &CacheKey, value: &CacheValue) -> bool {
        // Adaptive caching based on access frequency and value size
        let frequency = self.access_counter.frequency(key);
        let size = value.size();
        
        // Use logarithmic decay for size penalty
        let size_factor = 1.0 / (1.0 + (size as f64).ln());
        let frequency_factor = (frequency as f64).sqrt();
        
        let score = frequency_factor * size_factor;
        score > self.cache_threshold()
    }
}
```

### 2. Access Pattern Tracking
```rust
pub struct AccessCounter {
    counts: DashMap<CacheKey, CountMinSketch>,
    window: Duration,
}

impl AccessCounter {
    pub fn record(&self, key: &CacheKey) {
        self.counts.entry(key.clone())
            .or_insert_with(|| CountMinSketch::new(0.01, 0.99))
            .increment();
    }
    
    pub fn frequency(&self, key: &CacheKey) -> u32 {
        self.counts.get(key)
            .map(|sketch| sketch.estimate())
            .unwrap_or(0)
    }
    
    pub fn decay(&self) {
        // Periodically decay counts to adapt to changing patterns
        for mut entry in self.counts.iter_mut() {
            entry.value_mut().decay(0.9);
        }
    }
}

pub struct CountMinSketch {
    counters: Vec<Vec<u32>>,
    hash_functions: Vec<Box<dyn Fn(&[u8]) -> usize>>,
    width: usize,
    depth: usize,
}

impl CountMinSketch {
    pub fn increment(&mut self) {
        for (i, hash_fn) in self.hash_functions.iter().enumerate() {
            let hash = hash_fn(&[]) % self.width;
            self.counters[i][hash] = self.counters[i][hash].saturating_add(1);
        }
    }
    
    pub fn estimate(&self) -> u32 {
        self.hash_functions.iter()
            .enumerate()
            .map(|(i, hash_fn)| {
                let hash = hash_fn(&[]) % self.width;
                self.counters[i][hash]
            })
            .min()
            .unwrap_or(0)
    }
    
    pub fn decay(&mut self, factor: f32) {
        for row in &mut self.counters {
            for count in row {
                *count = (*count as f32 * factor) as u32;
            }
        }
    }
}
```

## L2: Disk Cache

### 1. Persistent Disk Cache
```rust
impl L2Cache {
    pub async fn new(config: L2Config) -> Result<Self> {
        let db = sled::open(&config.path)?;
        
        // Configure compression
        let compression = match config.compression {
            CompressionType::None => CompressionStrategy::None,
            CompressionType::Lz4 => CompressionStrategy::Lz4(lz4::EncoderBuilder::new()),
            CompressionType::Zstd => CompressionStrategy::Zstd(zstd::Encoder::new(3)?),
        };
        
        Ok(Self {
            db,
            compression,
            max_size: config.max_size,
        })
    }
    
    pub async fn get(&self, key: &CacheKey) -> Result<Option<CacheValue>> {
        let key_bytes = bincode::serialize(key)?;
        
        if let Some(compressed) = self.db.get(key_bytes)? {
            let decompressed = self.compression.decompress(&compressed)?;
            let value: CacheValue = bincode::deserialize(&decompressed)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    
    pub async fn put(&self, key: CacheKey, value: CacheValue) -> Result<()> {
        // Check size limits
        if self.db.size_on_disk()? > self.max_size {
            self.evict_lru().await?;
        }
        
        let key_bytes = bincode::serialize(&key)?;
        let value_bytes = bincode::serialize(&value)?;
        let compressed = self.compression.compress(&value_bytes)?;
        
        self.db.insert(key_bytes, compressed)?;
        self.db.flush_async().await?;
        
        Ok(())
    }
    
    async fn evict_lru(&self) -> Result<()> {
        // Simple LRU eviction
        let target_size = self.max_size * 80 / 100; // Evict to 80% capacity
        
        while self.db.size_on_disk()? > target_size {
            // Get oldest entry (sled maintains insertion order)
            if let Some(Ok((key, _))) = self.db.iter().next() {
                self.db.remove(key)?;
            } else {
                break;
            }
        }
        
        Ok(())
    }
}
```

### 2. Compression Strategy
```rust
pub enum CompressionStrategy {
    None,
    Lz4(lz4::EncoderBuilder),
    Zstd(zstd::Encoder<'static>),
}

impl CompressionStrategy {
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::None => Ok(data.to_vec()),
            Self::Lz4(encoder) => {
                let mut compressed = Vec::new();
                encoder.build(&mut compressed)?.write_all(data)?;
                Ok(compressed)
            }
            Self::Zstd(encoder) => {
                Ok(encoder.compress(data)?)
            }
        }
    }
    
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::None => Ok(data.to_vec()),
            Self::Lz4(_) => {
                Ok(lz4::decompress(data)?)
            }
            Self::Zstd(_) => {
                Ok(zstd::decode_all(data)?)
            }
        }
    }
}
```

## Cache Coordination

### 1. Multi-Level Coordinator
```rust
pub struct CacheCoordinator {
    l1: Arc<L1Cache>,
    l2: Arc<L2Cache>,
    l3: Option<Arc<L3Cache>>,
    promotion_policy: PromotionPolicy,
}

impl CacheCoordinator {
    pub async fn get(&self, key: &CacheKey) -> Option<CacheValue> {
        // Try L1 first
        if let Some(value) = self.l1.get(key).await {
            return Some(value);
        }
        
        // Try L2
        if let Ok(Some(value)) = self.l2.get(key).await {
            // Promote to L1 if hot
            if self.promotion_policy.should_promote_to_l1(key, &value) {
                self.l1.put(key.clone(), value.clone()).await;
            }
            return Some(value);
        }
        
        // Try L3 if available
        if let Some(l3) = &self.l3 {
            if let Ok(Some(value)) = l3.get(key).await {
                // Promote through levels
                if self.promotion_policy.should_promote_to_l2(key, &value) {
                    let _ = self.l2.put(key.clone(), value.clone()).await;
                }
                if self.promotion_policy.should_promote_to_l1(key, &value) {
                    self.l1.put(key.clone(), value.clone()).await;
                }
                return Some(value);
            }
        }
        
        None
    }
    
    pub async fn put(&self, key: CacheKey, value: CacheValue) {
        // Determine which cache levels to write to
        let levels = self.promotion_policy.determine_levels(&key, &value);
        
        if levels.contains(&CacheLevel::L1) {
            self.l1.put(key.clone(), value.clone()).await;
        }
        
        if levels.contains(&CacheLevel::L2) {
            let _ = self.l2.put(key.clone(), value.clone()).await;
        }
        
        if levels.contains(&CacheLevel::L3) {
            if let Some(l3) = &self.l3 {
                let _ = l3.put(key.clone(), value.clone()).await;
            }
        }
    }
}
```

### 2. Promotion Policy
```rust
pub struct PromotionPolicy {
    l1_threshold: f64,
    l2_threshold: f64,
    access_history: Arc<AccessHistory>,
}

impl PromotionPolicy {
    pub fn should_promote_to_l1(&self, key: &CacheKey, value: &CacheValue) -> bool {
        let score = self.calculate_score(key, value);
        score > self.l1_threshold
    }
    
    pub fn should_promote_to_l2(&self, key: &CacheKey, value: &CacheValue) -> bool {
        let score = self.calculate_score(key, value);
        score > self.l2_threshold
    }
    
    fn calculate_score(&self, key: &CacheKey, value: &CacheValue) -> f64 {
        let frequency = self.access_history.frequency(key);
        let recency = self.access_history.recency(key);
        let size = value.size();
        
        // LRFU (Least Recently/Frequently Used) scoring
        let lambda = 0.5; // Balance between recency and frequency
        let recency_score = (-lambda * recency.as_secs_f64()).exp();
        let frequency_score = frequency as f64;
        
        // Size penalty
        let size_penalty = 1.0 / (1.0 + (size as f64 / 1024.0).ln());
        
        (recency_score + frequency_score) * size_penalty
    }
}
```

## Specialized Caches

### 1. Query Result Cache
```rust
pub struct QueryCache {
    cache: Arc<L1Cache>,
    query_hasher: QueryHasher,
}

impl QueryCache {
    pub async fn get_or_compute<F, Fut>(
        &self,
        query: &str,
        compute: F,
    ) -> Result<QueryResult>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<QueryResult>>,
    {
        let key = self.query_hasher.hash(query);
        
        if let Some(cached) = self.cache.get(&key).await {
            if let Ok(result) = cached.try_into::<QueryResult>() {
                return Ok(result);
            }
        }
        
        let result = compute().await?;
        self.cache.put(key, CacheValue::from(result.clone())).await;
        
        Ok(result)
    }
}
```

### 2. Embedding Cache
```rust
pub struct EmbeddingCache {
    cache: DashMap<String, Arc<Vec<f32>>>,
    max_entries: usize,
}

impl EmbeddingCache {
    pub async fn get_or_embed<F, Fut>(
        &self,
        text: &str,
        embed: F,
    ) -> Result<Arc<Vec<f32>>>
    where
        F: FnOnce(&str) -> Fut,
        Fut: Future<Output = Result<Vec<f32>>>,
    {
        // Check cache
        if let Some(embedding) = self.cache.get(text) {
            return Ok(embedding.clone());
        }
        
        // Compute embedding
        let embedding = Arc::new(embed(text).await?);
        
        // Cache with size limit
        if self.cache.len() < self.max_entries {
            self.cache.insert(text.to_string(), embedding.clone());
        } else {
            // Random eviction for simplicity
            if rand::random::<f32>() < 0.1 {
                let key = self.cache.iter()
                    .next()
                    .map(|entry| entry.key().clone());
                    
                if let Some(key) = key {
                    self.cache.remove(&key);
                    self.cache.insert(text.to_string(), embedding.clone());
                }
            }
        }
        
        Ok(embedding)
    }
}
```

## Cache Warming

### 1. Predictive Cache Warming
```rust
pub struct CacheWarmer {
    coordinator: Arc<CacheCoordinator>,
    predictor: AccessPredictor,
}

impl CacheWarmer {
    pub async fn warm_cache(&self) {
        let predictions = self.predictor.predict_next_accesses();
        
        for (key, probability) in predictions {
            if probability > 0.7 {
                // Pre-fetch high probability items
                if let Some(value) = self.fetch_value(&key).await {
                    self.coordinator.put(key, value).await;
                }
            }
        }
    }
}

pub struct AccessPredictor {
    markov_chain: MarkovChain<CacheKey>,
    time_series: TimeSeries,
}

impl AccessPredictor {
    pub fn predict_next_accesses(&self) -> Vec<(CacheKey, f64)> {
        let current_pattern = self.time_series.current_pattern();
        self.markov_chain.predict(current_pattern)
    }
}
```

## Memory Profile
- **L1 Cache**: 1MB (configurable)
- **L2 Cache metadata**: 500KB
- **Bloom filters**: 100KB
- **Access counters**: 200KB
- **Coordination overhead**: 200KB
- **Total**: ~2MB in-memory + disk usage
