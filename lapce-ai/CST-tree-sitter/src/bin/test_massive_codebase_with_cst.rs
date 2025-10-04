//! Comprehensive test for massive codebase with full CST storage
//! Tests all success criteria including memory usage per file size

use lapce_tree_sitter::all_languages_support::SupportedLanguage;
use lapce_tree_sitter::enhanced_codex_format::EnhancedSymbolExtractor;
use lapce_tree_sitter::performance_metrics::PerformanceTracker;
use lapce_tree_sitter::native_parser_manager::NativeParserManager;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::sync::Arc;
use parking_lot::RwLock;
use tree_sitter::{Parser, Tree};

/// Stores CST along with metadata
#[derive(Clone)]
struct StoredCST {
    file_path: PathBuf,
    file_size: usize,
    line_count: usize,
    tree: Option<Tree>,  // The actual CST
    parse_time_ms: f64,
    memory_before: u64,
    memory_after: u64,
    language: String,
    symbol_count: usize,
}

/// Memory statistics per file size category
#[derive(Default, Debug)]
struct MemoryStatsBySize {
    tiny_files: Vec<f64>,       // < 1KB
    small_files: Vec<f64>,      // 1KB - 10KB
    medium_files: Vec<f64>,     // 10KB - 100KB
    large_files: Vec<f64>,      // 100KB - 1MB
    huge_files: Vec<f64>,       // > 1MB
}

impl MemoryStatsBySize {
    fn add_sample(&mut self, file_size: usize, memory_mb: f64) {
        match file_size {
            0..=1024 => self.tiny_files.push(memory_mb),
            1025..=10240 => self.small_files.push(memory_mb),
            10241..=102400 => self.medium_files.push(memory_mb),
            102401..=1048576 => self.large_files.push(memory_mb),
            _ => self.huge_files.push(memory_mb),
        }
    }
    
    fn get_averages(&self) -> HashMap<&'static str, f64> {
        let mut result = HashMap::new();
        
        if !self.tiny_files.is_empty() {
            let avg = self.tiny_files.iter().sum::<f64>() / self.tiny_files.len() as f64;
            result.insert("tiny (<1KB)", avg);
        }
        if !self.small_files.is_empty() {
            let avg = self.small_files.iter().sum::<f64>() / self.small_files.len() as f64;
            result.insert("small (1-10KB)", avg);
        }
        if !self.medium_files.is_empty() {
            let avg = self.medium_files.iter().sum::<f64>() / self.medium_files.len() as f64;
            result.insert("medium (10-100KB)", avg);
        }
        if !self.large_files.is_empty() {
            let avg = self.large_files.iter().sum::<f64>() / self.large_files.len() as f64;
            result.insert("large (100KB-1MB)", avg);
        }
        if !self.huge_files.is_empty() {
            let avg = self.huge_files.iter().sum::<f64>() / self.huge_files.len() as f64;
            result.insert("huge (>1MB)", avg);
        }
        
        result
    }
}

