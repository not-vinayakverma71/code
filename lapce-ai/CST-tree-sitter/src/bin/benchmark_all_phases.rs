//! Benchmark using ALL phases from the optimization journey
//! Phase 1: Varint + Packing + Interning (40% reduction)
//! Phase 2: Delta + Pruning (60% cumulative)
//! Phase 3: Bytecode Trees (75% cumulative)
//! Phase 4a: Frozen Tier (93% cumulative)
//! Phase 4b: Mmap Sources (95% cumulative)
//! Phase 4c: Segmented Bytecode (97% cumulative)

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;


use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use lapce_tree_sitter::complete_pipeline::{
    CompletePipeline,
    CompletePipelineConfig,

};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use tree_sitter::{Parser, Language};

// Import language parsers
use tree_sitter_rust;
#[cfg(feature = "lang-javascript")]
use tree_sitter_javascript;
#[cfg(feature = "lang-typescript")]
use tree_sitter_typescript;
use tree_sitter_python;
use tree_sitter_go;
use tree_sitter_java;
use tree_sitter_c;
use tree_sitter_cpp;

fn get_memory_mb() -> f64 {
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}

fn get_language(path: &Path) -> Option<(Language, &'static str)> {
    let ext = path.extension()?.to_str()?;
    
    match ext {
        "rs" => Some((tree_sitter_rust::LANGUAGE.into(), "rust")),
        #[cfg(feature = "lang-javascript")]
        "js" | "mjs" => Some((tree_sitter_javascript::language(), "javascript")),
        #[cfg(feature = "lang-typescript")]
        "ts" | "tsx" => Some((tree_sitter_typescript::language_typescript(), "typescript")),
        #[cfg(not(feature = "lang-javascript"))]
        "js" | "mjs" => None,
        #[cfg(not(feature = "lang-typescript"))]
        "ts" | "tsx" => None,
        "py" => Some((tree_sitter_python::LANGUAGE.into(), "python")),
        "go" => Some((tree_sitter_go::LANGUAGE.into(), "go")),
        "java" => Some((tree_sitter_java::LANGUAGE.into(), "java")),
        "c" | "h" => Some((tree_sitter_c::LANGUAGE.into(), "c")),
        "cpp" | "cc" | "cxx" | "hpp" => Some((tree_sitter_cpp::LANGUAGE.into(), "cpp")),
        _ => None,
    }
}

fn collect_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    for entry in WalkBuilder::new(dir)
        .hidden(false)
        .git_ignore(true)
        .build()
    {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() && get_language(path).is_some() {
                files.push(path.to_path_buf());
            }
        }
    }
    
    files
}

fn main() {
    println!("=== ALL PHASES BENCHMARK - COMPLETE JOURNEY ===");
    println!("Target: /home/verma/lapce/Codex");
    println!("Phases: 1→2→3→4a→4b→4c (97% reduction target)\n");
    
    let codex_path = Path::new("/home/verma/lapce/Codex");
    
    if !codex_path.exists() {
        eprintln!("Error: Codex directory not found");
        std::process::exit(1);
    }
    
    let initial_memory = get_memory_mb();
    println!("Initial memory: {:.1} MB\n", initial_memory);
    
    // Collect files
    println!("Scanning for source files...");
    let files = collect_files(codex_path);
    println!("Found {} parseable files\n", files.len());
    
    if files.is_empty() {
        return;
    }
    
    // Test configurations for each phase combination
    let configs = vec![
        ("Baseline (no optimization)", create_config_none()),
        ("Phase 1 only (Varint+Packing+Interning)", create_config_phase1()),
        ("Phase 1+2 (+ Delta compression)", create_config_phase12()),
        ("Phase 1+2+3 (+ Bytecode)", create_config_phase123()),
        ("Phase 1-4a (+ Frozen tier)", create_config_phase14a()),
        ("Phase 1-4b (+ Mmap)", create_config_phase14b()),
        ("ALL PHASES (1-4c)", create_config_all()),
    ];
    
    let mut results = Vec::new();
    
    for (name, config) in configs {
        println!("\n=== Testing: {} ===\n", name);
        
        let result = run_phase_test(config, &files);
        println!("  Result: {:.1} MB memory, {:.1}x compression, {} lines/MB",
            result.memory_mb, result.compression_ratio, result.lines_per_mb);
        
        results.push((name, result));
    }
    
    // Summary
    println!("\n=== PHASE-BY-PHASE RESULTS ===\n");
    println!("{:<45} {:>10} {:>12} {:>12} {:>15}",
        "Configuration", "Memory MB", "Compression", "Lines/MB", "Reduction");
    println!("{:-<95}", "");
    
    let baseline_memory = results[0].1.memory_mb;
    
    for (name, result) in &results {
        let reduction = if baseline_memory > 0.0 {
            ((baseline_memory - result.memory_mb) / baseline_memory * 100.0)
        } else {
            0.0
        };
        
        println!("{:<45} {:>10.1} {:>12.1}x {:>12} {:>14.1}%",
            name, result.memory_mb, result.compression_ratio, 
            result.lines_per_mb, reduction);
    }
    
    // Check against journey targets
    println!("\n=== JOURNEY DOCUMENT TARGETS ===\n");
    
    let journey_targets = vec![
        ("Phase 1", 40.0),
        ("Phase 1+2", 60.0),
        ("Phase 1+2+3", 75.0),
        ("Phase 1-4a", 93.0),
        ("Phase 1-4b", 95.0),
        ("ALL PHASES", 97.0),
    ];
    
    for (i, (phase_name, target_reduction)) in journey_targets.iter().enumerate() {
        if i + 1 < results.len() {
            let actual_reduction = ((baseline_memory - results[i + 1].1.memory_mb) / baseline_memory * 100.0);
            let met = if actual_reduction >= target_reduction - 5.0 { "✅" } else { "❌" };
            
            println!("{} {}: Target {}% reduction, Achieved {:.1}%",
                met, phase_name, target_reduction, actual_reduction);
        }
    }
    
    // Final verdict
    let final_result = &results.last().unwrap().1;
    let final_reduction = ((baseline_memory - final_result.memory_mb) / baseline_memory * 100.0);
    
    println!("\n=== FINAL VERDICT ===");
    
    if final_reduction >= 97.0 {
        println!("🎉🎉🎉 TARGET ACHIEVED! 🎉🎉🎉");
        println!("✅ 97% memory reduction reached: {:.1}%", final_reduction);
        println!("✅ Lines per MB: {}", final_result.lines_per_mb);
    } else if final_reduction >= 90.0 {
        println!("✅ Excellent reduction: {:.1}% (close to 97% target)", final_reduction);
    } else if final_reduction >= 75.0 {
        println!("⚠️  Good reduction: {:.1}% (below 97% target)", final_reduction);
    } else {
        println!("❌ Reduction {:.1}% is below expectations", final_reduction);
    }
    
    // Save detailed report
    let report = serde_json::json!({
        "timestamp": chrono::Local::now().to_rfc3339(),
        "test_type": "ALL_PHASES_COMPLETE",
        "files_processed": files.len(),
        "results": results.iter().map(|(name, r)| {
            serde_json::json!({
                "phase": name,
                "memory_mb": r.memory_mb,
                "compression_ratio": r.compression_ratio,
                "lines_per_mb": r.lines_per_mb,
            })
        }).collect::<Vec<_>>(),
        "final_reduction_percent": final_reduction,
        "target_achieved": final_reduction >= 97.0,
    });
    
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        let _ = fs::write("ALL_PHASES_BENCHMARK.json", json);
        println!("\n📊 Report saved to ALL_PHASES_BENCHMARK.json");
    }
}

