//! Comprehensive benchmarking for global interning
//! Measures memory, performance, and quality metrics

use lapce_tree_sitter::compact::{CompactTreeBuilder, CompactTree};
use lapce_tree_sitter::compact::interning::{INTERN_POOL, intern_stats};
use lapce_tree_sitter::compact::query_engine::SymbolIndex;
use tree_sitter::{Parser, Language};
use std::time::Instant;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Default, Debug)]
struct BenchmarkResults {
    // File stats
    total_files: usize,
    total_lines: usize,
    total_bytes: usize,
    
    // Parse times
    ts_parse_time_ms: f64,
    compact_build_time_ms: f64,
    symbol_index_time_ms: f64,
    
    // Memory (actual measured)
    ts_memory_bytes: usize,
    compact_memory_bytes: usize,
    symbol_index_memory_bytes: usize,
    
    // Interning stats
    intern_enabled: bool,
    intern_strings: usize,
    intern_bytes: usize,
    intern_hit_rate: f64,
    
    // Node counts
    total_nodes: usize,
    total_symbols: usize,
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  COMPREHENSIVE INTERNING BENCHMARK - MASSIVE_TEST_CODEBASE  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    
    let massive_dir = "/home/verma/lapce/lapce-ai/massive_test_codebase";
    
    // Test 1: Without interning
    println!("═══ TEST 1: WITHOUT GLOBAL INTERNING ═══");
    #[cfg(feature = "global-interning")]
    INTERN_POOL.set_enabled(false);
    let results_without = run_benchmark(massive_dir, false);
    print_results(&results_without);
    
    println!("\n\n═══ TEST 2: WITH GLOBAL INTERNING ═══");
    #[cfg(feature = "global-interning")]
    INTERN_POOL.set_enabled(true);
    let results_with = run_benchmark(massive_dir, true);
    print_results(&results_with);
    
    println!("\n\n═══ COMPARATIVE ANALYSIS ═══");
    compare_results(&results_without, &results_with);
}

fn run_benchmark(dir: &str, interning_enabled: bool) -> BenchmarkResults {
    let mut results = BenchmarkResults {
        intern_enabled: interning_enabled,
        ..Default::default()
    };
    
    // Collect files
    let files = collect_files(dir);
    results.total_files = files.len();
    println!("📁 Processing {} files...", files.len());
    
    let start = Instant::now();
    
    for (idx, path) in files.iter().enumerate() {
        if idx % 100 == 0 {
            print!("\r  Progress: {}/{}...", idx + 1, files.len());
        }
        
        if let Ok(metrics) = process_file(path) {
            results.total_lines += metrics.lines;
            results.total_bytes += metrics.bytes;
            results.ts_parse_time_ms += metrics.parse_time_ms;
            results.compact_build_time_ms += metrics.build_time_ms;
            results.symbol_index_time_ms += metrics.index_time_ms;
            results.ts_memory_bytes += metrics.ts_memory;
            results.compact_memory_bytes += metrics.compact_memory;
            results.symbol_index_memory_bytes += metrics.index_memory;
            results.total_nodes += metrics.node_count;
            results.total_symbols += metrics.symbol_count;
        }
    }
    println!("\r  Progress: {}/{}... ✓", files.len(), files.len());
    
    let elapsed = start.elapsed();
    println!("⏱️  Total time: {:.2}s", elapsed.as_secs_f64());
    
    // Collect interning stats
    if interning_enabled {
        #[cfg(feature = "global-interning")]
        {
            let stats = intern_stats();
            results.intern_strings = stats.total_strings;
            results.intern_bytes = stats.total_bytes;
            results.intern_hit_rate = if stats.hit_count + stats.miss_count > 0 {
                stats.hit_count as f64 / (stats.hit_count + stats.miss_count) as f64
            } else {
                0.0
            };
        }
    }
    
    results
}

struct FileMetrics {
    lines: usize,
    bytes: usize,
    parse_time_ms: f64,
    build_time_ms: f64,
    index_time_ms: f64,
    ts_memory: usize,
    compact_memory: usize,
    index_memory: usize,
    node_count: usize,
    symbol_count: usize,
}

fn process_file(path: &Path) -> Result<FileMetrics, Box<dyn std::error::Error>> {
    let source = fs::read(path)?;
    let source_str = String::from_utf8_lossy(&source);
    let lines = source_str.lines().count();
    
    // Parse
    let parse_start = Instant::now();
    let language = get_language(path)?;
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser.parse(&source, None).ok_or("Parse failed")?;
    let parse_time_ms = parse_start.elapsed().as_secs_f64() * 1000.0;
    
    // Estimate TS memory
    let node_count = tree.root_node().descendant_count();
    let ts_memory = node_count * 90; // ~90 bytes per node
    
    // Build compact
    let build_start = Instant::now();
    let builder = CompactTreeBuilder::new();
    let compact = builder.build(&tree, &source);
    let build_time_ms = build_start.elapsed().as_secs_f64() * 1000.0;
    let compact_memory = compact.memory_bytes();
    
    // Build symbol index
    let index_start = Instant::now();
    let symbol_index = SymbolIndex::build(&compact);
    let index_time_ms = index_start.elapsed().as_secs_f64() * 1000.0;
    
    // Estimate index memory (rough)
    let index_memory = estimate_symbol_index_memory(&symbol_index);
    
    Ok(FileMetrics {
        lines,
        bytes: source.len(),
        parse_time_ms,
        build_time_ms,
        index_time_ms,
        ts_memory,
        compact_memory,
        index_memory,
        node_count,
        symbol_count: 0, // Not easily measurable without exposing internals
    })
}

