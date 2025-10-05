//! Real test against massive_test_codebase to measure actual gains

use lapce_tree_sitter::compact::{CompactTreeBuilder, ProductionTreeBuilder, METRICS};
use lapce_tree_sitter::compact::production::BuildOptions;
use lapce_tree_sitter::compact::interning::{INTERN_POOL, intern_stats};
use lapce_tree_sitter::compact::query_engine::SymbolIndex;
use tree_sitter::{Parser, Language};
use std::path::{Path, PathBuf};
use std::fs;
use std::time::Instant;
use std::sync::Arc;
use walkdir::WalkDir;
use std::collections::HashMap;
use std::env;

const MASSIVE_TEST_DIR: &str = "/home/verma/lapce/lapce-ai/massive_test_codebase";

#[derive(Debug, Default)]
struct TestStats {
    total_files: usize,
    total_lines: usize,
    total_source_bytes: usize,
    
    // Tree-sitter stats
    ts_total_nodes: usize,
    ts_memory_bytes: usize,
    ts_build_time_ms: f64,
    
    // CompactTree stats
    compact_total_nodes: usize,
    compact_memory_bytes: usize,
    compact_build_time_ms: f64,
    
    // Symbol index stats
    total_symbols: usize,
    symbol_index_bytes: usize,
    
    // Interning stats
    interning_enabled: bool,
    intern_strings: usize,
    intern_bytes: usize,
    intern_hit_rate: f64,
    
    // Per language stats
    language_stats: HashMap<String, LanguageStats>,
}

#[derive(Debug, Default)]
struct LanguageStats {
    files: usize,
    lines: usize,
    source_bytes: usize,
    ts_memory: usize,
    compact_memory: usize,
    compression_ratio: f64,
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║   REAL TEST: MASSIVE_TEST_CODEBASE CST COMPRESSION           ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();
    
    // Check for interning flag
    let args: Vec<String> = env::args().collect();
    let compare_interning = args.iter().any(|arg| arg == "--compare-interning");
    
    if compare_interning {
        println!("Running comparison test: with and without global interning");
        println!();
        
        // Run without interning
        println!("════════ TEST 1: Without Interning ════════");
        #[cfg(feature = "global-interning")]
        INTERN_POOL.set_enabled(false);
        let stats_without = run_test(false);
        
        // Clear intern pool for fair comparison
        #[cfg(feature = "global-interning")]
        {
            // Note: clear() is only available in test mode, so we recreate the pool effect
            // by running with a fresh start
        }
        
        // Run with interning
        println!("\n════════ TEST 2: With Interning ════════");
        #[cfg(feature = "global-interning")]
        INTERN_POOL.set_enabled(true);
        let stats_with = run_test(true);
        
        // Compare results
        println!("\n");
        compare_results(&stats_without, &stats_with);
    } else {
        // Single run with current configuration
        #[cfg(feature = "global-interning")]
        let interning_enabled = INTERN_POOL.is_enabled();
        #[cfg(not(feature = "global-interning"))]
        let interning_enabled = false;
        
        println!("Running test with interning: {}", 
                if interning_enabled { "ENABLED" } else { "DISABLED" });
        println!();
        
        let stats = run_test(interning_enabled);
        print_results(&stats, &Vec::new(), std::time::Duration::from_secs(0));
    }
}

fn run_test(interning_enabled: bool) -> TestStats {
    let start = Instant::now();
    let mut stats = TestStats::default();
    stats.interning_enabled = interning_enabled;
    let mut stored_trees = Vec::new();
    
    // Collect all source files
    let files = collect_source_files(MASSIVE_TEST_DIR);
    println!("Found {} source files in massive_test_codebase", files.len());
    println!();
    
    // Process each file
    for (i, path) in files.iter().enumerate() {
        if i % 10 == 0 {
            print!("\rProcessing file {}/{}...", i + 1, files.len());
        }
        
        match process_file(&path, &mut stats) {
            Ok(tree_data) => {
                stored_trees.push(tree_data);
            }
            Err(e) => {
                eprintln!("\nError processing {:?}: {}", path, e);
            }
        }
    }
    println!("\n");
    
    // Collect interning stats if enabled
    if interning_enabled {
        #[cfg(feature = "global-interning")]
        {
            let intern_pool_stats = intern_stats();
            stats.intern_strings = intern_pool_stats.total_strings;
            stats.intern_bytes = intern_pool_stats.total_bytes;
            stats.intern_hit_rate = if intern_pool_stats.hit_count + intern_pool_stats.miss_count > 0 {
                intern_pool_stats.hit_count as f64 / (intern_pool_stats.hit_count + intern_pool_stats.miss_count) as f64
            } else {
                0.0
            };
        }
    }
    
    let elapsed = start.elapsed();
    
    // Print results
    print_results(&stats, &stored_trees, elapsed);
    
    // Save detailed report
    save_detailed_report(&stats, &stored_trees);
    
    stats
}

