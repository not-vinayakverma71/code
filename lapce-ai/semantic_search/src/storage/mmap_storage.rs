// Memory-mapped storage for compressed embeddings with zero-copy access
use crate::error::{Error, Result};
use crate::embeddings::zstd_compression::{CompressedEmbedding, ZstdCompressor};
use memmap2::{Mmap, MmapMut, MmapOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Metadata for stored embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    pub id: String,
    pub offset: u64,
    pub size: u64,
    pub dimension: usize,
    pub compressed: bool,
}

/// Index manager for tracking embedding locations
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexManager {
    embeddings: HashMap<String, EmbeddingMetadata>,
    next_offset: u64,
    total_size: u64,
    embedding_count: usize,
}

impl IndexManager {
    pub fn new() -> Self {
        Self {
            embeddings: HashMap::new(),
            next_offset: 0,
            total_size: 0,
            embedding_count: 0,
        }
    }
    
    pub fn add_embedding(&mut self, metadata: EmbeddingMetadata) {
        self.next_offset = metadata.offset + metadata.size;
        self.total_size += metadata.size;
        self.embedding_count += 1;
        self.embeddings.insert(metadata.id.clone(), metadata);
    }
    
    pub fn get_metadata(&self, id: &str) -> Option<&EmbeddingMetadata> {
        self.embeddings.get(id)
    }
    
    pub fn remove_embedding(&mut self, id: &str) -> Option<EmbeddingMetadata> {
        if let Some(metadata) = self.embeddings.remove(id) {
            self.embedding_count -= 1;
            self.total_size -= metadata.size;
            Some(metadata)
        } else {
            None
        }
    }
}

/// Memory-mapped storage for embeddings
pub struct MmapStorage {
    data_path: PathBuf,
    index_path: PathBuf,
    data_file: Arc<RwLock<File>>,
    mmap: Arc<RwLock<Option<Mmap>>>,
    index: Arc<RwLock<IndexManager>>,
    compressor: Arc<RwLock<ZstdCompressor>>,
    max_file_size: u64,
}

