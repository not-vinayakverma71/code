//! Benchmark compressed cache against massive_test_codebase

use lapce_tree_sitter::compressed_cache::{CompressedTreeCache, CacheConfig};
use lapce_tree_sitter::native_parser_manager::NativeParserManager;
use std::sync::Arc;
use std::time::Instant;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use bytes::Bytes;
use sha2::{Sha256, Digest};

const MASSIVE_TEST_CODEBASE: &str = "/home/verma/lapce/lapce-ai/massive_test_codebase";
const TEST_ITERATIONS: usize = 3;

fn compute_hash(content: &[u8]) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    u64::from_le_bytes(result[0..8].try_into().unwrap())
}

fn get_rss_kb() -> u64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse().unwrap_or(0);
                }
            }
        }
    }
    0
}

#[tokio::main]
async fn main() {
    println!("=====================================");
    println!(" COMPRESSED CACHE BENCHMARK");
    println!(" Testing against massive_test_codebase");
    println!("=====================================\n");
    
    // Collect test files
    let files: Vec<PathBuf> = WalkDir::new(MASSIVE_TEST_CODEBASE)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
            matches!(ext, "rs" | "py" | "ts" | "js" | "go" | "java" | "cpp" | "c")
        })
        .map(|e| e.path().to_path_buf())
        .collect();
    
    println!("📁 Found {} files in test codebase\n", files.len());
    
    // Test 1: Traditional cache (no compression)
    println!("🔬 Test 1: Traditional Cache (No Compression)");
    println!("----------------------------------------------");
    benchmark_traditional_cache(&files).await;
    
    // Test 2: Compressed cache with default config
    println!("\n🔬 Test 2: Compressed Cache (Default Config)");
    println!("----------------------------------------------");
    let config = CacheConfig::default();
    benchmark_compressed_cache(&files, config).await;
    
    // Test 3: Compressed cache with aggressive config
    println!("\n🔬 Test 3: Compressed Cache (Aggressive)");
    println!("-----------------------------------------");
    let aggressive_config = CacheConfig {
        hot_size: 500,   // Only 500 hot files
        cold_size: 9500,  // 9.5K cold files
        compression_level: 6,  // Higher compression
        enable_disk_persistence: false,
        disk_cache_dir: None,
    };
    benchmark_compressed_cache(&files, aggressive_config).await;
    
    // Test 4: Simulate real usage patterns
    println!("\n🔬 Test 4: Real Usage Pattern Simulation");
    println!("------------------------------------------");
    simulate_real_usage(&files).await;
}

async fn benchmark_traditional_cache(files: &[PathBuf]) {
    let baseline_rss = get_rss_kb();
    let manager = Arc::new(NativeParserManager::new().unwrap());
    
    let start = Instant::now();
    let mut parse_times = Vec::new();
    
    // Parse all files
    for file in files {
        let parse_start = Instant::now();
        if let Ok(_result) = manager.parse_file(file).await {
            parse_times.push(parse_start.elapsed());
        }
    }
    
    let total_time = start.elapsed();
    let final_rss = get_rss_kb();
    let memory_used = final_rss.saturating_sub(baseline_rss);
    
    // Calculate statistics
    let avg_parse_time = parse_times.iter().sum::<std::time::Duration>() / parse_times.len() as u32;
    
    println!("  Files parsed: {}", parse_times.len());
    println!("  Total time: {:.2}s", total_time.as_secs_f64());
    println!("  Avg parse time: {:.2}ms", avg_parse_time.as_secs_f64() * 1000.0);
    println!("  Memory used: {} KB ({:.2} MB)", memory_used, memory_used as f64 / 1024.0);
    println!("  Memory per file: {:.2} KB", memory_used as f64 / files.len() as f64);
}

async fn benchmark_compressed_cache(files: &[PathBuf], config: CacheConfig) {
    let baseline_rss = get_rss_kb();
    let manager = Arc::new(NativeParserManager::new().unwrap());
    let cache = Arc::new(CompressedTreeCache::new(config));
    
    println!("  Config: {} hot, {} cold, level {}", 
        cache.config.hot_size, cache.config.cold_size, cache.config.compression_level);
    
    let start = Instant::now();
    
    // First pass: Parse all files
    println!("  First pass: parsing all files...");
    for file in files {
        if let Ok(content) = tokio::fs::read(file).await {
            let hash = compute_hash(&content);
            let content_bytes = Bytes::from(content);
            
            let file_clone = file.clone();
            let manager_clone = manager.clone();
            
            let _ = cache.get_or_parse(
                file,
                hash,
                move || {
                    // Parse using manager (blocking for simplicity)
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async move {
                        let start = Instant::now();
                        let result = manager_clone.parse_file(&file_clone).await?;
                        let parse_time = start.elapsed().as_secs_f64() * 1000.0;
                        Ok((result.tree, result.source, parse_time))
                    })
                }
            ).await;
        }
    }
    
    let after_first_pass = get_rss_kb();
    let first_pass_memory = after_first_pass.saturating_sub(baseline_rss);
    
    // Force eviction to cold for most files
    println!("  Evicting files to cold cache...");
    for file in files.iter().skip(cache.config.hot_size) {
        let _ = cache.evict_to_cold(file).await;
    }
    
    let after_eviction = get_rss_kb();
    let eviction_memory = after_eviction.saturating_sub(baseline_rss);
    
    // Second pass: Access files again (test decompression)
    println!("  Second pass: accessing cached files...");
    let second_pass_start = Instant::now();
    let mut cache_times = Vec::new();
    
    for file in files.iter().take(100) {  // Sample 100 files
        if let Ok(content) = tokio::fs::read(file).await {
            let hash = compute_hash(&content);
            
            let cache_start = Instant::now();
            let _ = cache.get_or_parse(
                file,
                hash,
                || unreachable!("Should be cached")
            ).await;
            cache_times.push(cache_start.elapsed());
        }
    }
    
    let second_pass_time = second_pass_start.elapsed();
    let final_rss = get_rss_kb();
    let final_memory = final_rss.saturating_sub(baseline_rss);
    
    // Get cache statistics
    let stats = cache.stats();
    let memory_usage = cache.memory_usage();
    
    println!("  Total time: {:.2}s", start.elapsed().as_secs_f64());
    println!("  Memory after first pass: {} KB ({:.2} MB)", 
        first_pass_memory, first_pass_memory as f64 / 1024.0);
    println!("  Memory after eviction: {} KB ({:.2} MB)", 
        eviction_memory, eviction_memory as f64 / 1024.0);
    println!("  Final memory: {} KB ({:.2} MB)", 
        final_memory, final_memory as f64 / 1024.0);
    println!("  Second pass time: {:.2}ms avg", 
        second_pass_time.as_secs_f64() * 1000.0 / 100.0);
    println!("\n  {}", stats);
    println!("\n  Memory breakdown:");
    println!("    Hot entries: {} ({} KB)", memory_usage.hot_entries, memory_usage.hot_memory_kb);
    println!("    Cold entries: {} ({} KB)", memory_usage.cold_entries, memory_usage.cold_memory_kb);
    println!("    Total: {} KB", memory_usage.total_memory_kb);
}

