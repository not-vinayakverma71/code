//! REAL CST Memory Test - Actually stores CSTs and measures memory
//! Tests against massive_test_codebase to get true lines/MB ratio

use lapce_tree_sitter::native_parser_manager::NativeParserManager;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tree_sitter::Tree;
use walkdir::WalkDir;
use std::time::Instant;

const MASSIVE_TEST_CODEBASE: &str = "/home/verma/lapce/lapce-ai/massive_test_codebase";

struct StoredCST {
    tree: Tree,
    source: Vec<u8>,
    file_path: PathBuf,
    line_count: usize,
}

fn get_rss_kb() -> u64 {
    // Read actual RSS from /proc/self/status
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

fn main() {
    println!("=====================================");
    println!(" REAL CST MEMORY TEST");
    println!(" Testing actual CST storage in memory");
    println!("=====================================\n");

    // Measure baseline memory
    let baseline_rss = get_rss_kb();
    println!("📊 Baseline RSS: {} KB ({:.2} MB)\n", baseline_rss, baseline_rss as f64 / 1024.0);

    let rt = tokio::runtime::Runtime::new().unwrap();
    
    rt.block_on(async {
        // Create parser manager
        let manager = Arc::new(NativeParserManager::new().unwrap());
        
        // Storage for ALL CSTs - THIS IS THE KEY DIFFERENCE
        let mut stored_csts: HashMap<PathBuf, StoredCST> = HashMap::new();
        
        // Collect all test files
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

        println!("📁 Found {} files to parse\n", files.len());
        
        let mut total_lines = 0;
        let mut total_bytes = 0;
        let mut files_parsed = 0;
        let mut parse_errors = 0;
        
        println!("🔄 Parsing and storing CSTs...");
        let start_time = Instant::now();
        
        // Parse files in batches to see memory growth
        let batch_size = 100;
        for (batch_idx, chunk) in files.chunks(batch_size).enumerate() {
            for file_path in chunk {
                if let Ok(content) = std::fs::read(&file_path) {
                    let lines = content.iter().filter(|&&b| b == b'\n').count() + 1;
                    
                    match manager.parse_file(&file_path).await {
                        Ok(parse_result) => {
                            // ACTUALLY STORE THE CST
                            stored_csts.insert(
                                file_path.clone(),
                                StoredCST {
                                    tree: parse_result.tree,
                                    source: content.clone(),
                                    file_path: file_path.clone(),
                                    line_count: lines,
                                }
                            );
                            
                            total_lines += lines;
                            total_bytes += content.len();
                            files_parsed += 1;
                        }
                        Err(_) => {
                            parse_errors += 1;
                        }
                    }
                }
            }
            
            // Report memory every 100 files
            if (batch_idx + 1) % 1 == 0 {
                let current_rss = get_rss_kb();
                let memory_used_kb = current_rss.saturating_sub(baseline_rss);
                let memory_used_mb = memory_used_kb as f64 / 1024.0;
                
                println!("  Batch {}: {} CSTs stored, {:.2} MB used", 
                    batch_idx + 1, stored_csts.len(), memory_used_mb);
            }
        }
        
        let parse_time = start_time.elapsed();
        
        // Final memory measurement with all CSTs in memory
        let final_rss = get_rss_kb();
        let total_memory_kb = final_rss.saturating_sub(baseline_rss);
        let total_memory_mb = total_memory_kb as f64 / 1024.0;
        
        println!("\n=====================================");
        println!(" RESULTS");
        println!("=====================================\n");
        
        println!("📊 Parse Statistics:");
        println!("  Files parsed: {}", files_parsed);
        println!("  Parse errors: {}", parse_errors);
        println!("  Total lines: {}", total_lines);
        println!("  Total bytes: {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1024.0 / 1024.0);
        println!("  Parse time: {:.2}s", parse_time.as_secs_f64());
        println!("  Parse speed: {:.0} lines/second", total_lines as f64 / parse_time.as_secs_f64());
        
        println!("\n💾 Memory Usage (WITH CSTs STORED):");
        println!("  Baseline RSS: {} KB ({:.2} MB)", baseline_rss, baseline_rss as f64 / 1024.0);
        println!("  Final RSS: {} KB ({:.2} MB)", final_rss, final_rss as f64 / 1024.0);
        println!("  CST Memory: {} KB ({:.2} MB)", total_memory_kb, total_memory_mb);
        println!("  CSTs in memory: {}", stored_csts.len());
        
        println!("\n📈 Efficiency Metrics:");
        if total_memory_mb > 0.0 {
            let lines_per_mb = total_lines as f64 / total_memory_mb;
            let kb_per_cst = if files_parsed > 0 { total_memory_kb as f64 / files_parsed as f64 } else { 0.0 };
            let bytes_per_line = if total_lines > 0 { (total_memory_kb * 1024) as f64 / total_lines as f64 } else { 0.0 };
            
            println!("  Lines per MB: {:.0}", lines_per_mb);
            println!("  KB per CST: {:.2}", kb_per_cst);
            println!("  Bytes per line: {:.2}", bytes_per_line);
            println!("  Memory overhead: {:.1}x source size", 
                (total_memory_kb * 1024) as f64 / total_bytes as f64);
        }
        
        // Test: Can we still use the stored CSTs?
        println!("\n🔍 Verifying stored CSTs are usable:");
        let mut sample_count = 0;
        for (path, cst) in stored_csts.iter().take(5) {
            let root = cst.tree.root_node();
            println!("  {} - {} nodes, {} bytes", 
                path.file_name().unwrap().to_str().unwrap(),
                root.descendant_count(),
                root.byte_range().len()
            );
            sample_count += 1;
        }
        println!("  ✅ All {} sampled CSTs are valid and usable", sample_count);
        
        println!("\n=====================================");
        println!(" COMPARISON WITH REQUIREMENTS");
        println!("=====================================\n");
        
        let required_memory_mb = 5.0;
        let actual_per_parser = total_memory_mb / 69.0; // 69 languages
        
        println!("❌ Requirement: < {} MB for all parsers", required_memory_mb);
        println!("📊 Reality: {:.2} MB for {} CSTs", total_memory_mb, stored_csts.len());
        println!("📊 Per-parser estimate: {:.3} MB", actual_per_parser);
        
        if total_memory_mb > required_memory_mb {
            println!("\n⚠️  MEMORY USAGE IS {:.1}x THE REQUIREMENT", total_memory_mb / required_memory_mb);
            println!("    This is for {} ACTUAL CSTs stored in memory", stored_csts.len());
            println!("    Not just parser initialization!");
        }
        
        // Keep CSTs alive to ensure memory measurement is accurate
        std::thread::sleep(std::time::Duration::from_millis(100));
        println!("\n✅ Test complete. {} CSTs still in memory.", stored_csts.len());
    });
}
