//! Test memory usage of just parser initialization without storing CSTs

use lapce_tree_sitter::native_parser_manager::NativeParserManager;
use std::sync::Arc;
use tree_sitter::Parser;

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

fn main() {
    println!("=====================================");
    println!(" PARSER-ONLY MEMORY TEST");
    println!(" Testing parser initialization without CST storage");
    println!("=====================================\n");

    // Baseline before anything
    let baseline_rss = get_rss_kb();
    println!("📊 Baseline RSS: {} KB ({:.2} MB)\n", baseline_rss, baseline_rss as f64 / 1024.0);

    // Test 1: NativeParserManager initialization
    println!("🔧 Initializing NativeParserManager...");
    let manager = NativeParserManager::new().unwrap();
    let after_manager = get_rss_kb();
    let manager_memory = after_manager.saturating_sub(baseline_rss);
    println!("  Memory after manager init: +{} KB ({:.2} MB)\n", 
        manager_memory, manager_memory as f64 / 1024.0);

    // Test 2: Create individual parsers for each language
    println!("🔧 Creating individual parsers for major languages...");
    let mut parsers = Vec::new();
    
    // Create parsers for major languages
    let languages = vec![
        ("Rust", tree_sitter_rust::LANGUAGE.into()),
        ("JavaScript", tree_sitter_javascript::language()),
        ("TypeScript", tree_sitter_typescript::language_typescript()),
        ("Python", tree_sitter_python::LANGUAGE.into()),
        ("Go", tree_sitter_go::LANGUAGE.into()),
        ("Java", tree_sitter_java::LANGUAGE.into()),
        ("C", tree_sitter_c::LANGUAGE.into()),
        ("C++", tree_sitter_cpp::LANGUAGE.into()),
        ("Ruby", tree_sitter_ruby::LANGUAGE.into()),
        ("PHP", tree_sitter_php::LANGUAGE_PHP.into()),
    ];

    for (name, lang) in languages {
        let mut parser = Parser::new();
        parser.set_language(&lang).unwrap();
        parsers.push((name, parser));
        
        let current_rss = get_rss_kb();
        let current_memory = current_rss.saturating_sub(baseline_rss);
        println!("  After {} parser: +{} KB total", name, current_memory);
    }

    let after_parsers = get_rss_kb();
    let parsers_memory = after_parsers.saturating_sub(baseline_rss);
    
    // Test 3: Parse a small file without storing CST
    println!("\n🔧 Parsing sample files (without storing CSTs)...");
    let sample_code = "fn main() { println!(\"test\"); }";
    
    for (name, parser) in &mut parsers {
        let tree = parser.parse(sample_code, None);
        if tree.is_some() {
            println!("  {} parsed successfully", name);
        }
        // Tree is dropped immediately, not stored
    }
    
    let after_parsing = get_rss_kb();
    let parsing_memory = after_parsing.saturating_sub(baseline_rss);

    println!("\n=====================================");
    println!(" RESULTS");
    println!("=====================================\n");
    
    println!("💾 Memory Usage Summary:");
    println!("  Baseline: {} KB ({:.2} MB)", baseline_rss, baseline_rss as f64 / 1024.0);
    println!("  After NativeParserManager: +{} KB ({:.2} MB)", 
        manager_memory, manager_memory as f64 / 1024.0);
    println!("  After {} parsers created: +{} KB ({:.2} MB)", 
        parsers.len(), parsers_memory, parsers_memory as f64 / 1024.0);
    println!("  After parsing (no CST storage): +{} KB ({:.2} MB)",
        parsing_memory, parsing_memory as f64 / 1024.0);
    
    println!("\n📊 Per-Component Breakdown:");
    println!("  NativeParserManager overhead: {:.2} MB", manager_memory as f64 / 1024.0);
    println!("  Average per parser: {:.3} MB", 
        (parsers_memory - manager_memory) as f64 / 1024.0 / parsers.len() as f64);
    
    println!("\n✅ Comparison with 5MB requirement:");
    let total_mb = parsing_memory as f64 / 1024.0;
    if total_mb < 5.0 {
        println!("  ✅ PASS: {:.2} MB < 5 MB requirement", total_mb);
    } else {
        println!("  ❌ FAIL: {:.2} MB > 5 MB requirement", total_mb);
    }
    
    println!("\n📝 Note: This is JUST parser initialization.");
    println!("  No CSTs are stored in memory.");
    println!("  Actual CST storage adds ~12.4 KB per file.");
}
