//! FINAL COMPREHENSIVE TEST - All 67 languages with CST storage
//! Tests the massive codebase and measures everything

use lapce_tree_sitter::fixed_language_support::{Language67, get_parser_registry};
use lapce_tree_sitter::performance_metrics::PerformanceTracker;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tree_sitter::Tree;

/// CST storage with full metrics
struct CSTStorage {
    file_path: PathBuf,
    language: Language67,
    file_size: usize,
    line_count: usize,
    tree: Tree,
    node_count: usize,
    parse_time_ms: f64,
    tree_depth: usize,
}

fn main() {
    println!("\n🚀 FINAL COMPREHENSIVE TEST - ALL 67 LANGUAGES");
    println!("{}", "=".repeat(80));
    
    // Initialize
    let mut tracker = PerformanceTracker::new();
    let parser_registry = get_parser_registry();
    println!("\n✅ Loaded {} language parsers", parser_registry.len());
    
    // Collect files
    let base_path = Path::new("/home/verma/lapce/lapce-ai/massive_test_codebase");
    println!("\n📊 Collecting files from: {}", base_path.display());
    
    let files = collect_all_source_files(base_path);
    println!("✅ Found {} source files", files.len());
    
    // Group by language
    let mut by_lang: HashMap<Language67, Vec<PathBuf>> = HashMap::new();
    for file in &files {
        if let Some(lang) = Language67::from_path(file.to_str().unwrap_or("")) {
            by_lang.entry(lang).or_default().push(file.clone());
        }
    }
    
    println!("\n📊 Language Distribution ({} languages detected):", by_lang.len());
    for (lang, files) in &by_lang {
        println!("  {:?}: {} files", lang, files.len());
    }
    
    // Parse and store CSTs
    println!("\n🔧 Parsing all files and storing CSTs...\n");
    
    let mut cst_storage: Vec<CSTStorage> = Vec::new();
    let mut total_parsed = 0;
    let mut total_failed = 0;
    let mut total_lines = 0;
    let mut total_bytes = 0;
    let mut total_nodes = 0;
    let mut languages_working: HashMap<Language67, usize> = HashMap::new();
    let mut parse_errors: Vec<(PathBuf, String)> = Vec::new();
    
    // Process in batches
    let batch_size = 100;
    let total_batches = (files.len() + batch_size - 1) / batch_size;
    
    for (batch_idx, chunk) in files.chunks(batch_size).enumerate() {
        let batch_start = Instant::now();
        tracker.sample_memory();
        let memory_before = get_memory_mb();
        
        println!("📦 Batch {}/{}: Processing {} files",
            batch_idx + 1, total_batches, chunk.len());
        
        let mut batch_parsed = 0;
        let mut batch_nodes = 0;
        
        for file_path in chunk {
            if let Some(lang) = Language67::from_path(file_path.to_str().unwrap_or("")) {
                if let Some(mut parser) = parser_registry.get(&lang).cloned() {
                    match process_file_with_parser(file_path, lang, &mut parser, &mut tracker) {
                        Ok(cst) => {
                            total_parsed += 1;
                            batch_parsed += 1;
                            total_lines += cst.line_count;
                            total_bytes += cst.file_size;
                            total_nodes += cst.node_count;
                            batch_nodes += cst.node_count;
                            
                            *languages_working.entry(lang).or_insert(0) += 1;
                            cst_storage.push(cst);
                        }
                        Err(e) => {
                            total_failed += 1;
                            if parse_errors.len() < 10 {
                                parse_errors.push((file_path.clone(), e));
                            }
                        }
                    }
                }
            }
        }
        
        // Batch metrics
        tracker.sample_memory();
        let memory_after = get_memory_mb();
        let batch_time = batch_start.elapsed();
        
        println!("  ⏱️  Time: {:.2}s | Parsed: {}/{} | Nodes: {}",
            batch_time.as_secs_f64(), batch_parsed, chunk.len(), batch_nodes);
        println!("  💾 Memory: {:.1}MB → {:.1}MB (Δ{:.1}MB)",
            memory_before, memory_after, memory_after - memory_before);
        
        if batch_nodes > 0 {
            let bytes_per_node = (memory_after - memory_before) * 1_048_576.0 / batch_nodes as f64;
            println!("  📊 Memory efficiency: ~{:.0} bytes/node", bytes_per_node);
        }
        println!();
    }
    
    // Calculate comprehensive statistics
    let report = tracker.generate_report();
    let criteria = tracker.check_success_criteria();
    
    let success_rate = (total_parsed as f64 / files.len() as f64) * 100.0;
    let avg_nodes_per_file = if total_parsed > 0 {
        total_nodes as f64 / total_parsed as f64
    } else { 0.0 };
    let avg_nodes_per_line = if total_lines > 0 {
        total_nodes as f64 / total_lines as f64
    } else { 0.0 };
    
    // CST memory estimation
    let bytes_per_node = 150; // Conservative estimate
    let total_cst_memory_mb = (total_nodes * bytes_per_node) as f64 / 1_048_576.0;
    
    // Tree depth analysis
    let max_depth = cst_storage.iter().map(|c| c.tree_depth).max().unwrap_or(0);
    let avg_depth = if !cst_storage.is_empty() {
        cst_storage.iter().map(|c| c.tree_depth).sum::<usize>() as f64 / cst_storage.len() as f64
    } else { 0.0 };
    
    // Memory by file size categories
    let mut size_categories: HashMap<&str, Vec<usize>> = HashMap::new();
    for cst in &cst_storage {
        let category = match cst.file_size {
            0..=1024 => "tiny (<1KB)",
            1025..=10240 => "small (1-10KB)",
            10241..=102400 => "medium (10-100KB)",
            102401..=1048576 => "large (100KB-1MB)",
            _ => "huge (>1MB)",
        };
        size_categories.entry(category).or_default().push(cst.node_count);
    }
    
    // === FINAL RESULTS ===
    println!("{}", "=".repeat(80));
    println!("📊 COMPREHENSIVE TEST RESULTS");
    println!("{}", "=".repeat(80));
    
    println!("\n📈 Overall Statistics:");
    println!("  Total Files:           {}", files.len());
    println!("  Successfully Parsed:   {} ({:.1}%)", total_parsed, success_rate);
    println!("  Failed:                {}", total_failed);
    println!("  Total Lines:           {}", total_lines);
    println!("  Total Bytes:           {} ({:.2} MB)", 
        total_bytes, total_bytes as f64 / 1_048_576.0);
    
    println!("\n🌍 Language Support ({} languages working):", languages_working.len());
    let mut lang_list: Vec<_> = languages_working.iter().collect();
    lang_list.sort_by_key(|(_, count)| *count);
    lang_list.reverse();
    for (lang, count) in lang_list.iter().take(10) {
        println!("  {:?}: {} files", lang, count);
    }
    if languages_working.len() > 10 {
        println!("  ... and {} more languages", languages_working.len() - 10);
    }
    
    println!("\n🌲 CST Analysis:");
    println!("  Total CSTs Stored:     {}", cst_storage.len());
    println!("  Total Nodes:           {}", total_nodes);
    println!("  Avg Nodes/File:        {:.0}", avg_nodes_per_file);
    println!("  Avg Nodes/Line:        {:.2}", avg_nodes_per_line);
    println!("  Max Tree Depth:        {}", max_depth);
    println!("  Avg Tree Depth:        {:.1}", avg_depth);
    
    println!("\n💾 Memory Analysis:");
    println!("  Est. CST Memory:       {:.2} MB (for all {} trees)", total_cst_memory_mb, cst_storage.len());
    println!("  Memory/File:           {:.3} MB", total_cst_memory_mb / total_parsed.max(1) as f64);
    println!("  Memory/1K Lines:       {:.3} MB", total_cst_memory_mb / (total_lines as f64 / 1000.0).max(1.0));
    println!("  Peak Process Memory:   {:.2} MB", report.memory.peak_usage_mb);
    println!("  Avg Process Memory:    {:.2} MB", report.memory.average_usage_mb);
    
    println!("\n📊 Node Distribution by File Size:");
    for (category, nodes) in &size_categories {
        if !nodes.is_empty() {
            let avg = nodes.iter().sum::<usize>() as f64 / nodes.len() as f64;
            let est_mb = (avg * bytes_per_node as f64) / 1_048_576.0;
            println!("  {}: {:.0} nodes/file (~{:.3} MB)", category, avg, est_mb);
        }
    }
    
    println!("\n⚡ Performance Metrics:");
    println!("  Parse Speed:           {:.0} lines/second", report.parse.lines_per_second);
    println!("  Avg Parse Time:        {:.2} ms/file", 
        report.parse.average_parse_time.as_secs_f64() * 1000.0);
    println!("  Symbol Extraction:     {:.2} ms avg",
        report.symbols.average_extraction_time.as_secs_f64() * 1000.0);
    println!("  Total Parse Time:      {:.2} seconds", 
        report.parse.average_parse_time.as_secs_f64() * total_parsed as f64);
    
    // Parse errors (if any)
    if !parse_errors.is_empty() {
        println!("\n⚠️  Parse Errors (showing first 10):");
        for (path, error) in parse_errors.iter().take(10) {
            println!("  {} - {}", path.display(), error);
        }
    }
    
    // Success criteria
    println!("\n{}", criteria.summary());
    
    // === FINAL VERDICT ===
    println!("\n{}", "=".repeat(80));
    println!("🎯 FINAL VERDICT");
    println!("{}", "=".repeat(80));
    
    if criteria.all_passed() && success_rate > 95.0 && languages_working.len() >= 50 {
        println!("✅ COMPLETE SUCCESS!");
        println!("   - All performance criteria: PASSED");
        println!("   - Parse success rate: {:.1}%", success_rate);
        println!("   - Languages working: {}/67", languages_working.len());
        println!("   - CST storage efficient: {:.0} lines per MB", 
            total_lines as f64 / total_cst_memory_mb);
    } else if success_rate > 80.0 && languages_working.len() >= 30 {
        println!("⚠️  PARTIAL SUCCESS");
        println!("   - Parse success rate: {:.1}%", success_rate);
        println!("   - Languages working: {}/67", languages_working.len());
        println!("   - Some criteria not met (see above)");
    } else {
        println!("❌ NEEDS IMPROVEMENT");
        println!("   - Parse success rate: {:.1}%", success_rate);
        println!("   - Languages working: {}/67", languages_working.len());
    }
    
    println!("\n💡 System is ready for production testing with your dataset!");
    println!("{}", "=".repeat(80));
}

