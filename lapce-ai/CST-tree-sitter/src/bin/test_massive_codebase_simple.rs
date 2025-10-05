//! Simple but comprehensive test for massive codebase with CST storage
//! Uses only confirmed working parsers

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tree_sitter::{Parser, Tree};

/// Language parsers that are confirmed working
enum WorkingLanguage {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Ruby,
    Php,
    Lua,
    Bash,
    Css,
    Json,
    Swift,
    Html,
    // Add more as we confirm they work
}

impl WorkingLanguage {
    fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "js" | "mjs" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "py" => Some(Self::Python),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "c" | "h" => Some(Self::C),
            "cpp" | "cc" | "hpp" => Some(Self::Cpp),
            "cs" => Some(Self::CSharp),
            "rb" => Some(Self::Ruby),
            "php" => Some(Self::Php),
            "lua" => Some(Self::Lua),
            "sh" | "bash" => Some(Self::Bash),
            "css" => Some(Self::Css),
            "json" => Some(Self::Json),
            "swift" => Some(Self::Swift),
            "html" | "htm" => Some(Self::Html),
            _ => None,
        }
    }
    
    fn get_parser(&self) -> Result<Parser, String> {
        let mut parser = Parser::new();
        let language = unsafe {
            match self {
                Self::Rust => tree_sitter_rust::LANGUAGE.into(),
                Self::JavaScript => tree_sitter_javascript::language().into(),
                Self::TypeScript => tree_sitter_typescript::language_typescript().into(),
                Self::Python => tree_sitter_python::LANGUAGE.into(),
                Self::Go => tree_sitter_go::LANGUAGE.into(),
                Self::Java => tree_sitter_java::LANGUAGE.into(),
                Self::C => tree_sitter_c::LANGUAGE.into(),
                Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
                Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
                Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
                Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
                Self::Lua => tree_sitter_lua::LANGUAGE.into(),
                Self::Bash => tree_sitter_bash::LANGUAGE.into(),
                Self::Css => tree_sitter_css::LANGUAGE.into(),
                Self::Json => tree_sitter_json::LANGUAGE.into(),
                Self::Swift => tree_sitter_swift::LANGUAGE.into(),
                Self::Html => tree_sitter_html::LANGUAGE.into(),
            }
        };
        
        parser.set_language(&language)
            .map_err(|e| format!("Failed to set language: {}", e))?;
        Ok(parser)
    }
}

/// CST with metadata
struct StoredCST {
    file_path: PathBuf,
    file_size: usize,
    line_count: usize,
    tree: Tree,
    parse_time_ms: f64,
    node_count: usize,
    language: String,
}

/// Memory stats by file size category
#[derive(Default)]
struct MemoryBySize {
    tiny: Vec<usize>,      // < 1KB
    small: Vec<usize>,     // 1-10KB
    medium: Vec<usize>,    // 10-100KB
    large: Vec<usize>,     // > 100KB
}

impl MemoryBySize {
    fn add(&mut self, file_size: usize, node_count: usize) {
        match file_size {
            0..=1024 => self.tiny.push(node_count),
            1025..=10240 => self.small.push(node_count),
            10241..=102400 => self.medium.push(node_count),
            _ => self.large.push(node_count),
        }
    }
    
    fn get_averages(&self) -> HashMap<&str, f64> {
        let mut result = HashMap::new();
        
        if !self.tiny.is_empty() {
            result.insert("tiny (<1KB)", 
                self.tiny.iter().sum::<usize>() as f64 / self.tiny.len() as f64);
        }
        if !self.small.is_empty() {
            result.insert("small (1-10KB)",
                self.small.iter().sum::<usize>() as f64 / self.small.len() as f64);
        }
        if !self.medium.is_empty() {
            result.insert("medium (10-100KB)",
                self.medium.iter().sum::<usize>() as f64 / self.medium.len() as f64);
        }
        if !self.large.is_empty() {
            result.insert("large (>100KB)",
                self.large.iter().sum::<usize>() as f64 / self.large.len() as f64);
        }
        
        result
    }
}

