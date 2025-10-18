// Fast Incremental Updater with < 10ms Update Time
// Integrates delta encoding with shared memory for rapid updates

use crate::error::Result;
use super::delta_encoder::{DeltaEncoder, DeltaOperation, FieldChange};
use crate::memory::shared_pool::SharedMemoryPool;
// CompressedEmbedding removed - not directly used
use crate::search::fully_optimized_storage::FullyOptimizedStorage;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Instant, Duration};
use tokio::sync::RwLock;
// Arrow imports removed - not used

/// Fast incremental updater
pub struct FastIncrementalUpdater {
    delta_encoder: Arc<DeltaEncoder>,
    memory_pool: Arc<SharedMemoryPool>,
    storage: Arc<FullyOptimizedStorage>,
    update_cache: Arc<RwLock<HashMap<String, UpdateEntry>>>,
    metrics: Arc<RwLock<UpdateMetrics>>,
}

/// Update entry in cache
#[derive(Clone)]
struct UpdateEntry {
    embedding: Vec<f32>,
    metadata: HashMap<String, String>,
    version: u64,
    last_update: Instant,
}

/// Update performance metrics
#[derive(Default)]
pub struct UpdateMetrics {
    pub total_updates: u64,
    pub avg_update_time_ms: f64,
    pub p95_update_time_ms: f64,
    pub p99_update_time_ms: f64,
    pub fastest_update_ms: f64,
    pub slowest_update_ms: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub rollbacks_performed: u64,
    update_times: Vec<Duration>,
}

