/// Stress test for IPC with 1000 concurrent clients
/// Measures memory usage, latency, throughput, and leak detection

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile;

const SOCKET_PATH: &str = "/tmp/stress_test_ipc.sock";

#[derive(Debug)]
struct StressMetrics {
    total_requests: AtomicUsize,
    successful: AtomicUsize,
    failed: AtomicUsize,
    total_latency_us: AtomicU64,
    min_latency_us: AtomicU64,
    max_latency_us: AtomicU64,
}

impl StressMetrics {
    fn new() -> Self {
        Self {
            total_requests: AtomicUsize::new(0),
            successful: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            total_latency_us: AtomicU64::new(0),
            min_latency_us: AtomicU64::new(u64::MAX),
            max_latency_us: AtomicU64::new(0),
        }
    }
    
    fn record_success(&self, latency_us: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        
        // Update min
        let mut current_min = self.min_latency_us.load(Ordering::Relaxed);
        while latency_us < current_min {
            match self.min_latency_us.compare_exchange_weak(
                current_min,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }
        
        // Update max
        let mut current_max = self.max_latency_us.load(Ordering::Relaxed);
        while latency_us > current_max {
            match self.max_latency_us.compare_exchange_weak(
                current_max,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }
    
    fn record_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed.fetch_add(1, Ordering::Relaxed);
    }
    
    fn report(&self) {
        let total = self.total_requests.load(Ordering::Relaxed);
        let success = self.successful.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let total_latency = self.total_latency_us.load(Ordering::Relaxed);
        let min_latency = self.min_latency_us.load(Ordering::Relaxed);
        let max_latency = self.max_latency_us.load(Ordering::Relaxed);
        
        let avg_latency = if success > 0 {
            total_latency / success as u64
        } else {
            0
        };
        
        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║           STRESS TEST RESULTS                          ║");
        println!("╠════════════════════════════════════════════════════════╣");
        println!("║  Total Requests:    {:>10}                       ║", total);
        println!("║  ✅ Successful:     {:>10}                       ║", success);
        println!("║  ❌ Failed:         {:>10}                       ║", failed);
        println!("║  Success Rate:      {:>9.2}%                      ║", (success as f64 / total as f64) * 100.0);
        println!("╠════════════════════════════════════════════════════════╣");
        println!("║  Min Latency:       {:>10} µs                    ║", if min_latency == u64::MAX { 0 } else { min_latency });
        println!("║  Avg Latency:       {:>10} µs                    ║", avg_latency);
        println!("║  Max Latency:       {:>10} µs                    ║", max_latency);
        println!("╚════════════════════════════════════════════════════════╝");
    }
}

fn get_memory_usage_kb() -> Option<usize> {
    let pid = std::process::id();
    let status_path = format!("/proc/{}/status", pid);
    let content = std::fs::read_to_string(status_path).ok()?;
    
    for line in content.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse().ok();
            }
        }
    }
    None
}