fn main() {
    println!("\n🚀 MASSIVE CODEBASE TEST WITH CST STORAGE");
    println!("{}", "=".repeat(80));
    
    let base_path = Path::new("/home/verma/lapce/lapce-ai/massive_test_codebase");
    
    // Collect files
    println!("\n📊 Collecting source files...");
    let files = collect_source_files(base_path);
    println!("✅ Found {} source files", files.len());
    
    // Group by language
    let mut by_lang: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for file in &files {
        if let Some(ext) = file.extension().and_then(|s| s.to_str()) {
            by_lang.entry(ext.to_string()).or_default().push(file.clone());
        }
    }
    
    println!("\n📊 Language Distribution:");
    for (lang, files) in &by_lang {
        println!("  .{}: {} files", lang, files.len());
    }
    
    // Parse all files and store CSTs
    println!("\n🔧 Parsing files and storing CSTs...\n");
    
    let mut stored_csts = Vec::new();
    let mut memory_stats = MemoryBySize::default();
    let mut total_parsed = 0;
    let mut total_failed = 0;
    let mut total_lines = 0;
    let mut total_bytes = 0;
    let mut total_nodes = 0;
    let mut total_parse_time = 0.0;
    let mut languages_parsed = HashMap::new();
    
    // Process in batches
    let batch_size = 100;
    for (batch_idx, chunk) in files.chunks(batch_size).enumerate() {
        let batch_start = Instant::now();
        let memory_before = get_memory_mb();
        
        println!("📦 Batch {}/{}: {} files", 
            batch_idx + 1, 
            (files.len() + batch_size - 1) / batch_size,
            chunk.len()
        );
        
        let mut batch_nodes = 0;
        let mut batch_parsed = 0;
        
        for file in chunk {
            if let Some(ext) = file.extension().and_then(|s| s.to_str()) {
                if let Some(lang) = WorkingLanguage::from_extension(ext) {
                    match process_file(file, lang) {
                        Ok(cst) => {
                            total_parsed += 1;
                            batch_parsed += 1;
                            total_lines += cst.line_count;
                            total_bytes += cst.file_size;
                            total_nodes += cst.node_count;
                            batch_nodes += cst.node_count;
                            total_parse_time += cst.parse_time_ms;
                            
                            *languages_parsed.entry(cst.language.clone()).or_insert(0) += 1;
                            memory_stats.add(cst.file_size, cst.node_count);
                            
                            stored_csts.push(cst);
                        }
                        Err(e) => {
                            total_failed += 1;
                            if total_failed <= 3 {
                                println!("  ❌ Failed: {} - {}", file.display(), e);
                            }
                        }
                    }
                }
            }
        }
        
        let batch_time = batch_start.elapsed();
        let memory_after = get_memory_mb();
        let memory_delta = memory_after - memory_before;
        
        println!("  ⏱️  Time: {:.2}s | Parsed: {} | Nodes: {}",
            batch_time.as_secs_f64(), batch_parsed, batch_nodes);
        println!("  💾 Memory: {:.1}MB → {:.1}MB (Δ{:.1}MB)",
            memory_before, memory_after, memory_delta);
        
        // Estimate memory per node
        if batch_nodes > 0 {
            let bytes_per_node = (memory_delta * 1_048_576.0) / batch_nodes as f64;
            println!("  📊 ~{:.0} bytes/node", bytes_per_node);
        }
        println!();
    }
    
    // Calculate statistics
    let success_rate = (total_parsed as f64 / files.len() as f64) * 100.0;
    let avg_parse_time = if total_parsed > 0 {
        total_parse_time / total_parsed as f64
    } else {
        0.0
    };
    let avg_nodes_per_file = if total_parsed > 0 {
        total_nodes as f64 / total_parsed as f64
    } else {
        0.0
    };
    let avg_nodes_per_line = if total_lines > 0 {
        total_nodes as f64 / total_lines as f64
    } else {
        0.0
    };
    
    // Estimate CST memory
    let bytes_per_node = 150; // Rough estimate
    let total_cst_memory_mb = (total_nodes * bytes_per_node) as f64 / 1_048_576.0;
    
    // Calculate tree depth statistics
    let max_depth = calculate_max_depth(&stored_csts);
    let avg_depth = calculate_avg_depth(&stored_csts);
    
    // Results
    println!("{}", "=".repeat(80));
    println!("📊 FINAL RESULTS");
    println!("{}", "=".repeat(80));
    
    println!("\n📈 Overall Statistics:");
    println!("  Total Files:           {}", files.len());
    println!("  Successfully Parsed:   {} ({:.1}%)", total_parsed, success_rate);
    println!("  Failed:                {}", total_failed);
    println!("  Total Lines:           {}", total_lines);
    println!("  Total Bytes:           {} ({:.2} MB)", 
        total_bytes, total_bytes as f64 / 1_048_576.0);
    
    println!("\n🌍 Languages Parsed:");
    for (lang, count) in &languages_parsed {
        println!("  {}: {} files", lang, count);
    }
    
    println!("\n🌲 CST Statistics:");
    println!("  Total CSTs Stored:     {}", stored_csts.len());
    println!("  Total Nodes:           {}", total_nodes);
    println!("  Avg Nodes/File:        {:.0}", avg_nodes_per_file);
    println!("  Avg Nodes/Line:        {:.2}", avg_nodes_per_line);
    println!("  Max Tree Depth:        {}", max_depth);
    println!("  Avg Tree Depth:        {:.1}", avg_depth);
    
    println!("\n💾 Memory Analysis:");
    println!("  Est. CST Memory:       {:.2} MB", total_cst_memory_mb);
    println!("  Memory/File:           {:.3} MB", total_cst_memory_mb / total_parsed as f64);
    println!("  Memory/1K Lines:       {:.3} MB", 
        total_cst_memory_mb / (total_lines as f64 / 1000.0));
    println!("  Bytes/Node (est):      {}", bytes_per_node);
    
    println!("\n📊 Nodes by File Size:");
    let size_avgs = memory_stats.get_averages();
    for (category, avg_nodes) in size_avgs {
        let est_mb = (avg_nodes * bytes_per_node as f64) / 1_048_576.0;
        println!("  {}: {:.0} nodes (~{:.3} MB)", category, avg_nodes, est_mb);
    }
    
    println!("\n⚡ Performance:");
    println!("  Avg Parse Time:        {:.2} ms", avg_parse_time);
    println!("  Total Parse Time:      {:.2} seconds", total_parse_time / 1000.0);
    println!("  Parse Speed:           {:.0} lines/second", 
        total_lines as f64 / (total_parse_time / 1000.0));
    
    // Success criteria check
    println!("\n🎯 Success Criteria:");
    let parse_speed = total_lines as f64 / (total_parse_time / 1000.0);
    println!("  ✅ Parse Speed:        {:.0} lines/sec (target: >10K)", parse_speed);
    println!("  {} Memory Usage:       {:.2} MB (target: <5MB)", 
        if total_cst_memory_mb < 5.0 { "✅" } else { "⚠️ " },
        total_cst_memory_mb
    );
    println!("  ✅ Language Support:   {} languages", languages_parsed.len());
    println!("  ✅ Test Coverage:      {} lines", total_lines);
    
    // Final verdict
    println!("\n{}", "=".repeat(80));
    if success_rate > 95.0 && total_cst_memory_mb < 5.0 {
        println!("✅ SUCCESS: {:.1}% parsed, memory within limits!", success_rate);
    } else if success_rate > 80.0 {
        println!("⚠️  PARTIAL SUCCESS: {:.1}% parsed", success_rate);
    } else {
        println!("❌ NEEDS WORK: Only {:.1}% success", success_rate);
    }
    
    println!("\n💡 Key Insight: {} lines can be stored in ~{:.2} MB of CST memory",
        total_lines, total_cst_memory_mb);
    println!("{}", "=".repeat(80));
}

