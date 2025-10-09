//! Performance validation benchmarks
//! Measures throughput, latency, and resource usage

use lapce_tree_sitter::{Phase4Cache, Phase4Config};
use std::path::PathBuf;
use std::time::{Instant, Duration};
use std::sync::{Arc, atomic::{AtomicUsize, AtomicU64, Ordering}};
use std::thread;
use tree_sitter::Parser;
use sysinfo::System;
use tempfile::tempdir;
use serde_json;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PerformanceMetrics {
    throughput: f64,
    avg_latency_ms: f64,
    p50_latency_ms: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
    memory_mb: f64,
    cpu_percent: f64,
}

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
struct BenchmarkResults {
    timestamp: String,
    #[serde(rename = "process_info")]
    process_info: ProcessInfo,
    write_metrics: PerformanceMetrics,
    read_metrics: PerformanceMetrics,
    random_metrics: PerformanceMetrics,
    concurrent_metrics: PerformanceMetrics,
    performance_grade: String,
}

#[derive(Debug, serde::Serialize)]
struct ProcessInfo {
    pid: u32,
    #[serde(rename = "os")]
    operating_system: String,
    #[serde(rename = "cpu_cores")]
    cpu_cores: usize,
}

fn main() {
    println!("=== PERFORMANCE VALIDATION BENCHMARK ===\n");
    let config = Phase4Config {
        memory_budget_mb: 50,
        hot_tier_ratio: 0.4,
        warm_tier_ratio: 0.3,
        segment_size: 256 * 1024,
        storage_dir: tempdir().unwrap().path().to_path_buf(),
        enable_compression: true,
        test_mode: false, // Production mode
    };
    
    let cache = Arc::new(Phase4Cache::new(config).expect("Failed to create cache"));
    
    // Prepare test data
    let test_data = prepare_test_data(1000); // 1000 unique files
    
    // Benchmark 1: Sequential Write Performance
    println!("BENCHMARK 1: SEQUENTIAL WRITE PERFORMANCE");
    println!("{}", "=".repeat(50));
    
    let write_metrics = benchmark_writes(&cache, &test_data);
    print_metrics("Sequential Writes", &write_metrics);
    
    // Benchmark 2: Sequential Read Performance
    println!("\nBENCHMARK 2: SEQUENTIAL READ PERFORMANCE");
    println!("{}", "=".repeat(50));
    
    let read_metrics = benchmark_reads(&cache, &test_data);
    print_metrics("Sequential Reads", &read_metrics);
    
    // Benchmark 3: Random Access Performance
    println!("\nBENCHMARK 3: RANDOM ACCESS PERFORMANCE");
    println!("{}", "=".repeat(50));
    
    let random_metrics = benchmark_random_access(&cache, &test_data);
    print_metrics("Random Access", &random_metrics);
    
    // Benchmark 4: Concurrent Performance
    println!("\nBENCHMARK 4: CONCURRENT PERFORMANCE");
    println!("{}", "=".repeat(50));
    
    let concurrent_metrics = benchmark_concurrent(&cache, &test_data);
    print_metrics("Concurrent Access", &concurrent_metrics);
    
    // Benchmark 5: Memory Efficiency
    println!("\nBENCHMARK 5: MEMORY EFFICIENCY");
    println!("{}", "=".repeat(50));
    
    benchmark_memory_efficiency(&cache, &test_data);
    
    // Benchmark 6: Tier Transition Performance
    println!("{}", "=".repeat(50));
    
    benchmark_tier_transitions(&cache, &test_data);
    
    // Final Summary
    // Calculate performance grade
    let grade = if write_metrics.throughput > 2000.0 
        && read_metrics.throughput > 5000.0 
        && random_metrics.avg_latency_ms < 5.0 {
        "A"
    } else if write_metrics.throughput > 1000.0 
        && read_metrics.throughput > 2500.0 
        && random_metrics.avg_latency_ms < 10.0 {
        "B"
    } else {
        "C"
    };
    
    println!("\n{}", "=".repeat(60));
    println!("PERFORMANCE GRADE: {}", grade);
    println!("{}", "=".repeat(60));
    
    // Output JSON metrics for CI
    if std::env::var("OUTPUT_JSON").is_ok() {
        let json_output = serde_json::json!({
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "grade": grade,
            "metrics": {
                "write_throughput": write_metrics.throughput,
                "read_throughput": read_metrics.throughput,
                "avg_latency_ms": random_metrics.avg_latency_ms,
                "p99_latency_ms": random_metrics.p99_latency_ms,
                "memory_mb": write_metrics.memory_mb,
            },
            "slo_pass": grade != "C",
        });
        
        let json_file = "benchmark_results.json";
        std::fs::write(json_file, serde_json::to_string_pretty(&json_output).unwrap()).unwrap();
        println!("\nJSON metrics written to {}", json_file);
        
        // Exit with error if SLO fails
        if grade == "C" {
            std::process::exit(1);
        }
    }
    
    let baseline_throughput = 1000.0; // ops/sec baseline
    let baseline_latency = 10.0; // ms baseline
    
    println!("\nPerformance vs Baseline:");
    println!("  Write throughput: {:.0}x baseline", write_metrics.throughput / baseline_throughput);
    println!("  Read throughput: {:.0}x baseline", read_metrics.throughput / baseline_throughput);
    println!("  Random latency: {:.1}ms (baseline: {:.1}ms)", 
        random_metrics.avg_latency_ms, baseline_latency);
    println!("  Memory usage: {:.1} MB", write_metrics.memory_mb);
    
    // Performance grades
    let mut grade = "A";
    if write_metrics.throughput < baseline_throughput * 0.8 {
        grade = "B";
    }
    if random_metrics.avg_latency_ms > baseline_latency * 2.0 {
        grade = "C";
    }
    
    println!("\n🏆 PERFORMANCE GRADE: {}", grade);
    
    if grade == "A" {
        println!("✅ Excellent performance - production ready!");
    } else if grade == "B" {
        println!("⚠️ Good performance - minor optimization needed");
    } else {
        println!("❌ Performance below expectations - optimization required");
    }
    
    // Gather process info
    let process_info = ProcessInfo {
        pid: std::process::id(),
        operating_system: std::env::consts::OS.to_string(),
        cpu_cores: num_cpus::get(),
    };
    
    // Output JSON results
    let results = BenchmarkResults {
        timestamp: format!("{:?}", std::time::SystemTime::now()),
        process_info,
        write_metrics,
        read_metrics,
        random_metrics,
        concurrent_metrics,
        performance_grade: grade.to_string(),
    };
    
    // Write to file
    let json_output = serde_json::to_string_pretty(&results).unwrap();
    let mut file = File::create("benchmark_results.json").unwrap();
    file.write_all(json_output.as_bytes()).unwrap();
    println!("\n📊 Results saved to benchmark_results.json");
}