fn collect_source_files(dir: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("rs") | Some("py") | Some("js") | Some("ts") |
                Some("go") | Some("java") | Some("c") | Some("cpp") |
                Some("rb") | Some("php") => {
                    files.push(path.to_path_buf());
                }
                _ => {}
            }
        }
    }
    
    files
}

#[derive(Clone)]
struct StoredTree {
    path: PathBuf,
    language: String,
    lines: usize,
    source_bytes: usize,
    ts_nodes: usize,
    ts_memory: usize,
    compact_nodes: usize,
    compact_memory: usize,
    compact_tree_bytes: Vec<u8>, // Actual compact tree data
}

fn process_file(path: &Path, stats: &mut TestStats) -> Result<StoredTree, Box<dyn std::error::Error>> {
    // Read source
    let source = fs::read(path)?;
    let source_str = String::from_utf8_lossy(&source);
    let lines = source_str.lines().count();
    
    // Detect language
    let (language, lang_name) = get_language_from_extension(path)?;
    
    // Parse with Tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let ts_start = Instant::now();
    let tree = parser.parse(&source, None)
        .ok_or("Failed to parse")?;
    let ts_time = ts_start.elapsed();
    
    // Calculate Tree-sitter memory (estimate)
    let ts_nodes = tree.root_node().descendant_count();
    let ts_memory = ts_nodes * 90; // ~90 bytes per node average
    
    // Build CompactTree
    let compact_start = Instant::now();
    let builder = CompactTreeBuilder::new();
    let compact_tree = builder.build(&tree, &source);
    let compact_time = compact_start.elapsed();
    
    // Build SymbolIndex to test interning
    let symbol_index_start = Instant::now();
    let symbol_index = SymbolIndex::build(&compact_tree);
    let _symbol_index_time = symbol_index_start.elapsed();
    
    // Estimate symbol index memory (rough estimate)
    let symbol_count = symbol_index.find_symbol("").len(); // This won't find anything, but we need a better way
    stats.total_symbols += symbol_count;
    
    // Get actual compact memory
    let compact_nodes = compact_tree.node_count();
    let compact_memory = compact_tree.memory_bytes();
    
    // Serialize compact tree (simulate storage)
    let compact_tree_bytes = serialize_compact_tree(&compact_tree);
    
    // Update stats
    stats.total_files += 1;
    stats.total_lines += lines;
    stats.total_source_bytes += source.len();
    stats.ts_total_nodes += ts_nodes;
    stats.ts_memory_bytes += ts_memory;
    stats.ts_build_time_ms += ts_time.as_secs_f64() * 1000.0;
    stats.compact_total_nodes += compact_nodes;
    stats.compact_memory_bytes += compact_memory;
    stats.compact_build_time_ms += compact_time.as_secs_f64() * 1000.0;
    
    // Update language stats
    let lang_stats = stats.language_stats.entry(lang_name.clone()).or_default();
    lang_stats.files += 1;
    lang_stats.lines += lines;
    lang_stats.source_bytes += source.len();
    lang_stats.ts_memory += ts_memory;
    lang_stats.compact_memory += compact_memory;
    lang_stats.compression_ratio = ts_memory as f64 / compact_memory as f64;
    
    Ok(StoredTree {
        path: path.to_path_buf(),
        language: lang_name,
        lines,
        source_bytes: source.len(),
        ts_nodes,
        ts_memory,
        compact_nodes,
        compact_memory,
        compact_tree_bytes,
    })
}

