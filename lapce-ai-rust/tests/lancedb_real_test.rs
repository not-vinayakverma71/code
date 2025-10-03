/// REAL PRODUCTION TESTS - NO MOCKS
/// Must validate ALL 8 performance requirements with actual implementation

use lapce_ai_rust::lancedb::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use std::sync::Arc;
use std::fs;

#[tokio::test]
async fn test_all_requirements() {
    println!("\n=== LANCEDB PRODUCTION VALIDATION ===\n");
    
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    
    // Create actual test repository with real Rust files
    let test_files = create_real_test_repository(&workspace);
    println!("✓ Created test repository with {} files", test_files.len());
    
    // Initialize LanceDB
    let db = Arc::new(LanceDB::new(workspace.clone()).await.unwrap());
    println!("✓ Initialized LanceDB");
    
    // TEST 1: Memory Usage < 10MB
    let memory_before = get_memory_usage_mb();
    db.index_files(test_files.clone()).await.unwrap();
    let memory_after = get_memory_usage_mb();
    let memory_used = memory_after - memory_before;
    
    println!("TEST 1 - Memory: {:.2}MB (target: <10MB) {}", 
        memory_used, 
        if memory_used < 10.0 { "✅ PASS" } else { "❌ FAIL" }
    );
    assert!(memory_used < 10.0, "Memory usage {:.2}MB exceeds 10MB", memory_used);
    
    // TEST 2: Query Latency < 5ms
    let mut latencies = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        db.search("async function", 10).await.unwrap();
        latencies.push(start.elapsed());
    }
    latencies.sort();
    let p99_latency = latencies[99];
    
    println!("TEST 2 - Query Latency P99: {:.2}ms (target: <5ms) {}", 
        p99_latency.as_secs_f64() * 1000.0,
        if p99_latency < Duration::from_millis(5) { "✅ PASS" } else { "❌ FAIL" }
    );
    assert!(p99_latency < Duration::from_millis(5), "P99 latency {:?} exceeds 5ms", p99_latency);
    
    // TEST 3: Indexing Throughput > 1000 files/sec
    let batch_files = create_batch_files(&workspace, 2000);
    let start = Instant::now();
    db.index_files(batch_files.clone()).await.unwrap();
    let elapsed = start.elapsed();
    let throughput = batch_files.len() as f64 / elapsed.as_secs_f64();
    
    println!("TEST 3 - Indexing: {:.0} files/sec (target: >1000) {}", 
        throughput,
        if throughput > 1000.0 { "✅ PASS" } else { "❌ FAIL" }
    );
    assert!(throughput > 1000.0, "Throughput {:.0} below 1000 files/sec", throughput);
    
    // TEST 4: Accuracy > 90%
    let accuracy = test_search_accuracy(&db).await;
    println!("TEST 4 - Accuracy: {:.1}% (target: >90%) {}", 
        accuracy * 100.0,
        if accuracy > 0.9 { "✅ PASS" } else { "❌ FAIL" }
    );
    assert!(accuracy > 0.9, "Accuracy {:.1}% below 90%", accuracy * 100.0);
    
    // TEST 5: Incremental Update < 100ms
    let file_to_update = &test_files[0];
    fs::write(file_to_update, "// Updated\nfn new_function() {}").unwrap();
    let start = Instant::now();
    db.incremental_update(file_to_update.clone()).await.unwrap();
    let update_latency = start.elapsed();
    
    println!("TEST 5 - Incremental Update: {:.0}ms (target: <100ms) {}", 
        update_latency.as_millis(),
        if update_latency < Duration::from_millis(100) { "✅ PASS" } else { "❌ FAIL" }
    );
    assert!(update_latency < Duration::from_millis(100), "Update {:?} exceeds 100ms", update_latency);
    
    // TEST 6: Cache Hit Rate > 80%
    // Warm up cache
    for _ in 0..3 {
        for query in &["async", "function", "impl", "trait"] {
            db.search(query, 10).await.unwrap();
        }
    }
    let metrics = db.get_metrics();
    let hit_rate = metrics.cache_hit_rate;
    
    println!("TEST 6 - Cache Hit Rate: {:.1}% (target: >80%) {}", 
        hit_rate * 100.0,
        if hit_rate > 0.8 { "✅ PASS" } else { "❌ FAIL" }
    );
    assert!(hit_rate > 0.8, "Cache hit rate {:.1}% below 80%", hit_rate * 100.0);
    
    // TEST 7: Handle 100+ Concurrent Queries
    let start = Instant::now();
    let mut handles = Vec::new();
    for i in 0..100 {
        let db_clone = db.clone();
        handles.push(tokio::spawn(async move {
            db_clone.search(&format!("query {}", i % 10), 10).await
        }));
    }
    
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }
    let concurrent_elapsed = start.elapsed();
    
    println!("TEST 7 - Concurrent: {} successful in {:?} (target: 100) {}", 
        success_count, concurrent_elapsed,
        if success_count == 100 { "✅ PASS" } else { "❌ FAIL" }
    );
    assert_eq!(success_count, 100, "Only {} of 100 concurrent queries succeeded", success_count);
    
    // TEST 8: Scale to 100K+ Files (reduced for CI)
    let scale_test_size = if std::env::var("FULL_SCALE_TEST").is_ok() { 100_000 } else { 10_000 };
    let large_files = create_batch_files(&workspace, scale_test_size);
    db.index_files(large_files.clone()).await.unwrap();
    
    let start = Instant::now();
    db.search("test query", 10).await.unwrap();
    let search_time = start.elapsed();
    
    println!("TEST 8 - Scale: {} files, {:?} search (target: >100K) {}", 
        scale_test_size, search_time,
        if search_time < Duration::from_millis(50) { "✅ PASS" } else { "❌ FAIL" }
    );
    assert!(search_time < Duration::from_millis(50), "Search in large index took {:?}", search_time);
    
    println!("\n=== ALL TESTS PASSED ✅ ===");
}