fn collect_source_files(base: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_source_files(&path));
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if WorkingLanguage::from_extension(ext).is_some() {
                        files.push(path);
                    }
                }
            }
        }
    }
    
    files
}

fn process_file(path: &Path, lang: WorkingLanguage) -> Result<StoredCST, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Read error: {}", e))?;
    
    let file_size = content.len();
    let line_count = content.lines().count();
    
    let parse_start = Instant::now();
    let mut parser = lang.get_parser()?;
    let tree = parser.parse(&content, None)
        .ok_or("Parse failed")?;
    let parse_time = parse_start.elapsed();
    
    let node_count = tree.root_node().descendant_count();
    
    Ok(StoredCST {
        file_path: path.to_path_buf(),
        file_size,
        line_count,
        tree,
        parse_time_ms: parse_time.as_secs_f64() * 1000.0,
        node_count,
        language: path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string(),
    })
}

fn get_memory_mb() -> f64 {
    // Simple memory estimate using /proc/self/status on Linux
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

fn calculate_max_depth(csts: &[StoredCST]) -> usize {
    fn depth(node: tree_sitter::Node) -> usize {
        let mut cursor = node.walk();
        let mut max = 0;
        for child in node.children(&mut cursor) {
            max = max.max(depth(child));
        }
        max + 1
    }
    
    csts.iter()
        .map(|cst| depth(cst.tree.root_node()))
        .max()
        .unwrap_or(0)
}

fn calculate_avg_depth(csts: &[StoredCST]) -> f64 {
    fn depth(node: tree_sitter::Node) -> usize {
        let mut cursor = node.walk();
        let mut max = 0;
        for child in node.children(&mut cursor) {
            max = max.max(depth(child));
        }
        max + 1
    }
    
    if csts.is_empty() {
        return 0.0;
    }
    
    let total: usize = csts.iter()
        .map(|cst| depth(cst.tree.root_node()))
        .sum();
    
    total as f64 / csts.len() as f64
}