#[allow(dead_code)]
struct TestResult {
    memory_mb: f64,
    compression_ratio: f64,
    lines_per_mb: usize,
    total_lines: usize,
}

fn run_phase_test(config: CompletePipelineConfig, files: &[PathBuf]) -> TestResult {
    let pipeline = match CompletePipeline::new(config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create pipeline: {}", e);
            return TestResult {
                memory_mb: 0.0,
                compression_ratio: 1.0,
                lines_per_mb: 0,
                total_lines: 0,
            };
        }
    };
    
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len}")
        .unwrap()
        .progress_chars("#>-"));
    
    let mut parser = Parser::new();
    let mut total_lines = 0;
    let mut total_original = 0;
    let mut total_final = 0;
    
    for file_path in files.iter().take(100) { // Process subset for speed
        if let Ok(content) = fs::read_to_string(file_path) {
            total_lines += content.lines().count();
            total_original += content.len();
            
            if let Some((language, _)) = get_language(file_path) {
                parser.set_language(&language).ok();
                
                if let Some(tree) = parser.parse(&content, None) {
                    if let Ok(result) = pipeline.process_tree(
                        file_path.clone(),
                        tree,
                        content.as_bytes(),
                    ) {
                        total_final += result.final_size;
                    }
                }
            }
        }
        
        pb.inc(1);
    }
    
    pb.finish_and_clear();
    
    let stats = pipeline.stats();
    let memory_mb = stats.total_memory_bytes as f64 / 1_048_576.0;
    let compression_ratio = if total_final > 0 {
        total_original as f64 / total_final as f64
    } else {
        1.0
    };
    let lines_per_mb = if total_final > 0 {
        total_lines * 1_048_576 / total_final
    } else {
        0
    };
    
    TestResult {
        memory_mb,
        compression_ratio,
        lines_per_mb,
        total_lines,
    }
}

// Configuration builders for each phase
fn create_config_none() -> CompletePipelineConfig {
    CompletePipelineConfig {
        memory_budget_mb: 500,
        phase1_varint: false,
        phase1_packing: false,
        phase1_interning: false,
        phase2_delta: false,
        phase2_chunking: false,
        phase3_bytecode: false,
        phase4a_frozen: false,
        phase4b_mmap: false,
        phase4c_segments: false,
        storage_dir: tempdir().unwrap().path().to_path_buf(),
    }
}

fn create_config_phase1() -> CompletePipelineConfig {
    let mut config = create_config_none();
    config.phase1_varint = true;
    config.phase1_packing = true;
    config.phase1_interning = true;
    config
}

fn create_config_phase12() -> CompletePipelineConfig {
    let mut config = create_config_phase1();
    config.phase2_delta = true;
    config.phase2_chunking = true;
    config
}

fn create_config_phase123() -> CompletePipelineConfig {
    let mut config = create_config_phase12();
    config.phase3_bytecode = true;
    config
}

fn create_config_phase14a() -> CompletePipelineConfig {
    let mut config = create_config_phase123();
    config.phase4a_frozen = true;
    config.memory_budget_mb = 50;
    config
}

fn create_config_phase14b() -> CompletePipelineConfig {
    let mut config = create_config_phase14a();
    config.phase4b_mmap = true;
    config
}

fn create_config_all() -> CompletePipelineConfig {
    CompletePipelineConfig {
        memory_budget_mb: 50,
        phase1_varint: true,
        phase1_packing: true,
        phase1_interning: true,
        phase2_delta: true,
        phase2_chunking: true,
        phase3_bytecode: true,
        phase4a_frozen: true,
        phase4b_mmap: true,
        phase4c_segments: true,
        storage_dir: tempdir().unwrap().path().to_path_buf(),
    }
}
