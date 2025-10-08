//! Integration test for large codebase handling
//! Tests the system with a real-world sized codebase

use lapce_tree_sitter::{Phase4Cache, Phase4Config};
use std::path::{Path, PathBuf};
use std::time::{Instant, Duration};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::thread;
use tree_sitter::Parser;
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};

fn main() {
    println!("=== LARGE CODEBASE INTEGRATION TEST ===\n");
    
    // Find a large directory to test with
    let test_dir = find_test_directory();
    println!("Testing with directory: {}", test_dir.display());
    
    // Count files
    let files = collect_source_files(&test_dir, 10000); // Limit to 10k files
    println!("Found {} source files\n", files.len());
    
    if files.is_empty() {
        println!("No files found to test!");
        return;
    }
    
    let config = Phase4Config {
        memory_budget_mb: 100,      // 100MB budget
        hot_tier_ratio: 0.3,        // 30MB hot
        warm_tier_ratio: 0.3,       // 30MB warm  
        segment_size: 256 * 1024,   // 256KB segments
        storage_dir: std::env::temp_dir().join("lapce_test_large"),
        enable_compression: true,   // Use compression
        test_mode: false,           // Production mode
    };
    
    let cache = Arc::new(Phase4Cache::new(config).expect("Failed to create cache"));
    
    // Phase 1: Store all files
    println!("PHASE 1: STORING FILES");
    println!("{}", "=".repeat(50));
    
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos}/{len} {msg}")
            .unwrap()
    );
    
    let mut total_source_bytes = 0;
    let mut total_parse_time = Duration::ZERO;
    let mut stored_files = Vec::new();
    
    for (i, path) in files.iter().enumerate() {
        if let Ok(content) = std::fs::read_to_string(path) {
            total_source_bytes += content.len();
            
            let start = Instant::now();
            
            // Parse based on extension
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if let Some(tree) = parse_file(ext, &content) {
                let hash = hash_path(path);
                
                if cache.store(path.clone(), hash, tree, content.as_bytes()).is_ok() {
                    stored_files.push((path.clone(), hash, content.len()));
                    total_parse_time += start.elapsed();
                    
                    if i % 100 == 0 {
                        pb.set_message(format!("{:.1} MB processed", 
                            total_source_bytes as f64 / 1_048_576.0));
                    }
                }
            }
        }
        pb.inc(1);
    }
    
    pb.finish_with_message("Complete");
    
    let stats = cache.stats();
    println!("\nPhase 1 Results:");
    println!("  Files stored: {}", stored_files.len());
    println!("  Total source: {:.1} MB", total_source_bytes as f64 / 1_048_576.0);
    println!("  Parse time: {:.2}s", total_parse_time.as_secs_f64());
    println!("  Avg parse time: {:.1}ms/file", 
        total_parse_time.as_millis() as f64 / stored_files.len() as f64);
    println!("  Cache stats:");
    println!("    Hot: {} entries", stats.hot_entries);
    println!("    Warm: {} entries", stats.warm_entries);
    println!("    Cold: {} entries", stats.cold_entries);
    println!("    Frozen: {} entries", stats.frozen_entries);
    
    // Phase 2: Random access pattern
    println!("\nPHASE 2: RANDOM ACCESS PATTERN");
    println!("{}", "=".repeat(50));
    
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    
    let access_count = (stored_files.len() * 2).min(5000);
    let mut hits = 0;
    let mut total_access_time = Duration::ZERO;
    
    let pb = ProgressBar::new(access_count as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos}/{len} hits: {msg}")
            .unwrap()
    );
    
    for _ in 0..access_count {
        if let Some((path, hash, _)) = stored_files.choose(&mut rng) {
            let start = Instant::now();
            if cache.get(path, *hash).unwrap().is_some() {
                hits += 1;
            }
            total_access_time += start.elapsed();
            pb.set_message(format!("{}", hits));
        }
        pb.inc(1);
    }
    
    pb.finish();
    
    let hit_rate = hits as f64 / access_count as f64 * 100.0;
    println!("\nPhase 2 Results:");
    println!("  Access count: {}", access_count);
    println!("  Hits: {} ({:.1}%)", hits, hit_rate);
    println!("  Avg access time: {:.2}ms", 
        total_access_time.as_micros() as f64 / access_count as f64 / 1000.0);
    
    // Phase 3: Concurrent access
    println!("\nPHASE 3: CONCURRENT ACCESS");
    println!("{}", "=".repeat(50));
    
    let thread_count = 8;
    let accesses_per_thread = 1000;
    let total_hits = Arc::new(AtomicUsize::new(0));
    let total_misses = Arc::new(AtomicUsize::new(0));
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for tid in 0..thread_count {
        let cache = cache.clone();
        let files = stored_files.clone();
        let hits = total_hits.clone();
        let misses = total_misses.clone();
        
        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            
            for _ in 0..accesses_per_thread {
                if let Some((path, hash, _)) = files.choose(&mut rng) {
                    match cache.get(path, *hash) {
                        Ok(Some(_)) => hits.fetch_add(1, Ordering::Relaxed),
                        _ => misses.fetch_add(1, Ordering::Relaxed),
                    };
                }
            }
            
            println!("  Thread {} completed {} accesses", tid, accesses_per_thread);
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let concurrent_time = start.elapsed();
    let total_concurrent_accesses = thread_count * accesses_per_thread;
    let concurrent_hits = total_hits.load(Ordering::Relaxed);
    
    println!("\nPhase 3 Results:");
    println!("  Threads: {}", thread_count);
    println!("  Total accesses: {}", total_concurrent_accesses);
    println!("  Concurrent hits: {} ({:.1}%)", concurrent_hits,
        concurrent_hits as f64 / total_concurrent_accesses as f64 * 100.0);
    println!("  Time: {:.2}s", concurrent_time.as_secs_f64());
    println!("  Throughput: {:.0} accesses/sec", 
        total_concurrent_accesses as f64 / concurrent_time.as_secs_f64());
    
    // Phase 4: Memory pressure test
    println!("\nPHASE 4: MEMORY PRESSURE");
    println!("{}", "=".repeat(50));
    
    let final_stats = cache.stats();
    let memory_used = final_stats.total_memory_bytes as f64 / 1_048_576.0;
    let memory_budget = 100.0; // MB
    
    println!("  Memory budget: {:.1} MB", memory_budget);
    println!("  Memory used: {:.1} MB", memory_used);
    println!("  Within budget: {}", if memory_used <= memory_budget { "✅" } else { "❌" });
    println!("  Final distribution:");
    println!("    Hot: {} entries ({:.1} MB)", 
        final_stats.hot_entries,
        final_stats.hot_bytes as f64 / 1_048_576.0);
    println!("    Warm: {} entries ({:.1} MB)", 
        final_stats.warm_entries,
        final_stats.warm_bytes as f64 / 1_048_576.0);
    println!("    Cold: {} entries ({:.1} MB)", 
        final_stats.cold_entries,
        final_stats.cold_bytes as f64 / 1_048_576.0);
    println!("    Frozen: {} entries ({:.1} MB disk)", 
        final_stats.frozen_entries,
        final_stats.frozen_bytes as f64 / 1_048_576.0);
    
    // Summary
    println!("\n{}", "=".repeat(50));
    println!("INTEGRATION TEST SUMMARY");
    println!("{}", "=".repeat(50));
    
    if hit_rate > 95.0 && memory_used <= memory_budget && concurrent_hits > 0 {
        println!("✅ ALL TESTS PASSED");
        println!("  - Large codebase handled successfully");
        println!("  - High cache hit rate maintained");
        println!("  - Memory budget respected");
        println!("  - Concurrent access working");
    } else {
        println!("⚠️ SOME ISSUES DETECTED");
        if hit_rate <= 95.0 {
            println!("  - Hit rate below expected: {:.1}%", hit_rate);
        }
        if memory_used > memory_budget {
            println!("  - Memory budget exceeded: {:.1} MB", memory_used);
        }
        if concurrent_hits == 0 {
            println!("  - Concurrent access failed");
        }
    }
}

