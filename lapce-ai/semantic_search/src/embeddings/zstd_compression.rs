// ZSTD Compression for Embeddings - Zero-loss, high-performance compression
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::sync::Arc;
use zstd::stream::{decode_all, encode_all};

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub compression_level: i32,     // 1-22, higher = better compression
    pub enable_dictionary: bool,    // Use dictionary for better compression
    pub enable_checksum: bool,      // Verify integrity
    pub chunk_size: usize,          // Batch compression chunk size
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            compression_level: 3,    // Fast compression
            enable_dictionary: true,
            enable_checksum: true,
            chunk_size: 100,        // 100 embeddings per batch
        }
    }
}

/// Compressed embedding with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedEmbedding {
    pub id: String,
    pub compressed_data: Vec<u8>,
    pub original_size: usize,
    pub compressed_size: usize,
    pub dimension: usize,
    pub checksum: u32,
    pub compression_ratio: f32,
}

impl CompressedEmbedding {
    /// Calculate compression ratio
    pub fn ratio(&self) -> f32 {
        self.original_size as f32 / self.compressed_size as f32
    }
    
    /// Get space savings percentage
    pub fn space_saved(&self) -> f32 {
        (1.0 - (self.compressed_size as f32 / self.original_size as f32)) * 100.0
    }
}

/// ZSTD compression handler for embeddings
pub struct ZstdCompressor {
    config: CompressionConfig,
    dictionary: Option<Vec<u8>>,
    stats: CompressionStats,
}

#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    pub total_original_bytes: usize,
    pub total_compressed_bytes: usize,
    pub embeddings_compressed: usize,
    pub average_ratio: f32,
    pub best_ratio: f32,
    pub worst_ratio: f32,
}

impl ZstdCompressor {
    /// Create new compressor with config
    pub fn new(config: CompressionConfig) -> Self {
        Self {
            config,
            dictionary: None,
            stats: CompressionStats::default(),
        }
    }
    
