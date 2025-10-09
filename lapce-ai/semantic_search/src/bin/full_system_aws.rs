use lancedb::search::semantic_search_engine::{
    ChunkMetadata,
    SearchConfig,
    SearchFilters,
    SemanticSearchEngine,
};
use lancedb::embeddings::aws_titan_production::AwsTitanProduction;
use lancedb::embeddings::embedder_interface::IEmbedder;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use walkdir::WalkDir;

fn detect_language(path: &std::path::Path) -> Option<String> {
    match path.extension()?.to_str()? {
        "rs" => Some("rust".to_string()),
        "py" => Some("python".to_string()),
        "js" | "jsx" => Some("javascript".to_string()),
        "ts" | "tsx" => Some("typescript".to_string()),
        "go" => Some("go".to_string()),
        "java" => Some("java".to_string()),
        "cpp" | "cc" | "cxx" => Some("cpp".to_string()),
        "c" | "h" => Some("c".to_string()),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║    FULL SYSTEM BENCHMARK (AWS TITAN, 100 FILES, PERSISTENT STORAGE)    ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝\n");

    let start_time = Instant::now();

    // Phase 0: Prepare run directory
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let run_dir = Path::new("runs/aws_100_files/index").join(timestamp);
    fs::create_dir_all(&run_dir)?;
    println!("🗂️  Run directory: {}", run_dir.display());

    // Collect files
    println!("\n📁 Phase 1: Collecting real Rust files (target = 100)");
    println!("════════════════════════════════════════════════════════");
    let code_root = Path::new("/home/verma/lapce/lapce-ai-rust");
    let files = collect_rust_files(code_root, 100);

    if files.len() < 100 {
        eprintln!("❌ Only found {} files. Need at least 100.", files.len());
        return Err("not enough files".into());
    }

    let mut total_bytes = 0usize;
    for (idx, path) in files.iter().enumerate() {
        let metadata = fs::metadata(path)?;
        total_bytes += metadata.len() as usize;
        println!("  {:>3}: {} ({} bytes)", idx + 1, path.display(), metadata.len());
    }
    println!("\n  ✅ Collected {} files (total {:.2} MB)\n", files.len(), total_bytes as f64 / 1_048_576.0);

    // Phase 2: Initialize AWS Titan and semantic search engine
    println!("⚙️  Phase 2: Initializing system components");
    println!("════════════════════════════════════════════════════════");

    let aws_init = Instant::now();
    let embedder: Arc<dyn IEmbedder> = Arc::new(AwsTitanProduction::new_from_config().await?);
    println!("  ✅ AWS Titan ready in {:?}", aws_init.elapsed());

    let config = SearchConfig {
        db_path: run_dir.to_string_lossy().into_owned(),
        cache_size: 5000,
        cache_ttl: 600,
        batch_size: 10,
        max_results: 10,
        min_score: 0.0,
        optimal_batch_size: Some(10),
        max_embedding_dim: Some(1536),
        index_nprobes: Some(4),
    };

    let engine_init = Instant::now();
    let engine = Arc::new(SemanticSearchEngine::new(config.clone(), embedder.clone()).await?);
    println!("  ✅ SemanticSearchEngine ready in {:?}", engine_init.elapsed());

    // Phase 3: Indexing 100 files with AWS Titan embeddings
    println!("\n🔥 Phase 3: Indexing with AWS Titan embeddings (rate-limited)");
    println!("════════════════════════════════════════════════════════");

    let index_start = Instant::now();
    let mem_before = get_memory_mb();

    let mut embeddings_batch = Vec::new();
    let mut metadata_batch = Vec::new();
    let mut aws_time = Duration::ZERO;
    let mut files_indexed = 0usize;

    for (idx, path) in files.iter().enumerate() {
        let content = fs::read_to_string(path)?;
        let truncated = truncate_utf8(&content, 1200);
        let lines = content.lines().count();

        let aws_start = Instant::now();
        let response = embedder
            .create_embeddings(vec![truncated.to_string()], None)
            .await?;
        aws_time += aws_start.elapsed();

        let embedding = response
            .embeddings
            .into_iter()
            .next()
            .ok_or("AWS returned empty embedding")?;

        let mut metadata = HashMap::new();
        metadata.insert("source_size".to_string(), content.len().to_string());
        metadata.insert("full_path".to_string(), path.display().to_string());

        embeddings_batch.push(embedding);
        metadata_batch.push(ChunkMetadata {
            path: path.clone(),
            content: truncated.to_string(),
            start_line: 0,
            end_line: lines,
            language: detect_language(path),
            metadata,
        });

        if embeddings_batch.len() >= config.batch_size {
            engine
                .batch_insert(embeddings_batch.clone(), metadata_batch.clone())
                .await?;
            embeddings_batch.clear();
            metadata_batch.clear();
        }

        files_indexed += 1;
        if idx % 10 == 9 {
            println!(
                "  Indexed {:>3} files (AWS API time: {:.2}s)",
                files_indexed,
                aws_time.as_secs_f64()
            );
        }

        // polite pause (AWS rate limiting)
        sleep(Duration::from_millis(250)).await;
    }

    if !embeddings_batch.is_empty() {
        engine
            .batch_insert(embeddings_batch.clone(), metadata_batch.clone())
            .await?;
    }

    engine.optimize_index().await?;
    let index_time = index_start.elapsed();
    let mem_after = get_memory_mb();

    println!("\n  📊 Indexing summary:");
    println!("  • Files indexed: {}", files_indexed);
    println!("  • Total source size: {:.2} MB", total_bytes as f64 / 1_048_576.0);
    println!("  • Total time: {:.2} s", index_time.as_secs_f64());
    println!("  • AWS API time: {:.2} s", aws_time.as_secs_f64());
    println!("  • Index throughput: {:.2} files/sec", files_indexed as f64 / index_time.as_secs_f64());
    println!("  • Memory before: {:.2} MB", mem_before);
    println!("  • Memory after: {:.2} MB", mem_after);
    println!("  • Memory used: {:.2} MB", mem_after - mem_before);

    // Phase 4: Query performance (cold + warm)
    println!("\n🔍 Phase 4: Query performance evaluation (cold vs warm)");
    println!("════════════════════════════════════════════════════════");

    let queries = vec![
        "async error handling future",
        "database connection pool",
        "parse json configuration",
        "cache optimization",
        "concurrent task execution",
    ];

    let mut cold_times = Vec::new();
    let mut warm_times = Vec::new();

    for query in &queries {
        println!("\n  Query: '{}'", query);
        let cold_start = Instant::now();
        let cold_results = engine
            .search(query, 10, Some(SearchFilters::default()))
            .await?;
        let cold_time = cold_start.elapsed();
        cold_times.push(cold_time);
        println!("    Cold query time: {:?} ({} results)", cold_time, cold_results.len());

        let warm_start = Instant::now();
        let warm_results = engine
            .search(query, 10, Some(SearchFilters::default()))
            .await?;
        let warm_time = warm_start.elapsed();
        warm_times.push(warm_time);
        println!("    Warm query time: {:?} (cache hit: {})", warm_time, warm_results.len());
        println!(
            "    Improvement: {:.2}x",
            cold_time.as_secs_f64() / warm_time.as_secs_f64()
        );
    }

    cold_times.sort();
    warm_times.sort();

    let cold_p50 = percentile(&cold_times, 0.50);
    let cold_p95 = percentile(&cold_times, 0.95);
    let warm_p50 = percentile(&warm_times, 0.50);
    let warm_p95 = percentile(&warm_times, 0.95);

    println!("\n📈 Query latency summary:");
    println!("  • Cold avg: {:.2} s", average_duration(&cold_times).as_secs_f64());
    println!("  • Cold P50: {:.2} s", cold_p50.as_secs_f64());
    println!("  • Cold P95: {:.2} s", cold_p95.as_secs_f64());
    println!("  • Warm avg: {:.2} ms", average_duration(&warm_times).as_secs_f64() * 1000.0);
    println!("  • Warm P50: {:.2} ms", warm_p50.as_secs_f64() * 1000.0);
    println!("  • Warm P95: {:.2} ms", warm_p95.as_secs_f64() * 1000.0);

    // Phase 5: Persist summary to disk
    println!("\n📝 Phase 5: Persisting run summary");
    println!("════════════════════════════════════════════════════════");

    let summary_path = run_dir.join("summary.json");
    let summary = serde_json::json!({
        "files_indexed": files_indexed,
        "total_source_mb": total_bytes as f64 / 1_048_576.0,
        "index_time_sec": index_time.as_secs_f64(),
        "aws_api_time_sec": aws_time.as_secs_f64(),
        "index_throughput_fps": files_indexed as f64 / index_time.as_secs_f64(),
        "memory_before_mb": mem_before,
        "memory_after_mb": mem_after,
        "memory_used_mb": mem_after - mem_before,
        "cold_avg_sec": average_duration(&cold_times).as_secs_f64(),
        "cold_p50_sec": cold_p50.as_secs_f64(),
        "cold_p95_sec": cold_p95.as_secs_f64(),
        "warm_avg_ms": average_duration(&warm_times).as_secs_f64() * 1000.0,
        "warm_p50_ms": warm_p50.as_secs_f64() * 1000.0,
        "warm_p95_ms": warm_p95.as_secs_f64() * 1000.0,
        "run_dir": run_dir,
        "elapsed_total_sec": start_time.elapsed().as_secs_f64()
    });
    fs::write(&summary_path, serde_json::to_string_pretty(&summary)?)?;
    println!("  ✅ Summary written to {}", summary_path.display());

    println!("\n╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║                  FULL SYSTEM BENCHMARK COMPLETED SUCCESSFULLY         ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝");

    Ok(())
}

fn collect_rust_files(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(metadata) = entry.metadata() {
                if metadata.len() > 512 && metadata.len() < 200 * 1024 {
                    files.push(path.to_path_buf());
                    if files.len() >= max_files {
                        break;
                    }
                }
            }
        }
    }
    files
}

fn truncate_utf8(text: &str, max_len: usize) -> &str {
    if text.len() <= max_len {
        return text;
    }
    let mut end = max_len;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

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

fn percentile(values: &[Duration], percentile: f64) -> Duration {
    if values.is_empty() {
        return Duration::from_secs(0);
    }
    let position = ((values.len() - 1) as f64 * percentile).round() as usize;
    values[position]
}

fn average_duration(values: &[Duration]) -> Duration {
    if values.is_empty() {
        return Duration::from_secs(0);
    }
    let total = values
        .iter()
        .fold(Duration::from_secs(0), |acc, d| acc + *d);
    total / values.len() as u32
}