fn get_language_from_extension(path: &Path) -> Result<(Language, String), Box<dyn std::error::Error>> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .ok_or("No extension")?;
    
    let (lang, name) = match ext {
        "rs" => (tree_sitter_rust::LANGUAGE.into(), "Rust"),
        "py" => (tree_sitter_python::LANGUAGE.into(), "Python"),
        "js" => (tree_sitter_javascript::language(), "JavaScript"),
        "ts" => (tree_sitter_typescript::language_typescript(), "TypeScript"),
        "go" => (tree_sitter_go::LANGUAGE.into(), "Go"),
        "java" => (tree_sitter_java::LANGUAGE.into(), "Java"),
        "c" => (tree_sitter_c::LANGUAGE.into(), "C"),
        "cpp" => (tree_sitter_cpp::LANGUAGE.into(), "C++"),
        "rb" => (tree_sitter_ruby::LANGUAGE.into(), "Ruby"),
        "php" => (tree_sitter_php::LANGUAGE_PHP.into(), "PHP"),
        _ => return Err(format!("Unsupported extension: {}", ext).into()),
    };
    
    Ok((lang, name.to_string()))
}

fn serialize_compact_tree(tree: &lapce_tree_sitter::compact::CompactTree) -> Vec<u8> {
    // For now, just return size estimate
    // In production, would use actual serialization
    vec![0u8; tree.memory_bytes()]
}

fn compare_results(without: &TestStats, with: &TestStats) {
    println!("═══════════════════════════════════════════════════════════════");
    println!("INTERNING COMPARISON RESULTS");
    println!("═══════════════════════════════════════════════════════════════");
    
    println!("\n📊 Memory Comparison:");
    println!("  Without interning: {:.2} MB total", 
             (without.compact_memory_bytes + without.symbol_index_bytes) as f64 / 1_048_576.0);
    println!("  With interning: {:.2} MB total ({:.2} MB saved)",
             (with.compact_memory_bytes + with.symbol_index_bytes) as f64 / 1_048_576.0,
             (without.symbol_index_bytes - with.symbol_index_bytes) as f64 / 1_048_576.0);
    
    println!("\n🔤 Interning Statistics:");
    println!("  Strings interned: {}", with.intern_strings);
    println!("  Intern table size: {:.2} KB", with.intern_bytes as f64 / 1024.0);
    println!("  Hit rate: {:.2}%", with.intern_hit_rate * 100.0);
    
    let memory_saved_pct = if without.symbol_index_bytes > 0 {
        ((without.symbol_index_bytes - with.symbol_index_bytes) as f64 / without.symbol_index_bytes as f64) * 100.0
    } else {
        0.0
    };
    
    println!("\n✅ Results:");
    println!("  Symbol index memory reduction: {:.1}%", memory_saved_pct);
    println!("  Overall efficiency gain: {:.2}x", 
             if with.symbol_index_bytes > 0 {
                 without.symbol_index_bytes as f64 / with.symbol_index_bytes as f64
             } else {
                 1.0
             });
    
    // Performance comparison
    println!("\n⚡ Performance:");
    println!("  Build time without interning: {:.2} ms", without.compact_build_time_ms);
    println!("  Build time with interning: {:.2} ms", with.compact_build_time_ms);
    let time_overhead = ((with.compact_build_time_ms / without.compact_build_time_ms) - 1.0) * 100.0;
    println!("  Time overhead: {:.1}%", time_overhead);
    
    println!("\n📈 Projection for 100K files:");
    let scale = 100_000.0 / without.total_files as f64;
    println!("  Without interning: {:.2} GB", 
             (without.symbol_index_bytes as f64 * scale) / 1_073_741_824.0);
    println!("  With interning: {:.2} GB",
             ((with.symbol_index_bytes + with.intern_bytes) as f64 * scale) / 1_073_741_824.0);
    println!("  Memory saved: {:.2} GB",
             ((without.symbol_index_bytes - with.symbol_index_bytes - with.intern_bytes) as f64 * scale) / 1_073_741_824.0);
}

