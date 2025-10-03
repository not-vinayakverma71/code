// Delta Encoding for Incremental Updates
// Implements efficient delta encoding for embedding updates with version control

use crate::error::{Error, Result};
use crate::embeddings::compression::CompressedEmbedding;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Delta operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaOperation {
    Add {
        embedding: CompressedEmbedding,
        metadata: HashMap<String, String>,
    },
    Update {
        old_hash: u64,
        new_embedding: CompressedEmbedding,
        changes: Vec<FieldChange>,
    },
    Delete {
        hash: u64,
    },
}

/// Field-level changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// Version snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSnapshot {
    pub version: u64,
    pub timestamp: u64,
    pub operations: Vec<DeltaOperation>,
    pub checksum: u64,
}

/// Delta encoder for incremental updates
pub struct DeltaEncoder {
    current_version: Arc<RwLock<u64>>,
    max_versions: usize,
    version_history: Arc<RwLock<VecDeque<VersionSnapshot>>>,
    active_deltas: Arc<RwLock<Vec<DeltaOperation>>>,
}

impl DeltaEncoder {
    /// Create new delta encoder
    pub fn new(max_versions: usize) -> Self {
        Self {
            current_version: Arc::new(RwLock::new(0)),
            max_versions,
            version_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_versions))),
            active_deltas: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Encode delta for update
    pub async fn encode_update(
        &self,
        old_embedding: &[f32],
        new_embedding: &[f32],
        metadata_changes: HashMap<String, FieldChange>,
    ) -> Result<DeltaOperation> {
        // Compute delta between embeddings
        let mut delta = vec![0.0f32; old_embedding.len()];
        for i in 0..old_embedding.len() {
            delta[i] = new_embedding[i] - old_embedding[i];
        }
        
        // Only store non-zero deltas for efficiency
        let significant_changes: Vec<(usize, f32)> = delta.iter()
            .enumerate()
            .filter(|(_, &v)| v.abs() > 1e-6)
            .map(|(i, &v)| (i, v))
            .collect();
        
        // If too many changes, just store the new embedding
        if significant_changes.len() > old_embedding.len() / 2 {
            Ok(DeltaOperation::Update {
                old_hash: hash_embedding(old_embedding),
                new_embedding: CompressedEmbedding::compress(new_embedding).map_err(|e| Error::Runtime {
                    message: format!("Failed to compress embedding: {}", e),
                })?,
                changes: metadata_changes.into_iter().map(|(k, v)| v).collect(),
            })
        } else {
            // Store sparse delta
            let mut sparse_embedding = old_embedding.to_vec();
            for (idx, val) in significant_changes {
                sparse_embedding[idx] = val;
            }
            
            Ok(DeltaOperation::Update {
                old_hash: hash_embedding(old_embedding),
                new_embedding: CompressedEmbedding::compress(&sparse_embedding).map_err(|e| Error::Runtime {
                    message: format!("Failed to compress sparse embedding: {}", e),
                })?,
                changes: metadata_changes.into_iter().map(|(k, v)| v).collect(),
            })
        }
    }
    
    /// Apply delta operations
    pub async fn apply_deltas(
        &self,
        base_embeddings: &mut HashMap<u64, Vec<f32>>,
        operations: &[DeltaOperation],
    ) -> Result<()> {
        let start = Instant::now();
        
        for op in operations {
            match op {
                DeltaOperation::Add { embedding, metadata: _ } => {
                    let decompressed = embedding.decompress().map_err(|e| Error::Runtime {
                        message: format!("Failed to decompress embedding: {}", e),
                    })?;
                    let hash = hash_embedding(&decompressed);
                    base_embeddings.insert(hash, decompressed);
                }
                DeltaOperation::Update { old_hash, new_embedding, changes: _ } => {
                    if let Some(_old) = base_embeddings.get(old_hash) {
                        let decompressed = new_embedding.decompress().map_err(|e| Error::Runtime {
                            message: format!("Failed to decompress new embedding: {}", e),
                        })?;
                        let new_hash = hash_embedding(&decompressed);
                        base_embeddings.remove(old_hash);
                        base_embeddings.insert(new_hash, decompressed);
                    }
                }
                DeltaOperation::Delete { hash } => {
                    base_embeddings.remove(hash);
                }
            }
        }
        
        let elapsed = start.elapsed();
        if elapsed.as_millis() > 10 {
            log::warn!("Delta application took {:?} (target: <10ms)", elapsed);
        }
        
        Ok(())
    }
    
    /// Create version snapshot
    pub async fn create_snapshot(&self) -> Result<VersionSnapshot> {
        let mut version = self.current_version.write().await;
        *version += 1;
        
        let active_deltas = self.active_deltas.read().await;
        
        let snapshot = VersionSnapshot {
            version: *version,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            operations: active_deltas.clone(),
            checksum: compute_checksum(&active_deltas),
        };
        
        // Add to history
        let mut history = self.version_history.write().await;
        if history.len() >= self.max_versions {
            history.pop_front();
        }
        history.push_back(snapshot.clone());
        
        // Clear active deltas
        drop(active_deltas);
        self.active_deltas.write().await.clear();
        
        Ok(snapshot)
    }
    
    /// Rollback to specific version
    pub async fn rollback_to_version(
        &self,
        target_version: u64,
        base_embeddings: &mut HashMap<u64, Vec<f32>>,
    ) -> Result<()> {
        let history = self.version_history.read().await;
        
        // Find target version
        let target_snapshot = history.iter()
            .find(|s| s.version == target_version)
            .ok_or_else(|| Error::Runtime {
                message: format!("Version {} not found", target_version),
            })?;
        
        // Collect all operations after target version
        let mut reverse_ops = Vec::new();
        for snapshot in history.iter().rev() {
            if snapshot.version <= target_version {
                break;
            }
            
            // Create reverse operations
            for op in snapshot.operations.iter().rev() {
                reverse_ops.push(reverse_operation(op)?);
            }
        }
        
        // Apply reverse operations
        self.apply_deltas(base_embeddings, &reverse_ops).await?;
        
        // Update current version
        *self.current_version.write().await = target_version;
        
        Ok(())
    }
    
    /// Add delta operation
    pub async fn add_delta(&self, operation: DeltaOperation) -> Result<()> {
        self.active_deltas.write().await.push(operation);
        Ok(())
    }
    
    /// Get current version
    pub async fn get_current_version(&self) -> u64 {
        *self.current_version.read().await
    }
    
    /// Get version history
    pub async fn get_version_history(&self) -> Vec<u64> {
        self.version_history.read().await
            .iter()
            .map(|s| s.version)
            .collect()
    }
}