async fn single_client_test(
    client_id: usize,
    metrics: Arc<StressMetrics>,
    message: &[u8],
) -> anyhow::Result<()> {
    let client = IpcClientVolatile::connect(SOCKET_PATH).await?;
    
    let start = Instant::now();
    match timeout(Duration::from_secs(5), client.send_bytes(message)).await {
        Ok(Ok(_response)) => {
            let latency_us = start.elapsed().as_micros() as u64;
            metrics.record_success(latency_us);
            Ok(())
        }
        Ok(Err(e)) => {
            metrics.record_failure();
            Err(e)
        }
        Err(_) => {
            metrics.record_failure();
            anyhow::bail!("Timeout")
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║         IPC STRESS TEST - 1000 CONCURRENT              ║");
    println!("╚════════════════════════════════════════════════════════╝\n");
    
    // Get baseline memory
    let mem_baseline_kb = get_memory_usage_kb().unwrap_or(0);
    println!("📊 Baseline Memory: {} KB", mem_baseline_kb);
    
    let metrics = Arc::new(StressMetrics::new());
    let test_message = b"Stress test message for concurrent IPC validation";
    
    // Phase 1: Warmup with 10 clients
    println!("\n🔥 Phase 1: Warmup (10 clients)");
    let mut handles = vec![];
    for i in 0..10 {
        let metrics_clone = metrics.clone();
        let msg = test_message.to_vec();
        handles.push(tokio::spawn(async move {
            single_client_test(i, metrics_clone, &msg).await
        }));
    }
    
    for h in handles {
        let _ = h.await;
    }
    
    let mem_after_warmup_kb = get_memory_usage_kb().unwrap_or(0);
    println!("📊 After Warmup: {} KB (Δ {} KB)", mem_after_warmup_kb, mem_after_warmup_kb as i64 - mem_baseline_kb as i64);
    
    // Phase 2: 100 concurrent clients
    println!("\n🔥 Phase 2: Medium Load (100 concurrent clients)");
    let phase2_start = Instant::now();
    let mut handles = vec![];
    for i in 0..100 {
        let metrics_clone = metrics.clone();
        let msg = test_message.to_vec();
        handles.push(tokio::spawn(async move {
            single_client_test(i, metrics_clone, &msg).await
        }));
    }
    
    for h in handles {
        let _ = h.await;
    }
    
    let phase2_duration = phase2_start.elapsed();
    let mem_after_100_kb = get_memory_usage_kb().unwrap_or(0);
    println!("⏱️  Duration: {:?}", phase2_duration);
    println!("📊 After 100 clients: {} KB (Δ {} KB)", mem_after_100_kb, mem_after_100_kb as i64 - mem_after_warmup_kb as i64);
    
    // Phase 3: 1000 concurrent clients (THE BIG TEST)
    println!("\n🔥 Phase 3: FULL STRESS (1000 concurrent clients)");
    println!("⚠️  This is the production-grade validation...");
    
    let phase3_start = Instant::now();
    let mut handles = vec![];
    
    // Spawn in batches to avoid overwhelming tokio
    for batch in 0..10 {
        println!("  → Spawning batch {} (clients {}-{})", batch + 1, batch * 100, (batch + 1) * 100);
        for i in (batch * 100)..((batch + 1) * 100) {
            let metrics_clone = metrics.clone();
            let msg = test_message.to_vec();
            handles.push(tokio::spawn(async move {
                single_client_test(i, metrics_clone, &msg).await
            }));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    println!("  → All clients spawned, waiting for completion...");
    
    let mut batch_count = 0;
    for h in handles {
        let _ = h.await;
        batch_count += 1;
        if batch_count % 100 == 0 {
            println!("  ✓ Completed {} / 1000 clients", batch_count);
        }
    }
    
    let phase3_duration = phase3_start.elapsed();
    let mem_after_1000_kb = get_memory_usage_kb().unwrap_or(0);
    
    println!("\n⏱️  Phase 3 Duration: {:?}", phase3_duration);
    println!("📊 After 1000 clients: {} KB", mem_after_1000_kb);
    println!("📊 Memory Growth: {} KB", mem_after_1000_kb as i64 - mem_after_100_kb as i64);
    
    // Memory leak detection
    let mem_growth_per_100 = (mem_after_1000_kb as i64 - mem_after_100_kb as i64) / 9;
    println!("\n🔍 MEMORY LEAK ANALYSIS:");
    println!("   Memory per 100 clients: {} KB", mem_growth_per_100);
    if mem_growth_per_100 > 1000 {
        println!("   ⚠️  WARNING: Potential memory leak detected!");
    } else {
        println!("   ✅ Memory usage looks healthy");
    }
    
    // Final metrics
    metrics.report();
    
    // Throughput calculation
    let throughput = metrics.successful.load(Ordering::Relaxed) as f64 / phase3_duration.as_secs_f64();
    println!("\n📈 Throughput: {:.2} req/sec", throughput);
    
    Ok(())
}