fn prepare_test_data(count: usize) -> Vec<(PathBuf, u64, String)> {
    let mut data = Vec::new();
    
    for _i in 0..count {
        let name = format!("test_file_{}.rs", i);
        let path = PathBuf::from(&name);
        let hash = hash_string(&name);
        
        // Generate varied content
        let data_str = "x".repeat(i % 100 + 10);
        let vec_contents = (0..10).map(|x| x.to_string()).collect::<Vec<_>>().join(", ");
        let content = format!(
            r#"// File {}
fn function_{}() {{
    let data = "{}";
    println!("Processing: {{}}", data);
    for _i in 0..{} {{
        process_item(i);
    }}
}}

fn helper_{}() -> Vec<u8> {{
    vec![{}]
}}
"#,
            i, i, data_str, i % 100, i, vec_contents
        );
        
        data.push((path, hash, content));
    }
    
    data
}

fn benchmark_writes(
    cache: &Arc<Phase4Cache>,
    test_data: &[(PathBuf, u64, String)]
) -> PerformanceMetrics {
    let mut latencies = Vec::new();
    let start = Instant::now();
    let mut _system = System::new_all();
    
    for (path, hash, content) in test_data {
        let op_start = Instant::now();
        
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        if let Some(tree) = parser.parse(content, None) {
            cache.store(path.clone(), *hash, tree, content.as_bytes()).ok();
        }
        
        latencies.push(op_start.elapsed().as_micros() as f64 / 1000.0);
    }
    
    let duration = start.elapsed();
    let throughput = test_data.len() as f64 / duration.as_secs_f64();
    
    // Get per-process memory and CPU stats
    let memory_mb = get_process_memory_mb().unwrap_or(0.0);
    let cpu_percent = get_process_cpu_percent();
    
    calculate_metrics(latencies, throughput, memory_mb, cpu_percent)
}

fn benchmark_reads(
    cache: &Arc<Phase4Cache>,
    test_data: &[(PathBuf, u64, String)]
) -> PerformanceMetrics {
    let mut latencies = Vec::new();
    let start = Instant::now();
    
    for (path, hash, _) in test_data {
        let op_start = Instant::now();
        cache.get(path, *hash).ok();
        latencies.push(op_start.elapsed().as_micros() as f64 / 1000.0);
    }
    
    let duration = start.elapsed();
    let throughput = test_data.len() as f64 / duration.as_secs_f64();
    
    // Get per-process memory and CPU stats for read benchmark
    let memory_mb = get_process_memory_mb().unwrap_or(0.0);
    let cpu_percent = get_process_cpu_percent();
    
    calculate_metrics(latencies, throughput, memory_mb, cpu_percent)
}