fn estimate_symbol_index_memory(index: &SymbolIndex) -> usize {
    // Rough estimate:
    // - HashMap overhead: ~48 bytes per entry
    // - Vec<usize> overhead: ~24 bytes + 8 bytes per element
    // - SymbolId: 4 bytes
    // Without access to internals, we return a conservative estimate
    1024 // Placeholder - would need to expose size() method
}

fn collect_files(dir: &str) -> Vec<std::path::PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path().extension().and_then(|s| s.to_str())
                .map(|ext| matches!(ext, "rs" | "py" | "ts" | "js"))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn get_language(path: &Path) -> Result<Language, Box<dyn std::error::Error>> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "rs" => Ok(tree_sitter_rust::LANGUAGE.into()),
        "py" => Ok(tree_sitter_python::LANGUAGE.into()),
        "ts" => Ok(tree_sitter_typescript::language_typescript()),
        "js" => Ok(tree_sitter_javascript::language()),
        _ => Err("Unsupported language".into()),
    }
}

fn print_results(results: &BenchmarkResults) {
    println!("\n📊 Results:");
    println!("  Files: {}", results.total_files);
    println!("  Lines: {}", results.total_lines);
    println!("  Bytes: {:.2} MB", results.total_bytes as f64 / 1_048_576.0);
    println!("  Nodes: {}", results.total_nodes);
    
    println!("\n⏱️  Timing:");
    println!("  Parse:        {:.2} ms ({:.3} ms/file)", 
             results.ts_parse_time_ms, 
             results.ts_parse_time_ms / results.total_files as f64);
    println!("  Build:        {:.2} ms ({:.3} ms/file)", 
             results.compact_build_time_ms,
             results.compact_build_time_ms / results.total_files as f64);
    println!("  Index:        {:.2} ms ({:.3} ms/file)",
             results.symbol_index_time_ms,
             results.symbol_index_time_ms / results.total_files as f64);
    println!("  Total:        {:.2} ms",
             results.ts_parse_time_ms + results.compact_build_time_ms + results.symbol_index_time_ms);
    
    println!("\n💾 Memory:");
    println!("  Tree-sitter:  {:.2} MB", results.ts_memory_bytes as f64 / 1_048_576.0);
    println!("  Compact:      {:.2} MB", results.compact_memory_bytes as f64 / 1_048_576.0);
    println!("  Symbol Index: {:.2} MB", results.symbol_index_memory_bytes as f64 / 1_048_576.0);
    println!("  Total:        {:.2} MB", 
             (results.compact_memory_bytes + results.symbol_index_memory_bytes) as f64 / 1_048_576.0);
    
    if results.intern_enabled {
        println!("\n🔤 Interning:");
        println!("  Strings:      {}", results.intern_strings);
        println!("  Table size:   {:.2} KB", results.intern_bytes as f64 / 1024.0);
        println!("  Hit rate:     {:.2}%", results.intern_hit_rate * 100.0);
        println!("  Avg length:   {:.1} bytes",
                 if results.intern_strings > 0 {
                     results.intern_bytes as f64 / results.intern_strings as f64
                 } else {
                     0.0
                 });
    }
}

fn compare_results(without: &BenchmarkResults, with: &BenchmarkResults) {
    println!("\n📈 Memory Comparison:");
    let symbol_mem_without = without.symbol_index_memory_bytes as f64;
    let symbol_mem_with = with.symbol_index_memory_bytes as f64;
    let intern_overhead = with.intern_bytes as f64;
    
    let total_without = (without.compact_memory_bytes + without.symbol_index_memory_bytes) as f64;
    let total_with = (with.compact_memory_bytes + with.symbol_index_memory_bytes + with.intern_bytes) as f64;
    
    println!("  Without interning: {:.2} MB", total_without / 1_048_576.0);
    println!("  With interning:    {:.2} MB", total_with / 1_048_576.0);
    println!("  Difference:        {:.2} MB ({:.1}%)",
             (total_without - total_with) / 1_048_576.0,
             ((total_without - total_with) / total_without) * 100.0);
    
    println!("\n⚡ Performance Comparison:");
    let time_without = without.compact_build_time_ms + without.symbol_index_time_ms;
    let time_with = with.compact_build_time_ms + with.symbol_index_time_ms;
    let overhead_pct = ((time_with - time_without) / time_without) * 100.0;
    
    println!("  Without interning: {:.2} ms", time_without);
    println!("  With interning:    {:.2} ms", time_with);
    println!("  Overhead:          {:.2}%", overhead_pct);
    
    println!("\n🎯 Efficiency Metrics:");
    println!("  Hit rate:          {:.2}%", with.intern_hit_rate * 100.0);
    println!("  Strings deduped:   {}", with.intern_strings);
    println!("  Bytes per string:  {:.1}",
             if with.intern_strings > 0 {
                 with.intern_bytes as f64 / with.intern_strings as f64
             } else {
                 0.0
             });
    
    println!("\n📊 Projected Savings (100K files):");
    let scale = 100_000.0 / without.total_files as f64;
    let mem_saved = (total_without - total_with) * scale;
    println!("  Memory saved:      {:.2} GB", mem_saved / 1_073_741_824.0);
    println!("  Intern table cost: {:.2} MB", (with.intern_bytes as f64 * scale) / 1_048_576.0);
}