fn find_test_directory() -> PathBuf {
    // Try to find a suitable test directory
    let candidates = vec![
        PathBuf::from(".."),  // Parent directory
        PathBuf::from("../.."),  // Grandparent
        PathBuf::from("/home/verma/lapce"),  // User's project
        std::env::current_dir().unwrap(),  // Current dir
    ];
    
    for candidate in candidates {
        if candidate.exists() && candidate.is_dir() {
            // Count .rs files
            let count = WalkBuilder::new(&candidate)
                .max_depth(Some(3))
                .build()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| 
                    matches!(ext.to_str(), Some("rs") | Some("py") | Some("js"))))
                .take(100)
                .count();
            
            if count > 10 {
                return candidate;
            }
        }
    }
    
    std::env::current_dir().unwrap()
}

fn collect_source_files(dir: &Path, limit: usize) -> Vec<PathBuf> {
    WalkBuilder::new(dir)
        .max_depth(Some(10))
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map_or(false, |ft| ft.is_file()))
        .filter(|e| {
            e.path().extension().map_or(false, |ext| {
                matches!(ext.to_str(), 
                    Some("rs") | Some("py") | Some("js") | 
                    Some("ts") | Some("go") | Some("java"))
            })
        })
        .map(|e| e.path().to_path_buf())
        .take(limit)
        .collect()
}

fn parse_file(ext: &str, content: &str) -> Option<tree_sitter::Tree> {
    let mut parser = Parser::new();
    
    let language = match ext {
        "rs" => tree_sitter_rust::LANGUAGE.into(),
        "py" => tree_sitter_python::LANGUAGE.into(),
        "js" | "jsx" => return None, // JavaScript needs special handling
        "ts" | "tsx" => return None, // TypeScript needs special handling
        "go" => tree_sitter_go::LANGUAGE.into(),
        _ => return None,
    };
    
    parser.set_language(&language).ok()?;
    parser.parse(content, None)
}

fn hash_path(path: &Path) -> u64 {
    path.to_string_lossy().bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
}