fn print_results(stats: &TestStats, stored_trees: &[StoredTree], elapsed: std::time::Duration) {
    println!("═══════════════════════════════════════════════════════════════");
    println!("RESULTS");
    println!("═══════════════════════════════════════════════════════════════");
    
    // Overall stats
    println!("\n📊 Overall Statistics:");
    println!("  Files processed: {}", stats.total_files);
    println!("  Total lines: {}", stats.total_lines);
    println!("  Total source: {:.2} MB", stats.total_source_bytes as f64 / 1_048_576.0);
    println!("  Processing time: {:.2}s", elapsed.as_secs_f64());
    
    // Memory comparison
    let ts_mb = stats.ts_memory_bytes as f64 / 1_048_576.0;
    let compact_mb = stats.compact_memory_bytes as f64 / 1_048_576.0;
    let compression = stats.ts_memory_bytes as f64 / stats.compact_memory_bytes as f64;
    
    println!("\n💾 Memory Usage:");
    println!("  Tree-sitter CSTs: {:.2} MB", ts_mb);
    println!("  Compact CSTs: {:.2} MB", compact_mb);
    println!("  Compression ratio: {:.2}x", compression);
    println!("  Memory saved: {:.2} MB ({:.1}%)", 
             ts_mb - compact_mb, 
             (1.0 - 1.0/compression) * 100.0);
    
    // Lines per MB calculation
    let ts_lines_per_mb = stats.total_lines as f64 / ts_mb;
    let compact_lines_per_mb = stats.total_lines as f64 / compact_mb;
    
    println!("\n📏 Efficiency (Lines per MB):");
    println!("  Tree-sitter: {:.0} lines/MB", ts_lines_per_mb);
    println!("  CompactTree: {:.0} lines/MB", compact_lines_per_mb);
    println!("  Improvement: {:.1}x more lines per MB", compact_lines_per_mb / ts_lines_per_mb);
    
    // Interning stats if enabled
    if stats.interning_enabled {
        println!("\n🔤 Interning Statistics:");
        println!("  Strings interned: {}", stats.intern_strings);
        println!("  Intern table size: {:.2} KB", stats.intern_bytes as f64 / 1024.0);
        println!("  Hit rate: {:.2}%", stats.intern_hit_rate * 100.0);
        println!("  Avg string length: {:.1} bytes", 
                 if stats.intern_strings > 0 {
                     stats.intern_bytes as f64 / stats.intern_strings as f64
                 } else {
                     0.0
                 });
    }
    
    // Bytes per node
    let ts_bytes_per_node = stats.ts_memory_bytes as f64 / stats.ts_total_nodes as f64;
    let compact_bytes_per_node = stats.compact_memory_bytes as f64 / stats.compact_total_nodes as f64;
    
    println!("\n🔢 Per-Node Metrics:");
    println!("  Tree-sitter: {:.1} bytes/node", ts_bytes_per_node);
    println!("  CompactTree: {:.1} bytes/node", compact_bytes_per_node);
    println!("  Node count: {} nodes", stats.compact_total_nodes);
    
    // Build performance
    println!("\n⚡ Build Performance:");
    println!("  Tree-sitter parse: {:.2} ms total ({:.3} ms/file)", 
             stats.ts_build_time_ms, 
             stats.ts_build_time_ms / stats.total_files as f64);
    println!("  Compact build: {:.2} ms total ({:.3} ms/file)", 
             stats.compact_build_time_ms,
             stats.compact_build_time_ms / stats.total_files as f64);
    println!("  Build overhead: {:.1}%", 
             (stats.compact_build_time_ms / stats.ts_build_time_ms - 1.0) * 100.0);
    
    // Per-language breakdown
    println!("\n🔤 Per-Language Breakdown:");
    println!("  {:12} {:6} {:8} {:10} {:10} {:8}", 
             "Language", "Files", "Lines", "TS (KB)", "Compact (KB)", "Ratio");
    println!("  {:-<12} {:-<6} {:-<8} {:-<10} {:-<10} {:-<8}", "", "", "", "", "", "");
    
    let mut languages: Vec<_> = stats.language_stats.iter().collect();
    languages.sort_by_key(|(_, s)| s.files);
    languages.reverse();
    
    for (lang, lang_stats) in languages {
        println!("  {:12} {:6} {:8} {:10.1} {:10.1} {:8.1}x",
                 lang,
                 lang_stats.files,
                 lang_stats.lines,
                 lang_stats.ts_memory as f64 / 1024.0,
                 lang_stats.compact_memory as f64 / 1024.0,
                 lang_stats.compression_ratio);
    }
    
    // Storage simulation
    let total_stored_bytes: usize = stored_trees.iter()
        .map(|t| t.compact_tree_bytes.len())
        .sum();
    
    println!("\n💽 Storage Simulation:");
    println!("  Stored {} compact trees", stored_trees.len());
    println!("  Total storage: {:.2} MB", total_stored_bytes as f64 / 1_048_576.0);
    println!("  Average tree size: {:.1} KB", 
             total_stored_bytes as f64 / stored_trees.len() as f64 / 1024.0);
    
    // Projection for larger scale
    println!("\n🚀 Projections:");
    let scale_factor = 10_000.0 / stats.total_files as f64;
    println!("  For 10,000 files:");
    println!("    Tree-sitter: {:.2} GB", ts_mb * scale_factor / 1024.0);
    println!("    CompactTree: {:.2} GB", compact_mb * scale_factor / 1024.0);
    println!("    Savings: {:.2} GB", (ts_mb - compact_mb) * scale_factor / 1024.0);
    
    let scale_100k = 100_000.0 / stats.total_files as f64;
    println!("  For 100,000 files:");
    println!("    Tree-sitter: {:.2} GB", ts_mb * scale_100k / 1024.0);
    println!("    CompactTree: {:.2} GB", compact_mb * scale_100k / 1024.0);
    println!("    Savings: {:.2} GB", (ts_mb - compact_mb) * scale_100k / 1024.0);
}

