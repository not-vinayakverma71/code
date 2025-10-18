// TASK 1: ZSTD Compression Layer for Zero-Loss Embedding Storage
// Goal: 40-60% memory reduction with bit-perfect reconstruction

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use zstd::{Encoder, Decoder};
use std::io::{Read, Write};
use std::mem;

/// Compressed embedding with zero quality loss
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedEmbedding {
    /// Compressed f32 array using ZSTD
    compressed_data: Vec<u8>,
    
    /// Original dimensions for reconstruction
    original_dimensions: usize,
    
    /// Compression ratio for monitoring
    compression_ratio: f32,
    
    /// Checksum for validation
    checksum: u32,
}

impl CompressedEmbedding {
    /// Production-grade LOSSLESS compression using byte-shuffle + ZSTD
    /// - Byte-shuffle groups the same-significance bytes together to expose redundancy
    /// - ZSTD compresses the shuffled stream
    /// - Checksum is computed on the original unshuffled bytes
    pub fn compress(embedding: &[f32]) -> Result<Self> {
        let start = Instant::now();

        // Safety: casting &[f32] to &[u8]
        let raw_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                embedding.as_ptr() as *const u8,
                embedding.len() * mem::size_of::<f32>(),
            )
        };

        // Compute checksum on original bytes
        let checksum = crc32fast::hash(raw_bytes);

        // Byte-shuffle to improve compressibility (width=4 bytes per f32)
        let shuffled = byte_shuffle_4(raw_bytes);

        // ZSTD compression
        let mut encoder = Encoder::new(Vec::new(), 10)?; // good tradeoff; tune as needed
        encoder.include_checksum(true)?;
        encoder.include_contentsize(true)?;
        encoder.write_all(&shuffled)?;
        let compressed_data = encoder.finish()?;

        let original_size = raw_bytes.len();
        let compressed_size = compressed_data.len();
        let compression_ratio = compressed_size as f32 / original_size as f32;

        tracing::debug!(
            "Compressed {} bytes to {} bytes ({:.1}% reduction) in {:?}",
            original_size,
            compressed_size,
            (1.0 - compression_ratio) * 100.0,
            start.elapsed()
        );

        Ok(Self {
            compressed_data,
            original_dimensions: embedding.len(),
            compression_ratio,
            checksum,
        })
    }
    
    /// Decompress to original embedding (BIT-PERFECT LOSSLESS)
    pub fn decompress(&self) -> Result<Vec<f32>> {
        let start = Instant::now();

        // Decompress ZSTD data (shuffled bytes)
        let mut decoder = Decoder::new(&self.compressed_data[..])?;
        let mut shuffled = Vec::new();
        decoder.read_to_end(&mut shuffled)?;

        let expected_bytes = self.original_dimensions * mem::size_of::<f32>();
        if shuffled.len() != expected_bytes {
            return Err(anyhow::anyhow!(
                "Decompressed size mismatch: expected {} bytes, got {}",
                expected_bytes,
                shuffled.len()
            ));
        }

        // Unshuffle back to original byte ordering
        let bytes = byte_unshuffle_4(&shuffled);

        // Verify checksum for data integrity (checksum is for unshuffled/original bytes)
        let checksum = crc32fast::hash(&bytes);
        if checksum != self.checksum {
            return Err(anyhow::anyhow!("Checksum mismatch: data corruption detected!"));
        }

        // Reconstruct f32 values
        let mut result = Vec::with_capacity(self.original_dimensions);
        for i in 0..self.original_dimensions {
            let idx = i * 4;
            let bits = u32::from_le_bytes([
                bytes[idx],
                bytes[idx + 1],
                bytes[idx + 2],
                bytes[idx + 3],
            ]);
            result.push(f32::from_bits(bits));
        }

        tracing::debug!(
            "Decompressed {} dimensions in {:?}",
            result.len(),
            start.elapsed()
        );

        Ok(result)
    }
    
    /// Get compressed size in bytes
    pub fn size_bytes(&self) -> usize {
        self.compressed_data.len()
    }
    
    /// Get compression ratio
    pub fn compression_ratio(&self) -> f32 {
        self.compression_ratio
    }
    
    /// Get original dimensions
    pub fn original_dimensions(&self) -> u32 {
        self.original_dimensions as u32
    }
    
    /// Get checksum
    pub fn checksum(&self) -> u32 {
        self.checksum
    }
    
    /// Get compressed data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.compressed_data
    }
    
    /// Create from compressed bytes by inferring metadata from payload.
    /// This will decompress to infer the original length and checksum.
    /// Note: This does not allocate the decompressed f32 vector permanently.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Decompress to shuffled bytes
        let mut decoder = Decoder::new(bytes)?;
        let mut shuffled = Vec::new();
        decoder.read_to_end(&mut shuffled)?;

        if shuffled.len() % mem::size_of::<f32>() != 0 {
            return Err(anyhow::anyhow!(
                "Invalid compressed payload: byte length {} not divisible by 4",
                shuffled.len()
            ));
        }

        // Unshuffle to compute checksum and metadata
        let bytes_unshuffled = byte_unshuffle_4(&shuffled);
        let dims = bytes_unshuffled.len() / mem::size_of::<f32>();
        let checksum = crc32fast::hash(&bytes_unshuffled);
        let compression_ratio = bytes.len() as f32 / bytes_unshuffled.len() as f32;

        Ok(Self {
            compressed_data: bytes.to_vec(),
            original_dimensions: dims,
            compression_ratio,
            checksum,
        })
    }

    /// Construct directly from known parts (preferred when metadata columns are stored separately)
    pub fn from_parts(
        compressed_data: Vec<u8>,
        original_dimensions: usize,
        compression_ratio: f32,
        checksum: u32,
    ) -> Result<Self> {
        Ok(Self {
            compressed_data,
            original_dimensions,
            compression_ratio,
            checksum,
        })
    }
    
    /// Get compression ratio (0.0 to 1.0)
    pub fn get_compression_ratio(&self) -> f32 {
        self.compression_ratio
    }
    
    /// Validate integrity without full decompression
    pub fn validate(&self) -> bool {
        // Quick validation - attempt to decompress and verify size
        match Decoder::new(&self.compressed_data[..]) {
            Ok(mut decoder) => {
                let mut shuffled = Vec::new();
                if decoder.read_to_end(&mut shuffled).is_err() {
                    return false;
                }
                let expected_bytes = self.original_dimensions * mem::size_of::<f32>();
                shuffled.len() == expected_bytes
            }
            Err(_) => false,
        }
    }
}