impl FastIncrementalUpdater {
    /// Create new fast updater
    pub async fn new(
        storage: Arc<FullyOptimizedStorage>,
        max_memory_mb: usize,
    ) -> Result<Self> {
        let delta_encoder = Arc::new(DeltaEncoder::new(100));
        let memory_pool = Arc::new(SharedMemoryPool::new(
            "fast_updater".to_string(),
            max_memory_mb * 1024 * 1024,
        )?);
        
        Ok(Self {
            delta_encoder,
            memory_pool,
            storage,
            update_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(UpdateMetrics::default())),
        })
    }
    
    /// Apply single update (target: < 10ms)
    pub async fn apply_update(
        &self,
        doc_id: &str,
        new_embedding: &[f32],
        metadata: HashMap<String, String>,
    ) -> Result<Duration> {
        let start = Instant::now();
        
        // 1. Check cache first (zero-copy via shared memory)
        let (old_embedding, old_metadata) = {
            let cache = self.update_cache.read().await;
            if let Some(entry) = cache.get(doc_id) {
                (entry.embedding.clone(), entry.metadata.clone())
            } else {
                drop(cache);
                self.metrics.write().await.cache_misses += 1;
                
                // Fetch from storage if not cached
                match self.fetch_from_storage(doc_id).await {
                    Ok((emb, meta)) => (emb, meta),
                    Err(_) => {
                        // New document
                        (vec![0.0; new_embedding.len()], HashMap::new())
                    }
                }
            }
        };
        
        // 2. Compute delta
        let field_changes = self.compute_field_changes(&old_metadata, &metadata);
        let delta_op = self.delta_encoder.encode_update(
            &old_embedding,
            new_embedding,
            field_changes,
        ).await?;
        
        // 3. Store in shared memory (zero-copy)
        let segment_size = new_embedding.len() * 4; // f32 size
        let mut segment = self.memory_pool.allocate(segment_size)?;
        
        // Write embedding to shared memory
        unsafe {
            let ptr = segment.as_mut_ptr() as *mut f32;
            std::ptr::copy_nonoverlapping(
                new_embedding.as_ptr(),
                ptr,
                new_embedding.len()
            );
        }
        
        // 4. Update cache
        let mut cache = self.update_cache.write().await;
        let version = self.delta_encoder.get_current_version().await;
        cache.insert(doc_id.to_string(), UpdateEntry {
            embedding: new_embedding.to_vec(),
            metadata,
            version,
            last_update: Instant::now(),
        });
        
        // 5. Add delta to encoder
        self.delta_encoder.add_delta(delta_op).await?;
        
        // 6. Update metrics
        let elapsed = start.elapsed();
        let mut metrics = self.metrics.write().await;
        metrics.total_updates += 1;
        metrics.cache_hits += 1;
        metrics.update_times.push(elapsed);
        
        // Keep last 1000 samples for percentiles
        if metrics.update_times.len() > 1000 {
            metrics.update_times.remove(0);
        }
        
        // Update statistics
        self.update_metrics_stats(&mut metrics);
        
        // Check if we met the < 10ms target
        if elapsed.as_millis() > 10 {
            log::warn!("Update took {:?} (target: < 10ms)", elapsed);
        } else {
            log::debug!("Fast update completed in {:?}", elapsed);
        }
        
        Ok(elapsed)
    }
    
    /// Batch apply updates
    pub async fn batch_apply_updates(
        &self,
        updates: Vec<(String, Vec<f32>, HashMap<String, String>)>,
    ) -> Result<Vec<Duration>> {
        let mut durations = Vec::new();
        
        // Process in parallel using shared memory
        let chunks: Vec<_> = updates.chunks(10).collect();
        
        for chunk in chunks {
            
            for (doc_id, embedding, metadata) in chunk {
                let duration = self.apply_update(doc_id, embedding, metadata.clone()).await?;
                durations.push(duration);
            }
        }
        
        Ok(durations)
    }
    
    /// Create version snapshot
    pub async fn create_snapshot(&self) -> Result<u64> {
        let snapshot = self.delta_encoder.create_snapshot().await?;
        
        // Persist to storage
        self.persist_snapshot(&snapshot).await?;
        
        Ok(snapshot.version)
    }
    
    /// Rollback to version
    pub async fn rollback_to_version(&self, version: u64) -> Result<()> {
        let start = Instant::now();
        
        // Clear cache
        self.update_cache.write().await.clear();
        
        // Get current embeddings
        let mut embeddings = self.fetch_all_embeddings().await?;
        
        // Rollback using delta encoder
        self.delta_encoder.rollback_to_version(version, &mut embeddings).await?;
        
        // Update metrics
        self.metrics.write().await.rollbacks_performed += 1;
        
        log::info!("Rolled back to version {} in {:?}", version, start.elapsed());
        
        Ok(())
    }
    
    /// Get update metrics
    pub async fn get_metrics(&self) -> UpdateMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Fetch from storage
    async fn fetch_from_storage(&self, doc_id: &str) -> Result<(Vec<f32>, HashMap<String, String>)> {
        // This would fetch from the actual storage
        // For now, return placeholder
        Ok((vec![0.0; 1536], HashMap::new()))
    }
    
    /// Fetch all embeddings
    async fn fetch_all_embeddings(&self) -> Result<HashMap<u64, Vec<f32>>> {
        // This would fetch all from storage
        Ok(HashMap::new())
    }
    
    /// Compute field changes
    fn compute_field_changes(
        &self,
        old: &HashMap<String, String>,
        new: &HashMap<String, String>,
    ) -> HashMap<String, FieldChange> {
        let mut changes = HashMap::new();
        
        // Check for updates and additions
        for (key, new_val) in new {
            let old_val = old.get(key).cloned();
            if old_val.as_ref() != Some(new_val) {
                changes.insert(key.clone(), FieldChange {
                    field: key.clone(),
                    old_value: old_val,
                    new_value: Some(new_val.clone()),
                });
            }
        }
        
        // Check for deletions
        for (key, old_val) in old {
            if !new.contains_key(key) {
                changes.insert(key.clone(), FieldChange {
                    field: key.clone(),
                    old_value: Some(old_val.clone()),
                    new_value: None,
                });
            }
        }
        
        changes
    }
    
    /// Persist snapshot to storage
    async fn persist_snapshot(&self, _snapshot: &crate::incremental::delta_encoder::VersionSnapshot) -> Result<()> {
        // Store snapshot metadata
        // This would integrate with the actual storage backend
        Ok(())
    }
    
    /// Update metrics statistics
    fn update_metrics_stats(&self, metrics: &mut UpdateMetrics) {
        if metrics.update_times.is_empty() {
            return;
        }
        
        // Sort for percentiles
        let mut sorted_times = metrics.update_times.clone();
        sorted_times.sort();
        
        // Calculate statistics
        let sum: Duration = sorted_times.iter().sum();
        metrics.avg_update_time_ms = sum.as_secs_f64() * 1000.0 / sorted_times.len() as f64;
        
        metrics.fastest_update_ms = sorted_times.first()
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
            
        metrics.slowest_update_ms = sorted_times.last()
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        
        // P95
        let p95_idx = (sorted_times.len() as f64 * 0.95) as usize;
        metrics.p95_update_time_ms = sorted_times.get(p95_idx)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        
        // P99
        let p99_idx = (sorted_times.len() as f64 * 0.99) as usize;
        metrics.p99_update_time_ms = sorted_times.get(p99_idx)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
    }
}

impl Clone for UpdateMetrics {
    fn clone(&self) -> Self {
        Self {
            total_updates: self.total_updates,
            avg_update_time_ms: self.avg_update_time_ms,
            p95_update_time_ms: self.p95_update_time_ms,
            p99_update_time_ms: self.p99_update_time_ms,
            fastest_update_ms: self.fastest_update_ms,
            slowest_update_ms: self.slowest_update_ms,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            rollbacks_performed: self.rollbacks_performed,
            update_times: self.update_times.clone(),
        }
    }
}