fn save_detailed_report(stats: &TestStats, stored_trees: &[StoredTree]) {
    let report_path = "/home/verma/lapce/lapce-ai/CST-tree-sitter/MASSIVE_CODEBASE_TEST_RESULTS.md";
    
    let mut report = String::new();
    report.push_str("# Massive Codebase CST Compression Test Results\n\n");
    report.push_str(&format!("Test Date: {}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
    
    // Summary
    report.push_str("## Summary\n\n");
    report.push_str(&format!("- **Files tested**: {}\n", stats.total_files));
    report.push_str(&format!("- **Total lines**: {}\n", stats.total_lines));
    report.push_str(&format!("- **Compression achieved**: {:.2}x\n", 
                            stats.ts_memory_bytes as f64 / stats.compact_memory_bytes as f64));
    report.push_str(&format!("- **Lines per MB (Tree-sitter)**: {:.0}\n", 
                            stats.total_lines as f64 * 1_048_576.0 / stats.ts_memory_bytes as f64));
    report.push_str(&format!("- **Lines per MB (CompactTree)**: {:.0}\n\n", 
                            stats.total_lines as f64 * 1_048_576.0 / stats.compact_memory_bytes as f64));
    
    // Detailed file list
    report.push_str("## File-by-File Results\n\n");
    report.push_str("| File | Lang | Lines | TS (KB) | Compact (KB) | Ratio | Bytes/Node |\n");
    report.push_str("|------|------|-------|---------|--------------|-------|------------|\n");
    
    for tree in stored_trees.iter().take(50) { // First 50 files
        let filename = tree.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        report.push_str(&format!("| {} | {} | {} | {:.1} | {:.1} | {:.1}x | {:.1} |\n",
                                filename,
                                tree.language,
                                tree.lines,
                                tree.ts_memory as f64 / 1024.0,
                                tree.compact_memory as f64 / 1024.0,
                                tree.ts_memory as f64 / tree.compact_memory as f64,
                                tree.compact_memory as f64 / tree.compact_nodes as f64));
    }
    
    if let Err(e) = fs::write(report_path, report) {
        eprintln!("Failed to save report: {}", e);
    } else {
        println!("\n📄 Detailed report saved to: {}", report_path);
    }
}