    /// Train dictionary from sample embeddings for better compression
    pub fn train_dictionary(&mut self, samples: &[Vec<f32>]) -> Result<()> {
        if !self.config.enable_dictionary || samples.is_empty() {
            return Ok(());
        }
        
        // Convert each sample to bytes separately for proper training
        let mut training_samples = Vec::new();
        for sample in samples.iter().take(1000) {  // Use up to 1000 samples
            let bytes = self.embedding_to_bytes(sample)?;
            training_samples.push(bytes);
        }
        
        // Need enough data for dictionary training
        if training_samples.is_empty() || training_samples[0].len() < 100 {
            // Not enough data for dictionary, skip training
            return Ok(());
        }
        
        // Train dictionary with proper samples format
        let dict_size = std::cmp::min(16 * 1024, training_samples.len() * training_samples[0].len() / 10);
        let samples_refs: Vec<&[u8]> = training_samples.iter().map(|v| v.as_slice()).collect();
        
        let dictionary = zstd::dict::from_samples(&samples_refs, dict_size)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to train compression dictionary: {}", e),
            })?;
        
        self.dictionary = Some(dictionary);
        Ok(())
    }
    
    /// Compress single embedding
    pub fn compress_embedding(&mut self, embedding: &[f32], id: &str) -> Result<CompressedEmbedding> {
        // Convert to bytes
        let original_bytes = self.embedding_to_bytes(embedding)?;
        let original_size = original_bytes.len();
        
        // Compress with ZSTD
        let compressed = if let Some(ref dict) = self.dictionary {
            // Use dictionary compression
            let mut encoder = zstd::stream::Encoder::with_dictionary(
                Vec::new(),
                self.config.compression_level,
                dict,
            ).map_err(|e| Error::Runtime {
                message: format!("Failed to create encoder: {}", e),
            })?;
            
            encoder.write_all(&original_bytes).map_err(|e| Error::Runtime {
                message: format!("Failed to write to encoder: {}", e),
            })?;
            
            encoder.finish().map_err(|e| Error::Runtime {
                message: format!("Failed to finish compression: {}", e),
            })?
        } else {
            // Standard compression
            encode_all(&original_bytes[..], self.config.compression_level)
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to compress embedding: {}", e),
                })?
        };
        
        let compressed_size = compressed.len();
        let compression_ratio = original_size as f32 / compressed_size as f32;
        
        // Calculate checksum if enabled
        let checksum = if self.config.enable_checksum {
            crc32fast::hash(&compressed)
        } else {
            0
        };
        
        // Update stats
        self.stats.total_original_bytes += original_size;
        self.stats.total_compressed_bytes += compressed_size;
        self.stats.embeddings_compressed += 1;
        self.stats.average_ratio = self.stats.total_original_bytes as f32 
            / self.stats.total_compressed_bytes as f32;
        self.stats.best_ratio = self.stats.best_ratio.max(compression_ratio);
        self.stats.worst_ratio = if self.stats.worst_ratio == 0.0 {
            compression_ratio
        } else {
            self.stats.worst_ratio.min(compression_ratio)
        };
        
        Ok(CompressedEmbedding {
            id: id.to_string(),
            compressed_data: compressed,
            original_size,
            compressed_size,
            dimension: embedding.len(),
            checksum,
            compression_ratio,
        })
    }
    
    /// Decompress embedding
    pub fn decompress_embedding(&self, compressed: &CompressedEmbedding) -> Result<Vec<f32>> {
        // Verify checksum if enabled
        if self.config.enable_checksum && compressed.checksum != 0 {
            let calculated = crc32fast::hash(&compressed.compressed_data);
            if calculated != compressed.checksum {
                return Err(Error::Runtime {
                    message: format!(
                        "Checksum mismatch: expected {}, got {}",
                        compressed.checksum, calculated
                    ),
                });
            }
        }
        
        // Decompress
        let decompressed = if let Some(ref dict) = self.dictionary {
            // Use dictionary decompression
            let mut decoder = zstd::stream::Decoder::with_dictionary(
                &compressed.compressed_data[..],
                dict,
            ).map_err(|e| Error::Runtime {
                message: format!("Failed to create decoder: {}", e),
            })?;
            
            let mut output = Vec::new();
            decoder.read_to_end(&mut output).map_err(|e| Error::Runtime {
                message: format!("Failed to decompress: {}", e),
            })?;
            output
        } else {
            // Standard decompression
            decode_all(&compressed.compressed_data[..])
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to decompress embedding: {}", e),
                })?
        };
        
        // Convert bytes back to f32 vector
        self.bytes_to_embedding(&decompressed, compressed.dimension)
    }
    
    /// Decompress embedding directly from borrowed bytes (Phase 4 optimization)
    /// Avoids creating CompressedEmbedding struct and copying data
    pub fn decompress_from_slice(&self, compressed_data: &[u8], dimension: usize) -> Result<Vec<f32>> {
        // Decompress directly from borrowed slice
        let decompressed = if let Some(ref dict) = self.dictionary {
            // Use dictionary decompression
            let mut decoder = zstd::stream::Decoder::with_dictionary(
                compressed_data,
                dict,
            ).map_err(|e| Error::Runtime {
                message: format!("Failed to create decoder: {}", e),
            })?;
            
            let mut output = Vec::new();
            decoder.read_to_end(&mut output).map_err(|e| Error::Runtime {
                message: format!("Failed to decompress: {}", e),
            })?;
            output
        } else {
            // Standard decompression
            decode_all(compressed_data)
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to decompress embedding: {}", e),
                })?
        };
        
        // Convert bytes back to f32 vector
        self.bytes_to_embedding(&decompressed, dimension)
    }
    
    /// Batch compress embeddings
    pub fn batch_compress(
        &mut self,
        embeddings: Vec<Vec<f32>>,
        ids: Vec<String>,
    ) -> Result<Vec<CompressedEmbedding>> {
        if embeddings.len() != ids.len() {
            return Err(Error::Runtime {
                message: "Embeddings and IDs count mismatch".to_string(),
            });
        }
        
        let mut compressed = Vec::with_capacity(embeddings.len());
        
        for (embedding, id) in embeddings.iter().zip(ids.iter()) {
            compressed.push(self.compress_embedding(embedding, id)?);
        }
        
        Ok(compressed)
    }
    
    /// Batch decompress embeddings
    pub fn batch_decompress(
        &self,
        compressed: Vec<CompressedEmbedding>,
    ) -> Result<Vec<Vec<f32>>> {
        let mut decompressed = Vec::with_capacity(compressed.len());
        
        for comp in compressed.iter() {
            decompressed.push(self.decompress_embedding(comp)?);
        }
        
        Ok(decompressed)
    }
    
    /// Convert embedding to bytes
    fn embedding_to_bytes(&self, embedding: &[f32]) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(embedding.len() * 4);
        for value in embedding {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Ok(bytes)
    }
    
    /// Convert bytes to embedding
    fn bytes_to_embedding(&self, bytes: &[u8], dimension: usize) -> Result<Vec<f32>> {
        if bytes.len() != dimension * 4 {
            return Err(Error::Runtime {
                message: format!(
                    "Invalid byte length: expected {}, got {}",
                    dimension * 4,
                    bytes.len()
                ),
            });
        }
        
        let mut embedding = Vec::with_capacity(dimension);
        for i in 0..dimension {
            let start = i * 4;
            let value = f32::from_le_bytes([
                bytes[start],
                bytes[start + 1],
                bytes[start + 2],
                bytes[start + 3],
            ]);
            embedding.push(value);
        }
        
        Ok(embedding)
    }
    
    /// Get compression statistics
    pub fn get_stats(&self) -> &CompressionStats {
        &self.stats
    }
    
    /// Verify bit-perfect compression (for testing)
    pub fn verify_bit_perfect(&mut self, embedding: &[f32]) -> Result<bool> {
        let compressed = self.compress_embedding(embedding, "test")?;
        let decompressed = self.decompress_embedding(&compressed)?;
        
        if embedding.len() != decompressed.len() {
            return Ok(false);
        }
        
        for (original, decompressed) in embedding.iter().zip(decompressed.iter()) {
            if (original - decompressed).abs() > f32::EPSILON {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compression_bit_perfect() {
        let mut compressor = ZstdCompressor::new(CompressionConfig::default());
        
        // Test with typical embedding
        let mut embedding = Vec::with_capacity(1536);
        for _ in 0..1536 {
            embedding.extend_from_slice(&[0.1, -0.5, 0.3, 0.7, -0.2]);
        }
        embedding.truncate(1536);  // 1536-dim like AWS Titan
        
        assert!(compressor.verify_bit_perfect(&embedding).unwrap());
    }
    
    #[test]
    fn test_compression_ratio() {
        let mut compressor = ZstdCompressor::new(CompressionConfig::default());
        
        // Create repetitive embedding (should compress well)
        let mut embedding = Vec::with_capacity(1536);
        for i in 0..1536 {
            embedding.push((i % 10) as f32 / 10.0);
        }
        
        let compressed = compressor.compress_embedding(&embedding, "test").unwrap();
        
        println!("Original size: {} bytes", compressed.original_size);
        println!("Compressed size: {} bytes", compressed.compressed_size);
        println!("Compression ratio: {:.2}x", compressed.compression_ratio);
        println!("Space saved: {:.1}%", compressed.space_saved());
        
        assert!(compressed.compression_ratio > 1.0);
    }
    
    #[test]
    fn test_batch_operations() {
        let mut compressor = ZstdCompressor::new(CompressionConfig::default());
        
        let embeddings = vec![
            vec![0.1; 384],
            vec![0.2; 384],
            vec![0.3; 384],
        ];
        
        let ids = vec!["id1".to_string(), "id2".to_string(), "id3".to_string()];
        
        let compressed = compressor.batch_compress(embeddings.clone(), ids).unwrap();
        let decompressed = compressor.batch_decompress(compressed).unwrap();
        
        assert_eq!(embeddings.len(), decompressed.len());
        for (original, decompressed) in embeddings.iter().zip(decompressed.iter()) {
            assert_eq!(original.len(), decompressed.len());
        }
    }
}