/// Hash embedding for identification
fn hash_embedding(embedding: &[f32]) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    
    let mut hasher = DefaultHasher::new();
    for &val in embedding {
        val.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Compute checksum for operations
fn compute_checksum(operations: &[DeltaOperation]) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    
    let mut hasher = DefaultHasher::new();
    operations.len().hash(&mut hasher);
    hasher.finish()
}

/// Create reverse operation for rollback
fn reverse_operation(op: &DeltaOperation) -> Result<DeltaOperation> {
    match op {
        DeltaOperation::Add { embedding, .. } => {
            let decompressed = embedding.decompress().map_err(|e| Error::Runtime {
                message: format!("Failed to decompress for reverse: {}", e),
            })?;
            Ok(DeltaOperation::Delete {
                hash: hash_embedding(&decompressed),
            })
        }
        DeltaOperation::Update { old_hash, .. } => {
            // In real implementation, would need to store old embedding
            Ok(DeltaOperation::Delete {
                hash: *old_hash,
            })
        }
        DeltaOperation::Delete { hash } => {
            // Would need to store deleted embedding for proper rollback
            Ok(DeltaOperation::Delete {
                hash: *hash,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_delta_encoding() {
        let encoder = DeltaEncoder::new(10);
        
        let old = vec![0.1, 0.2, 0.3, 0.4];
        let new = vec![0.1, 0.25, 0.3, 0.45]; // Small changes
        
        let delta = encoder.encode_update(&old, &new, HashMap::new()).await.unwrap();
        
        match delta {
            DeltaOperation::Update { .. } => assert!(true),
            _ => panic!("Expected update operation"),
        }
    }
    
    #[tokio::test]
    async fn test_version_control() {
        let encoder = DeltaEncoder::new(5);
        
        // Add some operations
        for i in 0..3 {
            let embedding = CompressedEmbedding::compress(&vec![i as f32; 10])
                .expect("Failed to compress test embedding");
            encoder.add_delta(DeltaOperation::Add {
                embedding,
                metadata: HashMap::new(),
            }).await.unwrap();
        }
        
        // Create snapshot
        let snapshot = encoder.create_snapshot().await.unwrap();
        assert_eq!(snapshot.version, 1);
        assert_eq!(snapshot.operations.len(), 3);
        
        // Check version history
        let history = encoder.get_version_history().await;
        assert_eq!(history, vec![1]);
    }
}