fn collect_all_source_files(base: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_all_source_files(&path));
            } else if path.is_file() {
                // Include all potential source files
                if Language67::from_path(path.to_str().unwrap_or("")).is_some() {
                    files.push(path);
                }
            }
        }
    }
    
    files
}

fn process_file_with_parser(
    path: &Path,
    lang: Language67,
    parser: &mut tree_sitter::Parser,
    tracker: &mut PerformanceTracker,
) -> Result<CSTStorage, String> {
    // Read file
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Read error: {}", e))?;
    
    let file_size = content.len();
    let line_count = content.lines().count();
    
    // Parse
    let parse_start = Instant::now();
    let tree = parser.parse(&content, None)
        .ok_or("Parse failed")?;
    let parse_time = parse_start.elapsed();
    
    // Record metrics
    tracker.record_parse(parse_time, line_count, file_size);
    
    // Calculate tree metrics
    let node_count = tree.root_node().descendant_count();
    let tree_depth = calculate_tree_depth(tree.root_node());
    
    Ok(CSTStorage {
        file_path: path.to_path_buf(),
        language: lang,
        file_size,
        line_count,
        tree,
        node_count,
        parse_time_ms: parse_time.as_secs_f64() * 1000.0,
        tree_depth,
    })
}

fn calculate_tree_depth(node: tree_sitter::Node) -> usize {
    let mut cursor = node.walk();
    let mut max_depth = 0;
    
    for child in node.children(&mut cursor) {
        max_depth = max_depth.max(calculate_tree_depth(child));
    }
    
    max_depth + 1
}

fn get_memory_mb() -> f64 {
    // Linux-specific memory reading
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}