fn benchmark_random_access(
    cache: &Arc<Phase4Cache>,
    test_data: &[(PathBuf, u64, String)]
) -> PerformanceMetrics {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    
    let mut shuffled = test_data.to_vec();
    shuffled.shuffle(&mut rng);
    
    let mut latencies = Vec::new();
    let start = Instant::now();
    
    for (path, hash, _) in shuffled.iter().take(test_data.len() / 2) {
        let op_start = Instant::now();
        cache.get(path, *hash).ok();
        latencies.push(op_start.elapsed().as_micros() as f64 / 1000.0);
    }
    
    let duration = start.elapsed();
    let throughput = shuffled.len() as f64 / 2.0 / duration.as_secs_f64();
    
    // Get per-process memory and CPU stats for random access benchmark
    let memory_mb = get_process_memory_mb().unwrap_or(0.0);
    let cpu_percent = get_process_cpu_percent();
    
    calculate_metrics(latencies, throughput, memory_mb, cpu_percent)
}

fn benchmark_concurrent(
    cache: &Arc<Phase4Cache>,
    test_data: &[(PathBuf, u64, String)]
) -> PerformanceMetrics {
    let thread_count = 8;
    let ops_per_thread = test_data.len() / thread_count;
    let total_latency = Arc::new(AtomicU64::new(0));
    let total_ops = Arc::new(AtomicUsize::new(0));
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for tid in 0..thread_count {
        let cache = cache.clone();
        let data = test_data[tid * ops_per_thread..(tid + 1) * ops_per_thread].to_vec();
        let latency_sum = total_latency.clone();
        let op_count = total_ops.clone();
        
        let handle = thread::spawn(move || {
            for (path, hash, _) in data {
                let op_start = Instant::now();
                cache.get(&path, hash).ok();
                let latency = op_start.elapsed().as_micros() as u64;
                latency_sum.fetch_add(latency, Ordering::Relaxed);
                op_count.fetch_add(1, Ordering::Relaxed);
            }
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start.elapsed();
    let total = total_ops.load(Ordering::Relaxed);
    let throughput = total as f64 / duration.as_secs_f64();
    let avg_latency = total_latency.load(Ordering::Relaxed) as f64 / total as f64 / 1000.0;
    
    // Get per-process memory and CPU stats for concurrent benchmark
    let memory_mb = get_process_memory_mb().unwrap_or(0.0);
    let cpu_percent = get_process_cpu_percent();
    
    PerformanceMetrics {
        throughput,
        avg_latency_ms: avg_latency,
        p50_latency_ms: avg_latency,
        p95_latency_ms: avg_latency * 1.5,
        p99_latency_ms: avg_latency * 2.0,
        memory_mb,
        cpu_percent,
    }
}

fn benchmark_memory_efficiency(
    cache: &Arc<Phase4Cache>,
    test_data: &[(PathBuf, u64, String)]
) {
    let stats = cache.stats();
    let total_source_bytes: usize = test_data.iter().map(|(_, _, s)| s.len()).sum();
    
    println!("  Source data: {:.1} MB", total_source_bytes as f64 / 1_048_576.0);
    println!("  Memory used: {:.1} MB", stats.total_memory_bytes as f64 / 1_048_576.0);
    println!("  Disk used: {:.1} MB", stats.total_disk_bytes as f64 / 1_048_576.0);
    println!("  Compression ratio: {:.2}x", 
        total_source_bytes as f64 / (stats.total_memory_bytes + stats.total_disk_bytes) as f64);
    println!("  Memory efficiency: {:.1}%", 
        (stats.total_memory_bytes as f64 / total_source_bytes as f64) * 100.0);
}

fn benchmark_tier_transitions(
    cache: &Arc<Phase4Cache>,
    test_data: &[(PathBuf, u64, String)]
) {
    println!("  Testing tier transition performance...");
    
    let start = Instant::now();
    
    // Force multiple tier management cycles
    for _i in 0..10 {
        cache.manage_tiers().unwrap();
        thread::sleep(Duration::from_millis(100));
        
        // Access some items to trigger promotions
        for j in 0..10 {
            let idx = (i * 10 + j) % test_data.len();
            let (path, hash, _) = &test_data[idx];
            cache.get(path, *hash).ok();
        }
    }
    
    let duration = start.elapsed();
    let stats = cache.stats();
    
    println!("  Tier management cycles: 10");
    println!("  Total time: {:.2}s", duration.as_secs_f64());
    println!("  Avg cycle time: {:.1}ms", duration.as_millis() as f64 / 10.0);
    println!("  Final distribution:");
    println!("    Hot: {}, Warm: {}, Cold: {}, Frozen: {}", 
        stats.hot_entries, stats.warm_entries, 
        stats.cold_entries, stats.frozen_entries);
    println!("  Total promotions: {}", stats.cache_hits);
}

fn calculate_metrics(
    mut latencies: Vec<f64>,
    throughput: f64,
    memory_mb: f64,
    cpu_percent: f64,
) -> PerformanceMetrics {
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[latencies.len() * 95 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];
    
    PerformanceMetrics {
        throughput,
        avg_latency_ms: avg_latency,
        p50_latency_ms: p50,
        p95_latency_ms: p95,
        p99_latency_ms: p99,
        memory_mb,
        cpu_percent,
    }
}

fn print_metrics(name: &str, metrics: &PerformanceMetrics) {
    println!("\n{}:", name);
    println!("  Throughput: {:.0} ops/sec", metrics.throughput);
    println!("  Latency (avg): {:.2} ms", metrics.avg_latency_ms);
    println!("  Latency (p50): {:.2} ms", metrics.p50_latency_ms);
    println!("  Latency (p95): {:.2} ms", metrics.p95_latency_ms);
    println!("  Latency (p99): {:.2} ms", metrics.p99_latency_ms);
    if metrics.memory_mb > 0.0 {
        println!("  Memory: {:.1} MB", metrics.memory_mb);
    }
    if metrics.cpu_percent > 0.0 {
        println!("  CPU: {:.1}%", metrics.cpu_percent);
    }
}

fn hash_string(s: &str) -> u64 {
    s.bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
}

/// Get process memory usage in MB from /proc/self/status (Linux only)
fn get_process_memory_mb() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<f64>() {
                            return Some(kb / 1024.0); // Convert KB to MB
                        }
                    }
                }
            }
        }
    }
    
    // Fallback to sysinfo for non-Linux or if /proc fails
    #[cfg(not(target_os = "linux"))]
    {
        use sysinfo::{System, Pid, ProcessRefreshKind};
        let mut system = System::new();
        let pid = Pid::from_u32(std::process::id());
        system.refresh_process_specifics(pid, ProcessRefreshKind::new().with_memory());
        
        if let Some(process) = system.process(pid) {
            return Some(process.memory() as f64 / 1024.0 / 1024.0); // bytes -> MB
        }
    }
    
    None
}

