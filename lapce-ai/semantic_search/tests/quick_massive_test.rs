// Quick test on massive_test_codebase - get ACTUAL numbers
use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use std::fs;
use std::time::Instant;
use std::path::PathBuf;
use walkdir::WalkDir;

#[tokio::test]
async fn test_massive_codebase_sample() {
    let base_path = "../massive_test_codebase";
    
    // Collect first 500 files for accurate measurement
    let files: Vec<PathBuf> = WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                matches!(ext.to_str().unwrap_or(""), "rs" | "py" | "ts")
            } else {
                false
            }
        })
        .take(500)
        .map(|e| e.path().to_path_buf())
        .collect();
    
    println!("\n=== Massive Codebase Quick Test ===");
    println!("Files collected: {}", files.len());
    
    // Count lines
    let total_lines: usize = files.iter()
        .filter_map(|f| fs::read_to_string(f).ok())
        .map(|content| content.lines().count())
        .sum();
    
    println!("Total lines: {}", total_lines);
    
    // Parse and measure
    let pipeline = CstToAstPipeline::new();
    let start = Instant::now();
    let mut success = 0;
    let mut failed = 0;
    
    for file in &files {
        match pipeline.process_file(file).await {
            Ok(_) => success += 1,
            Err(e) => {
                failed += 1;
                println!("Failed {}: {}", file.display(), e);
            }
        }
    }
    
    let elapsed = start.elapsed();
    let lines_per_sec = total_lines as f64 / elapsed.as_secs_f64();
    
    println!("\n=== ACTUAL Results ===");
    println!("Success: {}/{}", success, files.len());
    println!("Failed: {}", failed);
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    println!("Throughput: {:.0} lines/sec", lines_per_sec);
    println!("Target: >10,000 lines/sec");
    
    if lines_per_sec >= 10000.0 {
        println!("✅ PASS - Exceeds target");
    } else {
        println!("❌ FAIL - Below target");
    }
}