fn main() {
    println!("\n🚀 MASSIVE CODEBASE TEST WITH FULL CST STORAGE");
    println!("{}", "=".repeat(80));
    println!();
    
    // Initialize performance tracker
    let mut tracker = PerformanceTracker::new();
    
    // Storage for all CSTs
    let stored_csts = Arc::new(RwLock::new(Vec::<StoredCST>::new()));
    let memory_stats = Arc::new(RwLock::new(MemoryStatsBySize::default()));
    
    // Collect files
    let base_path = Path::new("/home/verma/lapce/lapce-ai/massive_test_codebase");
    println!("📊 Collecting files from: {}", base_path.display());
    
    let files = collect_source_files(base_path);
    println!("✅ Found {} source files", files.len());
    
    // Group files by language
    let mut files_by_lang: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for file in &files {
        if let Some(ext) = file.extension().and_then(|s| s.to_str()) {
            files_by_lang.entry(ext.to_string()).or_insert_with(Vec::new).push(file.clone());
        }
    }
    
    println!("\n📊 Language Distribution:");
    for (lang, files) in &files_by_lang {
        println!("   {} files: {}", lang, files.len());
    }
    
    // Parse all files and store CSTs
    println!("\n🔧 Parsing files and storing CSTs...\n");
    
    let mut total_parsed = 0;
    let mut total_failed = 0;
    let mut total_lines = 0;
    let mut total_bytes = 0;
    let mut languages_working = HashMap::new();
    
    // Process files in batches to track memory growth
    let batch_size = 100;
    let mut batch_num = 0;
    
    for chunk in files.chunks(batch_size) {
        batch_num += 1;
        let batch_start = Instant::now();
        
        println!("📦 Batch {}/{}: Processing {} files", 
            batch_num, 
            (files.len() + batch_size - 1) / batch_size,
            chunk.len()
        );
        
        // Sample memory before batch
        tracker.sample_memory();
        let memory_before = get_current_memory_mb();
        
        for file_path in chunk {
            match process_file_with_cst(file_path, &mut tracker) {
                Ok(cst) => {
                    total_parsed += 1;
                    total_lines += cst.line_count;
                    total_bytes += cst.file_size;
                    
                    // Track language success
                    *languages_working.entry(cst.language.clone()).or_insert(0) += 1;
                    
                    // Track memory per file size
                    let memory_used = (cst.memory_after as f64 - cst.memory_before as f64) / 1_048_576.0;
                    memory_stats.write().add_sample(cst.file_size, memory_used);
                    
                    // Store the CST
                    stored_csts.write().push(cst);
                }
                Err(e) => {
                    total_failed += 1;
                    if total_failed <= 5 {
                        println!("   ❌ Failed: {:?} - {}", file_path, e);
                    }
                }
            }
        }
        
        // Sample memory after batch
        tracker.sample_memory();
        let memory_after = get_current_memory_mb();
        
        let batch_time = batch_start.elapsed();
        println!("   ⏱️  Batch time: {:.2}s", batch_time.as_secs_f64());
        println!("   💾 Memory: {:.2}MB → {:.2}MB (Δ{:.2}MB)",
            memory_before, memory_after, memory_after - memory_before
        );
        println!();
    }
    
    // Get final metrics
    let report = tracker.generate_report();
    let criteria = tracker.check_success_criteria();
    
    // Calculate statistics
    let success_rate = (total_parsed as f64 / files.len() as f64) * 100.0;
    let avg_parse_speed = report.parse.lines_per_second;
    
    // Print results
    println!("{}", "=".repeat(80));
    println!("📊 FINAL RESULTS");
    println!("{}", "=".repeat(80));
    
    println!("\n📈 Overall Statistics:");
    println!("  Total Files:           {}", files.len());
    println!("  Successfully Parsed:   {} ({:.1}%)", total_parsed, success_rate);
    println!("  Failed:                {}", total_failed);
    println!("  Total Lines:           {}", total_lines);
    println!("  Total Bytes:           {} ({:.2} MB)", total_bytes, total_bytes as f64 / 1_048_576.0);
    
    println!("\n🌍 Language Support:");
    for (lang, count) in languages_working.iter() {
        println!("  {}: {} files", lang, count);
    }
    println!("  Total Languages: {}", languages_working.len());
    
    // Memory analysis
    let csts = stored_csts.read();
    let total_cst_memory = estimate_cst_memory(&csts);
    
    println!("\n💾 Memory Analysis:");
    println!("  CSTs Stored:           {}", csts.len());
    println!("  Est. CST Memory:       {:.2} MB", total_cst_memory);
    println!("  Peak Memory:           {:.2} MB", report.memory.peak_usage_mb);
    println!("  Average Memory:        {:.2} MB", report.memory.average_usage_mb);
    
    // Memory by file size
    println!("\n💾 Memory Usage by File Size:");
    let size_averages = memory_stats.read().get_averages();
    for (size_category, avg_mb) in size_averages {
        println!("  {}: {:.3} MB average", size_category, avg_mb);
    }
    
    // CST statistics
    let avg_nodes = calculate_avg_nodes(&csts);
    let max_depth = calculate_max_depth(&csts);
    
    println!("\n🌲 CST Statistics:");
    println!("  Average Nodes/Tree:    {:.0}", avg_nodes);
    println!("  Max Tree Depth:        {}", max_depth);
    println!("  Avg Nodes/Line:        {:.2}", avg_nodes / (total_lines as f64 / csts.len() as f64));
    
    // Performance metrics
    println!("\n⚡ Performance Metrics:");
    println!("  Parse Speed:           {:.0} lines/second", avg_parse_speed);
    println!("  Avg Parse Time:        {:.2} ms", report.parse.average_parse_time.as_secs_f64() * 1000.0);
    println!("  Symbol Extraction:     {:.2} ms avg", 
        report.symbols.average_extraction_time.as_secs_f64() * 1000.0
    );
    
    // Success criteria check
    println!("\n{}", criteria.summary());
    
    // Final verdict
    println!("\n{}", "=".repeat(80));
    if criteria.all_passed() && success_rate > 95.0 {
        println!("✅ SUCCESS: All criteria met with {:.1}% success rate!", success_rate);
    } else if success_rate > 80.0 {
        println!("⚠️  PARTIAL SUCCESS: {:.1}% files parsed successfully", success_rate);
    } else {
        println!("❌ NEEDS IMPROVEMENT: Only {:.1}% success rate", success_rate);
    }
    
    // Memory per 1000 lines estimate
    let lines_per_mb = if total_cst_memory > 0.0 {
        total_lines as f64 / total_cst_memory
    } else {
        0.0
    };
    println!("\n📊 Efficiency Metric: {:.0} lines per MB of CST memory", lines_per_mb);
    println!("{}", "=".repeat(80));
}

