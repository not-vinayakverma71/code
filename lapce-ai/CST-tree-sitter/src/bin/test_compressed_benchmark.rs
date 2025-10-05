//! Simpler benchmark for compressed cache testing

use std::path::PathBuf;
use std::time::Instant;
use walkdir::WalkDir;
use std::collections::HashMap;
use tree_sitter::{Parser, Tree};
use bytes::Bytes;

const MASSIVE_TEST_CODEBASE: &str = "/home/verma/lapce/lapce-ai/massive_test_codebase";

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

// Simple compressed entry
struct CompressedEntry {
    compressed_data: Vec<u8>,
    original_size: usize,
    compressed_size: usize,
}

fn main() {
    println!("=====================================");
    println!(" COMPRESSED CACHE BENCHMARK");
    println!(" Testing compression efficiency");
    println!("=====================================\n");
    
    // Collect test files
    let files: Vec<PathBuf> = WalkDir::new(MASSIVE_TEST_CODEBASE)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
            matches!(ext, "rs" | "py" | "ts")
        })
        .map(|e| e.path().to_path_buf())
        .take(1000) // Test with first 1000 files
        .collect();
    
    println!("📁 Found {} files to test\n", files.len());
    
    // Test 1: Baseline - Store uncompressed
    println!("Test 1: Uncompressed Storage");
    println!("-----------------------------");
    test_uncompressed(&files);
    
    // Test 2: Compressed storage
    println!("\nTest 2: Compressed Storage");
    println!("---------------------------");
    test_compressed(&files);
    
    // Test 3: Hybrid (hot + compressed)
    println!("\nTest 3: Hybrid Storage (100 hot + 900 compressed)");
    println!("---------------------------------------------------");
    test_hybrid(&files);
}

fn parse_file(path: &PathBuf) -> Option<(Tree, Vec<u8>)> {
    let content = std::fs::read(path).ok()?;
    let mut parser = Parser::new();
    
    // Detect language by extension
    let ext = path.extension()?.to_str()?;
    let lang = match ext {
        "rs" => tree_sitter_rust::LANGUAGE.into(),
        "py" => tree_sitter_python::LANGUAGE.into(),
        "ts" => tree_sitter_typescript::language_typescript(),
        _ => return None,
    };
    
    parser.set_language(&lang).ok()?;
    let tree = parser.parse(&content, None)?;
    
    Some((tree, content))
}

fn test_uncompressed(files: &[PathBuf]) {
    let baseline = get_rss_kb();
    let mut stored_trees = HashMap::new();
    let mut total_source_size = 0;
    let start = Instant::now();
    
    for file in files {
        if let Some((tree, source)) = parse_file(file) {
            total_source_size += source.len();
            stored_trees.insert(file.clone(), (tree, source));
        }
    }
    
    let parse_time = start.elapsed();
    let final_rss = get_rss_kb();
    let memory_used = final_rss.saturating_sub(baseline);
    
    println!("  Files stored: {}", stored_trees.len());
    println!("  Parse time: {:.2}s", parse_time.as_secs_f64());
    println!("  Source size: {} KB", total_source_size / 1024);
    println!("  Memory used: {} KB ({:.2} MB)", memory_used, memory_used as f64 / 1024.0);
    println!("  Memory per file: {:.2} KB", memory_used as f64 / stored_trees.len() as f64);
}

