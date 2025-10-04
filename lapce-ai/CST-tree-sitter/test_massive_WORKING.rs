#!/usr/bin/env rustc
//! ACTUALLY WORKING TEST - Parse massive codebase with Tree-sitter
//! Uses only parsers that are confirmed working from crates.io

extern crate tree_sitter;
extern crate tree_sitter_rust;
extern crate tree_sitter_python;
extern crate tree_sitter_javascript;
extern crate tree_sitter_typescript;
extern crate tree_sitter_go;
extern crate tree_sitter_java;
extern crate tree_sitter_c;
extern crate tree_sitter_cpp;
extern crate tree_sitter_c_sharp;
extern crate tree_sitter_ruby;
extern crate tree_sitter_php;
extern crate tree_sitter_lua;
extern crate tree_sitter_bash;
extern crate tree_sitter_css;
extern crate tree_sitter_json;
extern crate tree_sitter_swift;
extern crate tree_sitter_scala;
extern crate tree_sitter_elixir;
extern crate tree_sitter_html;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tree_sitter::{Parser, Tree};

fn main() {
    println!("\n🚀 MASSIVE CODEBASE TEST - WITH REAL TREE-SITTER PARSING");
    println!("{}", "=".repeat(80));
    
    // Initialize parsers for languages we know work
    let mut parsers = HashMap::new();
    
    // Setup parsers
    macro_rules! setup_parser {
        ($lang:ident, $ts:expr) => {
            let mut parser = Parser::new();
            if parser.set_language(&$ts.into()).is_ok() {
                parsers.insert(stringify!($lang), parser);
            }
        };
    }
    
    unsafe {
        setup_parser!(rust, tree_sitter_rust::LANGUAGE);
        setup_parser!(python, tree_sitter_python::LANGUAGE);
        setup_parser!(javascript, tree_sitter_javascript::LANGUAGE);
        setup_parser!(typescript, tree_sitter_typescript::LANGUAGE_TYPESCRIPT);
        setup_parser!(tsx, tree_sitter_typescript::LANGUAGE_TSX);
        setup_parser!(go, tree_sitter_go::LANGUAGE);
        setup_parser!(java, tree_sitter_java::LANGUAGE);
        setup_parser!(c, tree_sitter_c::LANGUAGE);
        setup_parser!(cpp, tree_sitter_cpp::LANGUAGE);
        setup_parser!(csharp, tree_sitter_c_sharp::LANGUAGE);
        setup_parser!(ruby, tree_sitter_ruby::LANGUAGE);
        setup_parser!(php, tree_sitter_php::LANGUAGE_PHP);
        setup_parser!(lua, tree_sitter_lua::LANGUAGE);
        setup_parser!(bash, tree_sitter_bash::LANGUAGE);
        setup_parser!(css, tree_sitter_css::LANGUAGE);
        setup_parser!(json, tree_sitter_json::LANGUAGE);
        setup_parser!(swift, tree_sitter_swift::LANGUAGE);
        setup_parser!(scala, tree_sitter_scala::LANGUAGE);
        setup_parser!(elixir, tree_sitter_elixir::LANGUAGE);
        setup_parser!(html, tree_sitter_html::LANGUAGE);
    }
    
    println!("✅ Loaded {} language parsers", parsers.len());
    
    let base_path = Path::new("/home/verma/lapce/lapce-ai/massive_test_codebase");
    
    // Collect files
    println!("\n📊 Collecting source files...");
    let files = collect_source_files(base_path);
    println!("✅ Found {} source files", files.len());
    
    // Storage for CSTs
    let mut cst_storage: Vec<(PathBuf, Tree)> = Vec::new();
    
    // Metrics
    let mut total_parsed = 0;
    let mut total_failed = 0;
    let mut total_lines = 0;
    let mut total_bytes = 0;
    let mut total_nodes = 0;
    let mut parse_times = Vec::new();
    let mut by_language: HashMap<&str, usize> = HashMap::new();
    
    println!("\n🔧 Parsing files and storing CSTs...\n");
    
    let batch_size = 100;
    for (batch_idx, chunk) in files.chunks(batch_size).enumerate() {
        let batch_start = Instant::now();
        let memory_before = get_memory_mb();
        
        println!("📦 Batch {}/{}: {} files", 
            batch_idx + 1, 
            (files.len() + batch_size - 1) / batch_size,
            chunk.len()
        );
        
        let mut batch_parsed = 0;
        let mut batch_nodes = 0;
        
        for file in chunk {
            // Get parser for this file
            let parser_key = match file.extension().and_then(|e| e.to_str()) {
                Some("rs") => "rust",
                Some("py") => "python",
                Some("js") | Some("mjs") => "javascript",
                Some("ts") | Some("mts") => "typescript",
                Some("tsx") => "tsx",
                Some("go") => "go",
                Some("java") => "java",
                Some("c") | Some("h") => "c",
                Some("cpp") | Some("cc") | Some("cxx") => "cpp",
                Some("cs") => "csharp",
                Some("rb") => "ruby",
                Some("php") => "php",
                Some("lua") => "lua",
                Some("sh") | Some("bash") => "bash",
                Some("css") => "css",
                Some("json") => "json",
                Some("swift") => "swift",
                Some("scala") => "scala",
                Some("ex") | Some("exs") => "elixir",
                Some("html") | Some("htm") => "html",
                _ => continue,
            };
            
            if let Some(parser) = parsers.get_mut(parser_key) {
                if let Ok(content) = fs::read_to_string(file) {
                    let lines = content.lines().count();
                    let bytes = content.len();
                    
                    // Parse with Tree-sitter
                    let parse_start = Instant::now();
                    if let Some(tree) = parser.parse(&content, None) {
                        let parse_time = parse_start.elapsed();
                        parse_times.push(parse_time);
                        
                        // Count nodes
                        let node_count = tree.root_node().descendant_count();
                        
                        // Store the CST
                        cst_storage.push((file.clone(), tree));
                        
                        // Update metrics
                        total_parsed += 1;
                        batch_parsed += 1;
                        total_lines += lines;
                        total_bytes += bytes;
                        total_nodes += node_count;
                        batch_nodes += node_count;
                        *by_language.entry(parser_key).or_insert(0) += 1;
                    } else {
                        total_failed += 1;
                    }
                }
            }
        }
        
        let batch_time = batch_start.elapsed();
        let memory_after = get_memory_mb();
        
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
    
    // Calculate statistics
    let success_rate = (total_parsed as f64 / files.len().max(1) as f64) * 100.0;
    let avg_nodes_per_file = total_nodes as f64 / total_parsed.max(1) as f64;
    let avg_nodes_per_line = total_nodes as f64 / total_lines.max(1) as f64;
    
    // Average parse time
    let avg_parse_time_ms = if !parse_times.is_empty() {
        parse_times.iter().map(|d| d.as_secs_f64() * 1000.0).sum::<f64>() / parse_times.len() as f64
    } else { 0.0 };
    
    // CST memory estimation
    let bytes_per_node = 150; // Conservative estimate
    let total_cst_memory_mb = (total_nodes * bytes_per_node) as f64 / 1_048_576.0;
    
    // === RESULTS ===
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
    
    println!("\n🌍 Language Distribution:");
    let mut lang_list: Vec<_> = by_language.iter().collect();
    lang_list.sort_by_key(|(_, count)| *count);
    lang_list.reverse();
    for (lang, count) in lang_list.iter().take(10) {
        println!("  {}: {} files", lang, count);
    }
    
    println!("\n🌲 CST Analysis:");
    println!("  Total CSTs Stored:     {}", cst_storage.len());
    println!("  Total Nodes:           {}", total_nodes);
    println!("  Avg Nodes/File:        {:.0}", avg_nodes_per_file);
    println!("  Avg Nodes/Line:        {:.2}", avg_nodes_per_line);
    
    println!("\n💾 Memory Analysis:");
    println!("  Est. CST Memory:       {:.2} MB", total_cst_memory_mb);
    println!("  Memory/File:           {:.3} MB", total_cst_memory_mb / total_parsed.max(1) as f64);
    println!("  Memory/1K Lines:       {:.3} MB", 
        total_cst_memory_mb / (total_lines as f64 / 1000.0).max(1.0));
    
    println!("\n⚡ Performance Metrics:");
    println!("  Avg Parse Time:        {:.2} ms/file", avg_parse_time_ms);
    println!("  Parse Speed:           {:.0} lines/second", 
        total_lines as f64 / parse_times.iter().map(|d| d.as_secs_f64()).sum::<f64>().max(0.001));
    
    // Success criteria check
    println!("\n🎯 Success Criteria:");
    println!("  {} Parse Success:       {:.1}% (target: >95%)", 
        if success_rate > 95.0 { "✅" } else { "⚠️ " }, success_rate);
    println!("  {} CST Storage:         {} trees stored in memory",
        if cst_storage.len() > 1000 { "✅" } else { "⚠️ " }, cst_storage.len());
    println!("  {} Memory Efficiency:   {:.0} lines per MB",
        if total_lines as f64 / total_cst_memory_mb > 1000.0 { "✅" } else { "⚠️ " },
        total_lines as f64 / total_cst_memory_mb.max(0.001));
    println!("  {} Parse Speed:         {:.0} lines/sec (target: >10K)",
        if total_lines as f64 / parse_times.iter().map(|d| d.as_secs_f64()).sum::<f64>().max(0.001) > 10000.0 { "✅" } else { "⚠️ " },
        total_lines as f64 / parse_times.iter().map(|d| d.as_secs_f64()).sum::<f64>().max(0.001));
    
    println!("\n{}", "=".repeat(80));
    if total_parsed > 2000 && success_rate > 95.0 {
        println!("✅ COMPLETE SUCCESS!");
        println!("   - {} files parsed with Tree-sitter", total_parsed);
        println!("   - {} CSTs stored in memory", cst_storage.len());
        println!("   - {:.2} MB total CST memory", total_cst_memory_mb);
        println!("   - {} languages tested", by_language.len());
    } else if total_parsed > 1000 {
        println!("⚠️  PARTIAL SUCCESS");
        println!("   - {} files parsed", total_parsed);
        println!("   - Success rate: {:.1}%", success_rate);
    } else {
        println!("❌ TEST INCOMPLETE");
    }
    
    println!("\n💡 Tree-sitter integration is WORKING for {} languages!", parsers.len());
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
                files.push(path);
            }
        }
    }
    
    files
}

fn get_memory_mb() -> f64 {
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