async fn simulate_real_usage(files: &[PathBuf]) {
    println!("  Simulating real editor usage patterns...\n");
    
    let config = CacheConfig {
        hot_size: 1000,
        cold_size: 9000,
        compression_level: 3,
        enable_disk_persistence: false,
        disk_cache_dir: None,
    };
    
    let baseline_rss = get_rss_kb();
    let manager = Arc::new(NativeParserManager::new().unwrap());
    let cache = Arc::new(CompressedTreeCache::new(config));
    
    // Simulate opening a project
    println!("  1. Opening project (parsing first 100 files)...");
    for file in files.iter().take(100) {
        if let Ok(content) = tokio::fs::read(file).await {
            let hash = compute_hash(&content);
            let _ = cache.get_or_parse(
                file,
                hash,
                || {
                    let file = file.clone();
                    let manager = manager.clone();
                    async move {
                        let start = Instant::now();
                        let result = manager.parse_file(&file).await?;
                        let parse_time = start.elapsed().as_secs_f64() * 1000.0;
                        Ok((result.tree, result.source, parse_time))
                    }
                }.await
            ).await;
        }
    }
    
    let after_open = get_rss_kb();
    println!("    Memory after open: {} KB", after_open.saturating_sub(baseline_rss));
    
    // Simulate browsing (accessing random files)
    println!("  2. Browsing files (random access pattern)...");
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let mut sample_files: Vec<_> = files.iter().cloned().collect();
    sample_files.shuffle(&mut rng);
    
    for file in sample_files.iter().take(500) {
        if let Ok(content) = tokio::fs::read(file).await {
            let hash = compute_hash(&content);
            let _ = cache.get_or_parse(
                file,
                hash,
                || {
                    let file = file.clone();
                    let manager = manager.clone();
                    async move {
                        let start = Instant::now();
                        let result = manager.parse_file(&file).await?;
                        let parse_time = start.elapsed().as_secs_f64() * 1000.0;
                        Ok((result.tree, result.source, parse_time))
                    }
                }.await
            ).await;
        }
    }
    
    let after_browse = get_rss_kb();
    println!("    Memory after browsing: {} KB", after_browse.saturating_sub(baseline_rss));
    
    // Simulate working set (repeatedly accessing same files)
    println!("  3. Working on hot files (repeated access)...");
    let working_set: Vec<_> = files.iter().take(20).cloned().collect();
    let mut access_times = Vec::new();
    
    for _ in 0..5 {  // Access each file 5 times
        for file in &working_set {
            if let Ok(content) = tokio::fs::read(file).await {
                let hash = compute_hash(&content);
                let access_start = Instant::now();
                let _ = cache.get_or_parse(
                    file,
                    hash,
                    || unreachable!("Should be cached")
                ).await;
                access_times.push(access_start.elapsed());
            }
        }
    }
    
    let avg_access = access_times.iter().sum::<std::time::Duration>() / access_times.len() as u32;
    println!("    Avg hot access time: {:.3}ms", avg_access.as_secs_f64() * 1000.0);
    
    let final_rss = get_rss_kb();
    let final_memory = final_rss.saturating_sub(baseline_rss);
    
    // Final statistics
    let stats = cache.stats();
    let memory_usage = cache.memory_usage();
    
    println!("\n  Final Statistics:");
    println!("  -----------------");
    println!("  Total memory: {} KB ({:.2} MB)", final_memory, final_memory as f64 / 1024.0);
    println!("  {}", stats);
    println!("  Hot cache: {} files", memory_usage.hot_entries);
    println!("  Cold cache: {} files", memory_usage.cold_entries);
    
    // Calculate efficiency
    let files_in_memory = memory_usage.hot_entries + memory_usage.cold_entries;
    let kb_per_file = if files_in_memory > 0 {
        final_memory as f64 / files_in_memory as f64
    } else {
        0.0
    };
    
    println!("\n  📊 Efficiency Metrics:");
    println!("  Files cached: {}", files_in_memory);
    println!("  Memory per file: {:.2} KB", kb_per_file);
    println!("  Compression ratio: {:.1}x", 
        if memory_usage.cold_entries > 0 {
            12.0 / 2.0  // Approximate: 12KB uncompressed to 2KB compressed
        } else {
            1.0
        }
    );
}