fn collect_source_files(base_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            if path.is_dir() {
                files.extend(collect_source_files(&path));
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    // Include common source file extensions
                    if matches!(ext, 
                        "rs" | "js" | "ts" | "tsx" | "jsx" | 
                        "py" | "go" | "java" | "cpp" | "c" | 
                        "cs" | "rb" | "php" | "swift" | "kt" |
                        "scala" | "ex" | "hs" | "lua" | "sh"
                    ) {
                        files.push(path);
                    }
                }
            }
        }
    }
    
    files
}

fn process_file_with_cst(
    file_path: &Path, 
    tracker: &mut PerformanceTracker
) -> Result<StoredCST, Box<dyn std::error::Error>> {
    // Read file
    let content = fs::read_to_string(file_path)?;
    let file_size = content.len();
    let line_count = content.lines().count();
    
    // Get language
    let ext = file_path.extension()
        .and_then(|s| s.to_str())
        .ok_or("No extension")?;
    
    // Detect language
    let lang = SupportedLanguage::from_path(file_path.to_str().unwrap_or(""))
        .ok_or("Unsupported language")?;
    
    // Measure memory before parsing
    let memory_before = get_current_memory_bytes();
    
    // Parse with tree-sitter
    let parse_start = Instant::now();
    let mut parser = lang.get_parser()?;
    let tree = parser.parse(&content, None)
        .ok_or("Parse failed")?;
    let parse_time = parse_start.elapsed();
    
    // Measure memory after parsing
    let memory_after = get_current_memory_bytes();
    
    // Record metrics
    tracker.record_parse(parse_time, line_count, file_size);
    
    // Extract symbols (optional)
    let symbol_start = Instant::now();
    let mut extractor = EnhancedSymbolExtractor::new();
    let symbols = extractor.extract_symbols(ext, &content);
    let symbol_time = symbol_start.elapsed();
    
    let symbol_count = symbols
        .as_ref()
        .map(|s| s.lines().count())
        .unwrap_or(0);
    
    tracker.record_symbol_extraction(symbol_time, symbol_count);
    
    Ok(StoredCST {
        file_path: file_path.to_path_buf(),
        file_size,
        line_count,
        tree: Some(tree),
        parse_time_ms: parse_time.as_secs_f64() * 1000.0,
        memory_before,
        memory_after,
        language: ext.to_string(),
        symbol_count,
    })
}

fn get_current_memory_bytes() -> u64 {
    use sysinfo::{Pid, System};

    let mut system = System::new();
    let pid = Pid::from_u32(std::process::id());
    system.refresh_process(pid);

    system
        .process(pid)
        .map(|p| p.memory() * 1024) // memory() returns KB
        .unwrap_or(0)
}

fn get_current_memory_mb() -> f64 {
    get_current_memory_bytes() as f64 / 1_048_576.0
}

fn estimate_cst_memory(csts: &[StoredCST]) -> f64 {
    // Estimate based on nodes and tree structure
    // Each node approximately 100-200 bytes
    let total_nodes: usize = csts.iter()
        .filter_map(|cst| cst.tree.as_ref())
        .map(|tree| tree.root_node().descendant_count())
        .sum();
    
    // Rough estimate: 150 bytes per node
    (total_nodes * 150) as f64 / 1_048_576.0
}

fn calculate_avg_nodes(csts: &[StoredCST]) -> f64 {
    let total_nodes: usize = csts.iter()
        .filter_map(|cst| cst.tree.as_ref())
        .map(|tree| tree.root_node().descendant_count())
        .sum();
    
    if csts.is_empty() {
        0.0
    } else {
        total_nodes as f64 / csts.len() as f64
    }
}

fn calculate_max_depth(csts: &[StoredCST]) -> usize {
    fn get_depth(node: tree_sitter::Node) -> usize {
        let mut max_child_depth = 0;
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            max_child_depth = max_child_depth.max(get_depth(child));
        }
        
        max_child_depth + 1
    }
    
    csts.iter()
        .filter_map(|cst| cst.tree.as_ref())
        .map(|tree| get_depth(tree.root_node()))
        .max()
        .unwrap_or(0)
}