// ===== Helpers: byte-shuffle (4-byte width) =====

/// Byte-shuffle groups bytes by significance across values to improve compressibility.
/// Input is a stream of 4-byte little-endian values (e.g., f32/u32).
fn byte_shuffle_4(input: &[u8]) -> Vec<u8> {
    let n = input.len() / 4;
    let mut out = vec![0u8; n * 4];
    if n == 0 { return out; }
    for i in 0..4 {
        for j in 0..n {
            out[i * n + j] = input[j * 4 + i];
        }
    }
    out
}

fn byte_unshuffle_4(shuffled: &[u8]) -> Vec<u8> {
    let n = shuffled.len() / 4;
    let mut out = vec![0u8; n * 4];
    if n == 0 { return out; }
    for i in 0..4 {
        for j in 0..n {
            out[j * 4 + i] = shuffled[i * n + j];
        }
    }
    out
}

/// Batch compression for multiple embeddings
pub struct BatchCompressor {
    compression_level: i32,
    stats: CompressionStats,
}

#[derive(Debug, Default, Clone)]
pub struct CompressionStats {
    pub total_compressed: usize,
    pub total_original_bytes: usize,
    pub total_compressed_bytes: usize,
    pub average_compression_ratio: f32,
    pub best_ratio: f32,
    pub worst_ratio: f32,
}

impl BatchCompressor {
    pub fn new(compression_level: i32) -> Self {
        Self {
            compression_level: compression_level.clamp(1, 22),
            stats: CompressionStats::default(),
        }
    }
    
    /// Compress multiple embeddings in batch
    pub fn compress_batch(&mut self, embeddings: Vec<Vec<f32>>) -> Result<Vec<CompressedEmbedding>> {
        let mut compressed = Vec::with_capacity(embeddings.len());
        
        for embedding in embeddings {
            let compressed_emb = CompressedEmbedding::compress(&embedding)?;
            
            // Update stats
            let original_size = embedding.len() * std::mem::size_of::<f32>();
            let compressed_size = compressed_emb.size_bytes();
            let ratio = compressed_emb.get_compression_ratio();
            
            self.stats.total_compressed += 1;
            self.stats.total_original_bytes += original_size;
            self.stats.total_compressed_bytes += compressed_size;
            
            if self.stats.best_ratio == 0.0 || ratio < self.stats.best_ratio {
                self.stats.best_ratio = ratio;
            }
            if ratio > self.stats.worst_ratio {
                self.stats.worst_ratio = ratio;
            }
            
            compressed.push(compressed_emb);
        }
        
        // Update average
        self.stats.average_compression_ratio = 
            self.stats.total_compressed_bytes as f32 / self.stats.total_original_bytes as f32;
        
        Ok(compressed)
    }
    
    pub fn get_stats(&self) -> &CompressionStats {
        &self.stats
    }
    