fn test_compressed(files: &[PathBuf]) {
    let baseline = get_rss_kb();
    let mut compressed_storage = HashMap::new();
    let mut total_original = 0;
    let mut total_compressed = 0;
    let start = Instant::now();
    
    for file in files {
        if let Ok(content) = std::fs::read(file) {
            total_original += content.len();
            
            // Compress with zstd
            let compressed = zstd::encode_all(&content[..], 3).unwrap_or(content.clone());
            total_compressed += compressed.len();
            
            compressed_storage.insert(file.clone(), CompressedEntry {
                compressed_data: compressed.clone(),
                original_size: content.len(),
                compressed_size: compressed.len(),
            });
        }
    }
    
    let compress_time = start.elapsed();
    let final_rss = get_rss_kb();
    let memory_used = final_rss.saturating_sub(baseline);
    
    println!("  Files compressed: {}", compressed_storage.len());
    println!("  Compression time: {:.2}s", compress_time.as_secs_f64());
    println!("  Original size: {} KB", total_original / 1024);
    println!("  Compressed size: {} KB", total_compressed / 1024);
    println!("  Compression ratio: {:.2}x", total_original as f64 / total_compressed as f64);
    println!("  Memory used: {} KB ({:.2} MB)", memory_used, memory_used as f64 / 1024.0);
    println!("  Memory per file: {:.2} KB", memory_used as f64 / compressed_storage.len() as f64);
    
    // Test decompression speed
    println!("\n  Testing decompression (100 files)...");
    let decompress_start = Instant::now();
    let mut decompress_count = 0;
    
    for (_, entry) in compressed_storage.iter().take(100) {
        let _ = zstd::decode_all(&entry.compressed_data[..]);
        decompress_count += 1;
    }
    
    let decompress_time = decompress_start.elapsed();
    println!("  Decompressed {} files in {:.3}ms", 
        decompress_count, 
        decompress_time.as_secs_f64() * 1000.0);
    println!("  Avg decompression: {:.3}ms per file",
        decompress_time.as_secs_f64() * 1000.0 / decompress_count as f64);
}

fn test_hybrid(files: &[PathBuf]) {
    let baseline = get_rss_kb();
    
    // Hot cache: first 100 files uncompressed
    let mut hot_trees = HashMap::new();
    let mut cold_compressed = HashMap::new();
    
    let hot_files = 100;
    let start = Instant::now();
    
    for (idx, file) in files.iter().enumerate() {
        if idx < hot_files {
            // Store uncompressed in hot cache
            if let Some((tree, source)) = parse_file(file) {
                hot_trees.insert(file.clone(), (tree, source));
            }
        } else {
            // Store compressed in cold cache
            if let Ok(content) = std::fs::read(file) {
                let compressed = zstd::encode_all(&content[..], 3).unwrap_or(content.clone());
                cold_compressed.insert(file.clone(), compressed);
            }
        }
    }
    
    let process_time = start.elapsed();
    let final_rss = get_rss_kb();
    let memory_used = final_rss.saturating_sub(baseline);
    
    println!("  Hot cache: {} files (uncompressed)", hot_trees.len());
    println!("  Cold cache: {} files (compressed)", cold_compressed.len());
    println!("  Total processing time: {:.2}s", process_time.as_secs_f64());
    println!("  Memory used: {} KB ({:.2} MB)", memory_used, memory_used as f64 / 1024.0);
    
    let total_files = hot_trees.len() + cold_compressed.len();
    println!("  Memory per file (avg): {:.2} KB", memory_used as f64 / total_files as f64);
    
    // Simulate access pattern
    println!("\n  Simulating access pattern...");
    let mut access_times = Vec::new();
    
    // Access hot files (should be fast)
    for (file, _) in hot_trees.iter().take(10) {
        let access_start = Instant::now();
        // Just accessing, already in memory
        access_times.push(access_start.elapsed());
    }
    
    let hot_avg = access_times.iter().sum::<std::time::Duration>() / access_times.len() as u32;
    println!("  Hot access avg: {:.3}μs", hot_avg.as_nanos() as f64 / 1000.0);
    
    // Access cold files (need decompression)
    access_times.clear();
    for (_, compressed) in cold_compressed.iter().take(10) {
        let access_start = Instant::now();
        let _ = zstd::decode_all(&compressed[..]);
        access_times.push(access_start.elapsed());
    }
    
    let cold_avg = access_times.iter().sum::<std::time::Duration>() / access_times.len() as u32;
    println!("  Cold access avg: {:.3}ms", cold_avg.as_secs_f64() * 1000.0);
    
    // Calculate efficiency
    println!("\n  📊 Efficiency Summary:");
    println!("  ----------------------");
    
    // Estimate memory if all were uncompressed
    let estimated_uncompressed = (memory_used as f64 / total_files as f64) * files.len() as f64;
    
    println!("  If all uncompressed: ~{:.2} MB", estimated_uncompressed / 1024.0);
    println!("  With hybrid approach: {:.2} MB", memory_used as f64 / 1024.0);
    println!("  Memory saved: {:.2} MB ({:.1}%)", 
        (estimated_uncompressed - memory_used as f64) / 1024.0,
        ((estimated_uncompressed - memory_used as f64) / estimated_uncompressed) * 100.0);
}