impl MmapStorage {
    /// Create new memory-mapped storage
    pub fn new(base_path: &Path, max_file_size: u64) -> Result<Self> {
        let data_path = base_path.join("embeddings.dat");
        let index_path = base_path.join("index.json");
        
        // Create or open data file
        let data_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&data_path)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to open data file: {}", e),
            })?;
        
        // Load or create index
        let index = if index_path.exists() {
            let index_data = std::fs::read_to_string(&index_path)
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to read index: {}", e),
                })?;
            serde_json::from_str(&index_data)
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to parse index: {}", e),
                })?
        } else {
            IndexManager::new()
        };
        
        // Create memory map if file has content
        let file_len = data_file.metadata()
            .map_err(|e| Error::Runtime {
                message: format!("Failed to get file metadata: {}", e),
            })?
            .len();
        
        let mmap = if file_len > 0 {
            let mmap = unsafe {
                MmapOptions::new()
                    .map(&data_file)
                    .map_err(|e| Error::Runtime {
                        message: format!("Failed to create memory map: {}", e),
                    })?
            };
            Some(mmap)
        } else {
            None
        };
        
        let compressor = ZstdCompressor::new(Default::default());
        
        Ok(Self {
            data_path,
            index_path,
            data_file: Arc::new(RwLock::new(data_file)),
            mmap: Arc::new(RwLock::new(mmap)),
            index: Arc::new(RwLock::new(index)),
            compressor: Arc::new(RwLock::new(compressor)),
            max_file_size,
        })
    }
    
    /// Store compressed embedding
    pub fn store_compressed(&self, compressed: &CompressedEmbedding) -> Result<()> {
        let mut index = self.index.write().unwrap();
        let mut data_file = self.data_file.write().unwrap();
        
        // Seek to end of file
        let offset = data_file.seek(SeekFrom::End(0))
            .map_err(|e| Error::Runtime {
                message: format!("Failed to seek file: {}", e),
            })?;
        
        // Check file size limit
        if offset + compressed.compressed_data.len() as u64 > self.max_file_size {
            return Err(Error::Runtime {
                message: format!("File size limit exceeded: {} bytes", self.max_file_size),
            });
        }
        
        // Write compressed data
        data_file.write_all(&compressed.compressed_data)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to write data: {}", e),
            })?;
        
        // Update index
        let metadata = EmbeddingMetadata {
            id: compressed.id.clone(),
            offset,
            size: compressed.compressed_data.len() as u64,
            dimension: compressed.dimension,
            compressed: true,
        };
        
        index.add_embedding(metadata);
        
        // Recreate memory map
        self.update_mmap()?;
        
        // Save index
        self.save_index_internal(&index)?;
        
        Ok(())
    }
    
    /// Store raw embedding (will compress first)
    pub fn store_embedding(&self, id: &str, embedding: &[f32]) -> Result<()> {
        let mut compressor = self.compressor.write().unwrap();
        let compressed = compressor.compress_embedding(embedding, id)?;
        drop(compressor);
        
        self.store_compressed(&compressed)
    }
    
    /// Retrieve embedding by ID (zero-copy from mmap)
    pub fn get_embedding(&self, id: &str) -> Result<Vec<f32>> {
        let index = self.index.read().unwrap();
        let metadata = index.get_metadata(id)
            .ok_or_else(|| Error::Runtime {
                message: format!("Embedding not found: {}", id),
            })?
            .clone();
        drop(index);
        
        let mmap_guard = self.mmap.read().unwrap();
        let mmap = mmap_guard.as_ref()
            .ok_or_else(|| Error::Runtime {
                message: "No memory map available".to_string(),
            })?;
        
        // Zero-copy slice from memory map
        let start = metadata.offset as usize;
        let end = start + metadata.size as usize;
        
        if end > mmap.len() {
            return Err(Error::Runtime {
                message: format!("Invalid offset/size in metadata: {} + {} > {}", 
                    start, metadata.size, mmap.len()),
            });
        }
        
        let compressed_data = &mmap[start..end];
        
        // Phase 4 optimization: Decompress directly from mmap slice without copying
        let compressor = self.compressor.read().unwrap();
        compressor.decompress_from_slice(compressed_data, metadata.dimension)
    }
    
    /// Get embedding without decompression (returns compressed data)
    pub fn get_compressed(&self, id: &str) -> Result<CompressedEmbedding> {
        let index = self.index.read().unwrap();
        let metadata = index.get_metadata(id)
            .ok_or_else(|| Error::Runtime {
                message: format!("Embedding not found: {}", id),
            })?
            .clone();
        drop(index);
        
        let mmap_guard = self.mmap.read().unwrap();
        let mmap = mmap_guard.as_ref()
            .ok_or_else(|| Error::Runtime {
                message: "No memory map available".to_string(),
            })?;
        
        let start = metadata.offset as usize;
        let end = start + metadata.size as usize;
        
        let compressed_data = mmap[start..end].to_vec();
        
        Ok(CompressedEmbedding {
            id: id.to_string(),
            compressed_data,
            original_size: metadata.dimension * 4,
            compressed_size: metadata.size as usize,
            dimension: metadata.dimension,
            checksum: 0,
            compression_ratio: (metadata.dimension * 4) as f32 / metadata.size as f32,
        })
    }
    
    /// Batch store embeddings
    pub fn batch_store(&self, embeddings: Vec<(String, Vec<f32>)>) -> Result<()> {
        for (id, embedding) in embeddings {
            self.store_embedding(&id, &embedding)?;
        }
        Ok(())
    }
    
    /// Batch retrieve embeddings
    pub fn batch_get(&self, ids: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(ids.len());
        for id in ids {
            results.push(self.get_embedding(id)?);
        }
        Ok(results)
    }
    
    /// Update memory map after file changes
    fn update_mmap(&self) -> Result<()> {
        let data_file = self.data_file.read().unwrap();
        let file_len = data_file.metadata()
            .map_err(|e| Error::Runtime {
                message: format!("Failed to get file metadata: {}", e),
            })?
            .len();
        
        if file_len > 0 {
            let new_mmap = unsafe {
                MmapOptions::new()
                    .map(&*data_file)
                    .map_err(|e| Error::Runtime {
                        message: format!("Failed to create memory map: {}", e),
                    })?
            };
            
            let mut mmap = self.mmap.write().unwrap();
            *mmap = Some(new_mmap);
        }
        
        Ok(())
    }
    
    /// Save index to disk
    fn save_index_internal(&self, index: &IndexManager) -> Result<()> {
        let index_data = serde_json::to_string_pretty(index)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to serialize index: {}", e),
            })?;
        
        std::fs::write(&self.index_path, index_data)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to write index: {}", e),
            })?;
        
        Ok(())
    }
    
    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        let index = self.index.read().unwrap();
        let mmap = self.mmap.read().unwrap();
        
        StorageStats {
            embedding_count: index.embedding_count,
            total_size_bytes: index.total_size,
            mapped_size: mmap.as_ref().map(|m| m.len() as u64).unwrap_or(0),
            average_size: if index.embedding_count > 0 {
                index.total_size / index.embedding_count as u64
            } else {
                0
            },
        }
    }
    
    /// Check if embedding exists
    pub fn contains(&self, id: &str) -> bool {
        let index = self.index.read().unwrap();
        index.embeddings.contains_key(id)
    }
    
    /// Remove embedding
    pub fn remove(&self, id: &str) -> Result<()> {
        let mut index = self.index.write().unwrap();
        index.remove_embedding(id);
        self.save_index_internal(&index)?;
        Ok(())
    }
    
    /// Clear all embeddings
    pub fn clear(&self) -> Result<()> {
        let mut index = self.index.write().unwrap();
        index.embeddings.clear();
        index.next_offset = 0;
        index.total_size = 0;
        index.embedding_count = 0;
        
        // Truncate data file
        let mut data_file = self.data_file.write().unwrap();
        data_file.set_len(0)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to truncate file: {}", e),
            })?;
        
        // Clear mmap
        let mut mmap = self.mmap.write().unwrap();
        *mmap = None;
        
        self.save_index_internal(&index)?;
        Ok(())
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub embedding_count: usize,
    pub total_size_bytes: u64,
    pub mapped_size: u64,
    pub average_size: u64,
}