    pub fn print_stats(&self) {
        println!("\n📊 Compression Statistics:");
        println!("   Total embeddings: {}", self.stats.total_compressed);
        println!("   Original size: {:.2} MB", self.stats.total_original_bytes as f64 / 1_048_576.0);
        println!("   Compressed size: {:.2} MB", self.stats.total_compressed_bytes as f64 / 1_048_576.0);
        println!("   Average ratio: {:.1}%", self.stats.average_compression_ratio * 100.0);
        println!("   Best ratio: {:.1}%", self.stats.best_ratio * 100.0);
        println!("   Worst ratio: {:.1}%", self.stats.worst_ratio * 100.0);
        println!("   Space saved: {:.1}%", (1.0 - self.stats.average_compression_ratio) * 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    
    #[test]
    fn test_compression_decompression_exact() {
        // Create test embedding
        let mut rng = rand::thread_rng();
        let embedding: Vec<f32> = (0..1536)
            .map(|_| rng.gen_range(-1.0..1.0))
            .collect();
        
        // Compress
        let compressed = CompressedEmbedding::compress(&embedding).unwrap();
        
        // Verify compression occurred
        let original_size = embedding.len() * std::mem::size_of::<f32>();
        assert!(compressed.size_bytes() < original_size, "Should compress data");
        assert!(compressed.compression_ratio < 1.0, "Ratio should be < 1.0");
        
        // Decompress
        let decompressed = compressed.decompress().unwrap();
        
        // Verify bit-perfect reconstruction
        assert_eq!(embedding.len(), decompressed.len(), "Same dimensions");
        
        for (i, (original, reconstructed)) in embedding.iter().zip(decompressed.iter()).enumerate() {
            assert_eq!(
                original.to_bits(), 
                reconstructed.to_bits(),
                "Bit-perfect match required at index {}. Original: {}, Reconstructed: {}",
                i, original, reconstructed
            );
        }
        
        println!("✅ Compression ratio: {:.1}%", compressed.compression_ratio * 100.0);
        println!("✅ Space saved: {:.1}%", (1.0 - compressed.compression_ratio) * 100.0);
    }
    
    #[test]
    fn test_batch_compression() {
        let mut compressor = BatchCompressor::new(3);
        
        // Create batch of embeddings
        let mut embeddings = Vec::new();
        let mut rng = rand::thread_rng();
        
        for _ in 0..10 {
            let embedding: Vec<f32> = (0..1536)
                .map(|_| rng.gen_range(-1.0..1.0))
                .collect();
            embeddings.push(embedding);
        }
        
        // Compress batch
        let compressed = compressor.compress_batch(embeddings.clone()).unwrap();
        
        // Verify all compressed successfully
        assert_eq!(compressed.len(), 10);
        
        // Verify each can be decompressed correctly
        for (original, compressed) in embeddings.iter().zip(compressed.iter()) {
            let decompressed = compressed.decompress().unwrap();
            assert_eq!(original.len(), decompressed.len());
            
            // Check bit-perfect
            for (o, d) in original.iter().zip(decompressed.iter()) {
                assert_eq!(o.to_bits(), d.to_bits());
            }
        }
        
        // Print stats
        compressor.print_stats();
        
        // Verify stats - random data doesn't compress well, but should still get some compression
        assert!(compressor.stats.average_compression_ratio < 0.95, "Should achieve at least 5% compression on random data");
        println!("✅ Average compression on random data: {:.1}%", compressor.stats.average_compression_ratio * 100.0);
    }
    
    #[test]
    fn test_validation() {
        let mut rng = rand::thread_rng();
        let embedding: Vec<f32> = (0..1536)
            .map(|_| rng.gen_range(-1.0..1.0))
            .collect();
        
        let compressed = CompressedEmbedding::compress(&embedding).unwrap();
        
        // Should validate correctly
        assert!(compressed.validate());
        
        // Corrupt data
        let mut corrupted = compressed.clone();
        if !corrupted.compressed_data.is_empty() {
            corrupted.compressed_data[0] ^= 0xFF;
        }
        
        // Should fail validation on decompression
        assert!(corrupted.decompress().is_err());
    }
    
    #[test]
    fn test_aws_titan_embedding_size() {
        // AWS Titan embeddings are 1536 dimensions
        let embedding = vec![0.5_f32; 1536];
        
        let compressed = CompressedEmbedding::compress(&embedding).unwrap();
        
        let original_size = 1536 * 4; // 6144 bytes
        let compressed_size = compressed.size_bytes();
        
        println!("AWS Titan embedding (1536 dims):");
        println!("  Original: {} bytes", original_size);
        println!("  Compressed: {} bytes", compressed_size);
        println!("  Ratio: {:.1}%", compressed.compression_ratio * 100.0);
        println!("  Saved: {:.1}%", (1.0 - compressed.compression_ratio) * 100.0);
        
        // Should achieve significant compression on repetitive data
        assert!(compressed.compression_ratio < 0.1, "Highly compressible test data");
    }
}

// Add checksum dependency
use crc32fast;