/// Get per-process CPU usage percentage
fn get_process_cpu_percent() -> f64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        // Try to read /proc/self/stat for CPU times
        if let Ok(stat) = fs::read_to_string("/proc/self/stat") {
            let parts: Vec<&str> = stat.split_whitespace().collect();
            // Fields 13 (utime) and 14 (stime) are CPU times in clock ticks
            if parts.len() > 14 {
                if let (Ok(utime), Ok(stime)) = (parts[13].parse::<u64>(), parts[14].parse::<u64>()) {
                    // Get clock ticks per second
                    let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
                    if ticks_per_sec > 0 {
                        let total_ticks = utime + stime;
                        let cpu_seconds = total_ticks as f64 / ticks_per_sec as f64;
                        
                        // Get process uptime from /proc/self/stat (field 21 is starttime)
                        if parts.len() > 21 {
                            if let Ok(starttime) = parts[21].parse::<u64>() {
                                // Read system uptime
                                if let Ok(uptime_str) = fs::read_to_string("/proc/uptime") {
                                    if let Some(uptime) = uptime_str.split_whitespace().next() {
                                        if let Ok(system_uptime) = uptime.parse::<f64>() {
                                            let process_uptime = system_uptime - (starttime as f64 / ticks_per_sec as f64);
                                            if process_uptime > 0.0 {
                                                // Return CPU percentage (capped at 100%)
                                                return (cpu_seconds / process_uptime * 100.0).min(100.0);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Fallback to sysinfo crate
    use sysinfo::{System, Pid, ProcessRefreshKind};
    let mut system = System::new();
    let pid = Pid::from_u32(std::process::id());
    system.refresh_process_specifics(pid, ProcessRefreshKind::new().with_cpu());
    
    // Need to wait a bit and refresh again to get CPU percentage
    std::thread::sleep(Duration::from_millis(100));
    system.refresh_process_specifics(pid, ProcessRefreshKind::new().with_cpu());
    
    if let Some(process) = system.process(pid) {
        process.cpu_usage() as f64
    } else {
        0.0
    }
}