/// Concurrent access wrapper for thread-safe operations
pub struct ConcurrentMmapStorage {
    storage: Arc<MmapStorage>,
}

impl ConcurrentMmapStorage {
    pub fn new(base_path: &Path, max_file_size: u64) -> Result<Self> {
        Ok(Self {
            storage: Arc::new(MmapStorage::new(base_path, max_file_size)?),
        })
    }
    
    pub fn store(&self, id: &str, embedding: &[f32]) -> Result<()> {
        self.storage.store_embedding(id, embedding)
    }
    
    pub fn get(&self, id: &str) -> Result<Vec<f32>> {
        self.storage.get_embedding(id)
    }
    
    pub fn batch_store(&self, embeddings: Vec<(String, Vec<f32>)>) -> Result<()> {
        self.storage.batch_store(embeddings)
    }
    
    pub fn batch_get(&self, ids: &[String]) -> Result<Vec<Vec<f32>>> {
        self.storage.batch_get(ids)
    }
    
    pub fn contains(&self, id: &str) -> bool {
        self.storage.contains(id)
    }
    
    pub fn get_stats(&self) -> StorageStats {
        self.storage.get_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_mmap_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = MmapStorage::new(temp_dir.path(), 100 * 1024 * 1024).unwrap();
        
        // Store embedding
        let embedding = vec![0.1, 0.2, 0.3; 384];
        storage.store_embedding("test1", &embedding).unwrap();
        
        // Retrieve embedding
        let retrieved = storage.get_embedding("test1").unwrap();
        assert_eq!(embedding.len(), retrieved.len());
        
        // Verify values
        for (original, retrieved) in embedding.iter().zip(retrieved.iter()) {
            assert!((original - retrieved).abs() < f32::EPSILON);
        }
    }
    
    #[test]
    fn test_concurrent_access() {
        use std::thread;
        
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(ConcurrentMmapStorage::new(
            temp_dir.path(),
            100 * 1024 * 1024
        ).unwrap());
        
        let mut handles = vec![];
        
        // Spawn multiple threads to store embeddings
        for i in 0..10 {
            let storage_clone = storage.clone();
            let handle = thread::spawn(move || {
                let embedding = vec![i as f32 / 10.0; 384];
                storage_clone.store(&format!("embedding_{}", i), &embedding).unwrap();
            });
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify all embeddings
        for i in 0..10 {
            assert!(storage.contains(&format!("embedding_{}", i)));
        }
        
        let stats = storage.get_stats();
        assert_eq!(stats.embedding_count, 10);
    }
}
