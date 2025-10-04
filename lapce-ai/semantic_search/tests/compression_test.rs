// Test file for compression module (Task 1)
use lancedb::embeddings::compression::{CompressedEmbedding, BatchCompressor};
use rand::Rng;

#[test]
fn test_aws_titan_compression() {
    // AWS Titan embeddings are 1536 dimensions
    let mut rng = rand::thread_rng();
    
    // Create realistic embedding values (normalized between -1 and 1)
    let embedding: Vec<f32> = (0..1536)
        .map(|_| rng.gen_range(-1.0..1.0))
        .collect();
    
    // Compress the embedding
    let compressed = CompressedEmbedding::compress(&embedding).unwrap();
    
    // Calculate sizes
    let original_size = embedding.len() * std::mem::size_of::<f32>();
    let compressed_size = compressed.size_bytes();
    let compression_ratio = compressed.get_compression_ratio();
    
    println!("\n📊 AWS Titan Embedding Compression Results:");
    println!("   Original size: {} bytes ({:.2} KB)", original_size, original_size as f64 / 1024.0);
    println!("   Compressed size: {} bytes ({:.2} KB)", compressed_size, compressed_size as f64 / 1024.0);
    println!("   Compression ratio: {:.1}%", compression_ratio * 100.0);
    println!("   Space saved: {:.1}%", (1.0 - compression_ratio) * 100.0);
    
    // Decompress and verify bit-perfect reconstruction
    let decompressed = compressed.decompress().unwrap();
    
    assert_eq!(embedding.len(), decompressed.len(), "Dimensions must match");
    
    // Verify bit-perfect reconstruction
    for (i, (original, reconstructed)) in embedding.iter().zip(decompressed.iter()).enumerate() {
        assert_eq!(
            original.to_bits(),
            reconstructed.to_bits(),
            "Bit mismatch at index {}: {} != {}",
            i, original, reconstructed
        );
    }
    
    println!("   ✅ Bit-perfect reconstruction verified!");
    
    // Verify we achieve good compression
    assert!(compression_ratio < 0.8, "Should achieve at least 20% compression");
}

#[test] 
fn test_batch_compression_performance() {
    println!("\n📊 Batch Compression Performance Test:");
    
    let mut compressor = BatchCompressor::new(3);
    let mut rng = rand::thread_rng();
    
    // Simulate 100 files with ~10 embeddings each
    let num_files = 100;
    let embeddings_per_file = 10;
    
    let start = std::time::Instant::now();
    
    for file_idx in 0..num_files {
        let mut file_embeddings = Vec::new();
        
        for _ in 0..embeddings_per_file {
            let embedding: Vec<f32> = (0..1536)
                .map(|_| rng.gen_range(-1.0..1.0))
                .collect();
            file_embeddings.push(embedding);
        }
        
        compressor.compress_batch(file_embeddings).unwrap();
        
        if (file_idx + 1) % 20 == 0 {
            println!("   Processed {} files...", file_idx + 1);
        }
    }
    
    let elapsed = start.elapsed();
    
    compressor.print_stats();
    
    println!("\n⏱️  Performance Metrics:");
    println!("   Total time: {:?}", elapsed);
    println!("   Files/sec: {:.1}", num_files as f64 / elapsed.as_secs_f64());
    println!("   Embeddings/sec: {:.1}", 
        (num_files * embeddings_per_file) as f64 / elapsed.as_secs_f64());
    
    let stats = compressor.get_stats();
    
    // Calculate memory savings for 100 files
    let original_mb = stats.total_original_bytes as f64 / 1_048_576.0;
    let compressed_mb = stats.total_compressed_bytes as f64 / 1_048_576.0;
    
    println!("\n💾 Memory Usage (100 files, 1000 embeddings):");
    println!("   Original: {:.2} MB", original_mb);
    println!("   Compressed: {:.2} MB", compressed_mb);
    println!("   Saved: {:.2} MB ({:.1}%)", 
        original_mb - compressed_mb,
        (1.0 - stats.average_compression_ratio) * 100.0);
    
    // Verify compression targets
    assert!(stats.average_compression_ratio < 0.6, "Should achieve 40%+ compression");
}

#[test]
fn test_compression_validation() {
    let embedding = vec![0.5_f32; 1536];
    let compressed = CompressedEmbedding::compress(&embedding).unwrap();
    
    // Should pass validation
    assert!(compressed.validate(), "Valid compression should validate");
    
    // Test corrupted data by creating from bad bytes
    let mut corrupted_bytes = compressed.as_bytes().to_vec();
    if !corrupted_bytes.is_empty() {
        corrupted_bytes[0] ^= 0xFF; // Flip bits in compressed data
    }
    
    // Try to create from corrupted bytes
    match CompressedEmbedding::from_bytes(&corrupted_bytes) {
        Ok(corrupted) => {
            // If creation succeeds, decompression should fail
            match corrupted.decompress() {
                Ok(_) => panic!("Should have detected corruption"),
                Err(e) => println!("   ✅ Correctly detected corruption: {}", e),
            }
        }
        Err(e) => println!("   ✅ Correctly rejected corrupted data: {}", e),
    }
}
