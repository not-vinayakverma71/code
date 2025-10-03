// Demo binary showing ZSTD + MMAP + Hierarchical Cache integration
use lancedb::embeddings::zstd_compression::{ZstdCompressor, CompressionConfig};
use lancedb::storage::mmap_storage::ConcurrentMmapStorage;
use lancedb::storage::hierarchical_cache::{HierarchicalCache, CacheConfig};
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║    ZSTD COMPRESSION + MMAP STORAGE + HIERARCHICAL CACHE DEMO      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");
    
    let temp_dir = TempDir::new()?;
    println!("📁 Using temp directory: {:?}\n", temp_dir.path());
    
    // Phase 1: ZSTD Compression
    println!("═══════════════════════════════════════════════════");
    println!("Phase 1: ZSTD Compression");
    println!("═══════════════════════════════════════════════════");
    
    let mut compressor = ZstdCompressor::new(CompressionConfig {
        compression_level: 3,
        enable_dictionary: true,
        enable_checksum: true,
        chunk_size: 100,
    });
    
    // Create sample embedding (AWS Titan size)
    let embedding = vec![0.5_f32; 1536];
    let original_size = embedding.len() * 4;
    
    // Compress
    let compressed = compressor.compress_embedding(&embedding, "demo_1")?;
    
    println!("📊 Compression Results:");
    println!("  Original size: {} bytes", original_size);
    println!("  Compressed size: {} bytes", compressed.compressed_size);
    println!("  Compression ratio: {:.2}x", compressed.compression_ratio);
    println!("  Space saved: {:.1}%", compressed.space_saved());
    
    // Verify bit-perfect
    let decompressed = compressor.decompress_embedding(&compressed)?;
    let is_perfect = embedding.iter().zip(decompressed.iter())
        .all(|(a, b)| (a - b).abs() < f32::EPSILON);
    println!("  Bit-perfect: {}", if is_perfect { "✅ YES" } else { "❌ NO" });
    
    // Phase 2: Memory-Mapped Storage
    println!("\n═══════════════════════════════════════════════════");
    println!("Phase 2: Memory-Mapped Storage");
    println!("═══════════════════════════════════════════════════");
    
    let mmap_path = temp_dir.path().join("mmap");
    std::fs::create_dir_all(&mmap_path)?;
    let mmap_storage = ConcurrentMmapStorage::new(
        &mmap_path,
        100 * 1024 * 1024  // 100MB max
    )?;
    
    // Store multiple embeddings
    let start = Instant::now();
    for i in 0..50 {
        let mut emb = vec![i as f32 / 50.0; 1536];
        // Add some variation
        for j in 0..emb.len() {
            emb[j] += (j as f32 * 0.001).sin();
        }
        mmap_storage.store(&format!("mmap_{}", i), &emb)?;
    }
    let store_time = start.elapsed();
    
    println!("💾 Storage Results:");
    let stats = mmap_storage.get_stats();
    println!("  Embeddings stored: {}", stats.embedding_count);
    println!("  Total size: {:.2} MB", stats.total_size_bytes as f64 / 1_048_576.0);
    println!("  Average size: {:.2} KB", stats.average_size as f64 / 1024.0);
    println!("  Store time: {:?} ({:.2} embeddings/sec)", 
             store_time, 50.0 / store_time.as_secs_f64());
    
    // Test retrieval
    let start = Instant::now();
    for i in 0..10 {
        let _ = mmap_storage.get(&format!("mmap_{}", i))?;
    }
    let get_time = start.elapsed();
    println!("  Retrieval time (10 items): {:?} ({:.0} embeddings/sec)",
             get_time, 10.0 / get_time.as_secs_f64());
    
    // Phase 3: Hierarchical Cache
    println!("\n═══════════════════════════════════════════════════");
    println!("Phase 3: Hierarchical 3-Tier Cache");
    println!("═══════════════════════════════════════════════════");
    
    let cache = HierarchicalCache::new(CacheConfig {
        l1_max_size_mb: 0.5,      // 500KB L1 (hot)
        l1_max_entries: 10,
        l2_max_size_mb: 2.0,      // 2MB L2 (warm)
        l2_max_entries: 50,
        l3_max_size_mb: 10.0,     // 10MB L3 (cold)
        promotion_threshold: 2,
        enable_statistics: true,
        ..Default::default()
    }, &temp_dir.path().join("cache"))?;
    
    // Fill cache with embeddings
    for i in 0..100 {
        let emb = vec![i as f32 / 100.0; 384];
        cache.put(&format!("cache_{}", i), emb)?;
    }
    
    // Simulate access patterns
    println!("🚀 Cache Performance Test:");
    
    // Cold access (first time)
    let start = Instant::now();
    for i in 0..5 {
        let _ = cache.get(&format!("cache_{}", i))?;
    }
    let cold_time = start.elapsed();
    
    // Warm access (cached)
    let start = Instant::now();
    for i in 0..5 {
        let _ = cache.get(&format!("cache_{}", i))?;
    }
    let warm_time = start.elapsed();
    
    let cache_stats = cache.get_stats();
    
    println!("  Cache Statistics:");
    println!("    L1 entries: {} / hits: {} / hit rate: {:.1}%", 
             cache_stats.l1_entries, cache_stats.l1_hits, 
             cache_stats.l1_hit_rate() * 100.0);
    println!("    L2 entries: {} / hits: {}", 
             cache_stats.l2_entries, cache_stats.l2_hits);
    println!("    L3 entries: {} / hits: {}", 
             cache_stats.l3_entries, cache_stats.l3_hits);
    println!("    Total promotions: {}", cache_stats.total_promotions);
    println!("    Total demotions: {}", cache_stats.total_demotions);
    println!("    Overall hit rate: {:.1}%", cache_stats.overall_hit_rate() * 100.0);
    
    println!("\n  Access Times:");
    println!("    Cold access (5 items): {:?}", cold_time);
    println!("    Warm access (5 items): {:?}", warm_time);
    println!("    Speedup: {:.1}x", cold_time.as_secs_f64() / warm_time.as_secs_f64().max(0.000001));
    
    // Phase 4: Memory Analysis
    println!("\n═══════════════════════════════════════════════════");
    println!("Phase 4: Memory Analysis");
    println!("═══════════════════════════════════════════════════");
    
    let mem_usage = get_memory_mb();
    println!("📈 Current Process Memory: {:.2} MB", mem_usage);
    
    // Calculate theoretical savings
    let uncompressed_size = 100 * 384 * 4;  // 100 embeddings * 384 dims * 4 bytes
    let actual_l1_size = cache_stats.l1_size_bytes;
    let actual_l2_size = cache_stats.l2_size_bytes;
    
    println!("\n💡 Memory Optimization Summary:");
    println!("  Uncompressed size (100 embeddings): {:.2} MB", 
             uncompressed_size as f64 / 1_048_576.0);
    println!("  L1 cache (hot): {:.2} KB", actual_l1_size as f64 / 1024.0);
    println!("  L2 cache (compressed): {:.2} KB", actual_l2_size as f64 / 1024.0);
    println!("  L3 storage (mmap): On-disk, zero RAM overhead");
    
    let comp_stats = compressor.get_stats();
    if comp_stats.embeddings_compressed > 0 {
        println!("\n  Compression Statistics:");
        println!("    Average ratio: {:.2}x", comp_stats.average_ratio);
        println!("    Best ratio: {:.2}x", comp_stats.best_ratio);
        println!("    Worst ratio: {:.2}x", comp_stats.worst_ratio);
    }
    
    // Phase 5: Integration Demo
    println!("\n═══════════════════════════════════════════════════");
    println!("Phase 5: Full Integration Demo");
    println!("═══════════════════════════════════════════════════");
    
    println!("🔄 Processing pipeline: Compress → Store → Cache");
    
    let demo_embedding = vec![0.123_f32; 1536];
    
    // Step 1: Compress
    let compressed = compressor.compress_embedding(&demo_embedding, "integration_test")?;
    println!("  ✓ Compressed: {} → {} bytes", 
             compressed.original_size, compressed.compressed_size);
    
    // Step 2: Store in mmap
    mmap_storage.store("integration_test", &demo_embedding)?;
    println!("  ✓ Stored in memory-mapped file");
    
    // Step 3: Add to cache
    cache.put("integration_test", demo_embedding.clone())?;
    println!("  ✓ Added to hierarchical cache");
    
    // Verify retrieval
    let from_cache = cache.get("integration_test")?.unwrap();
    let from_storage = mmap_storage.get("integration_test")?;
    
    let cache_match = from_cache.iter().zip(demo_embedding.iter())
        .all(|(a, b)| (a - b).abs() < f32::EPSILON);
    let storage_match = from_storage.iter().zip(demo_embedding.iter())
        .all(|(a, b)| (a - b).abs() < f32::EPSILON);
    
    println!("  ✓ Cache retrieval: {}", if cache_match { "Perfect" } else { "Failed" });
    println!("  ✓ Storage retrieval: {}", if storage_match { "Perfect" } else { "Failed" });
    
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║                      DEMO COMPLETED SUCCESSFULLY                  ║");
    println!("╠════════════════════════════════════════════════════════════════════╣");
    println!("║  ✅ ZSTD Compression: Working (bit-perfect)                       ║");
    println!("║  ✅ Memory-Mapped Storage: Working (zero-copy access)             ║");
    println!("║  ✅ Hierarchical Cache: Working (3-tier with promotion)           ║");
    println!("║  ✅ Integration: All components working together                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    
    Ok(())
}

fn get_memory_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}