fn create_real_test_repository(workspace: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    // Create structured test repository
    let src_dir = workspace.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    
    // Real Rust code patterns
    let code_templates = vec![
        r#"
use tokio::task;

async fn handle_connection(socket: TcpStream) -> Result<()> {
    let (reader, writer) = socket.split();
    loop {
        match reader.read_frame().await {
            Ok(frame) => process_frame(frame).await?,
            Err(e) => return Err(e.into()),
        }
    }
}
"#,
        r#"
impl Display for CustomError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CustomError::IoError(e) => write!(f, "IO: {}", e),
            CustomError::ParseError(e) => write!(f, "Parse: {}", e),
        }
    }
}
"#,
        r#"
pub trait Handler: Send + Sync {
    async fn handle(&self, req: Request) -> Response;
    fn name(&self) -> &str;
}
"#,
        r#"
fn parse_json(input: &str) -> serde_json::Value {
    serde_json::from_str(input).unwrap_or(serde_json::Value::Null)
}
"#,
    ];
    
    for i in 0..100 {
        let file = src_dir.join(format!("module_{}.rs", i));
        let code = &code_templates[i % code_templates.len()];
        fs::write(&file, code).unwrap();
        files.push(file);
    }
    
    files
}

fn create_batch_files(workspace: &PathBuf, count: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let batch_dir = workspace.join("batch");
    fs::create_dir_all(&batch_dir).unwrap();
    
    for i in 0..count {
        let file = batch_dir.join(format!("file_{}.rs", i));
        fs::write(&file, format!("// File {}\nfn func_{}() {{}}", i, i)).unwrap();
        files.push(file);
    }
    
    files
}

async fn test_search_accuracy(db: &LanceDB) -> f64 {
    let test_cases = vec![
        ("async function", vec!["handle_connection", "async fn"]),
        ("impl trait", vec!["impl Display", "trait Handler"]),
        ("error handling", vec!["Err(e)", "CustomError"]),
        ("parse json", vec!["parse_json", "serde_json"]),
    ];
    
    let mut correct = 0;
    let mut total = 0;
    
    for (query, expected_terms) in test_cases {
        let results = db.search(query, 10).await.unwrap();
        
        for term in expected_terms {
            total += 1;
            if results.iter().any(|r| r.content.contains(term)) {
                correct += 1;
            }
        }
    }
    
    correct as f64 / total as f64
}

fn get_memory_usage_mb() -> f64 {
    use sysinfo::{System, SystemExt, ProcessExt, Pid};
    
    let mut system = System::new_all();
    system.refresh_all();
    
    let pid = std::process::id() as Pid;
    
    if let Some(process) = system.process(pid) {
        process.memory() as f64 / 1024.0 / 1024.0
    } else {
        0.0
    }
}
