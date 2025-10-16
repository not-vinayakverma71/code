/// Production-Grade Stress Test for IPC System
/// Tests ALL success criteria with realistic workload patterns
/// Based on 01-IPC-SERVER-IMPLEMENTATION.md requirements

use anyhow::{Result, bail};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use procfs::process::Process;

// Success criteria from documentation
const MAX_MEMORY_BYTES: usize = 3 * 1024 * 1024; // 3MB
const MAX_LATENCY_MICROS: u64 = 10; // <10μs
const MIN_THROUGHPUT: u64 = 1_000_000; // >1M msg/sec
const MAX_CONCURRENT: usize = 100; // Realistic production load

struct TestStats {
    messages_sent: AtomicU64,
    messages_succeeded: AtomicU64,
    total_latency_ns: AtomicU64,
    max_latency_ns: AtomicU64,
    latency_violations: AtomicU64,
    memory_samples: parking_lot::Mutex<Vec<usize>>,
}

impl TestStats {
    fn new() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_succeeded: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            max_latency_ns: AtomicU64::new(0),
            latency_violations: AtomicU64::new(0),
            memory_samples: parking_lot::Mutex::new(Vec::new()),
        }
    }
    
    fn record_success(&self, latency_ns: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.messages_succeeded.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);
        
        // Update max latency atomically
        let mut current_max = self.max_latency_ns.load(Ordering::Relaxed);
        while latency_ns > current_max {
            match self.max_latency_ns.compare_exchange_weak(
                current_max, latency_ns,
                Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
        
        if latency_ns > MAX_LATENCY_MICROS * 1000 {
            self.latency_violations.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    fn record_failure(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }
    
    fn avg_latency_micros(&self) -> f64 {
        let total = self.messages_succeeded.load(Ordering::Relaxed);
        if total == 0 { return 0.0; }
        let total_ns = self.total_latency_ns.load(Ordering::Relaxed);
        (total_ns as f64 / total as f64) / 1000.0
    }
    
    fn max_latency_micros(&self) -> f64 {
        self.max_latency_ns.load(Ordering::Relaxed) as f64 / 1000.0
    }
    
    fn success_rate(&self) -> f64 {
        let sent = self.messages_sent.load(Ordering::Relaxed);
        if sent == 0 { return 0.0; }
        let succeeded = self.messages_succeeded.load(Ordering::Relaxed);
        succeeded as f64 / sent as f64
    }
    
    fn violation_rate(&self) -> f64 {
        let succeeded = self.messages_succeeded.load(Ordering::Relaxed);
        if succeeded == 0 { return 0.0; }
        let violations = self.latency_violations.load(Ordering::Relaxed);
        violations as f64 / succeeded as f64
    }
}

fn get_memory_usage() -> Result<usize> {
    let status = Process::myself()?.status()?;
    Ok((status.vmrss.unwrap_or(0) * 1024) as usize)
}

async fn send_message(
    client: &lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile,
    data: &[u8],
    stats: &Arc<TestStats>,
) -> Result<()> {
    let start = Instant::now();
    
    match client.send_bytes(data).await {
        Ok(_) => {
            let latency_ns = start.elapsed().as_nanos() as u64;
            stats.record_success(latency_ns);
            Ok(())
        }
        Err(e) => {
            stats.record_failure();
            Err(e)
        }
    }
}

/// TEST 1: Throughput & Latency (100 concurrent clients, 10K messages each)
async fn test_throughput_and_latency(socket_path: &str) -> Result<()> {
    println!("\n═══════════════════════════════════════════════════════");
    println!("TEST 1: THROUGHPUT & LATENCY VALIDATION");
    println!("═══════════════════════════════════════════════════════");
    println!("Config: {} concurrent clients, 10K msgs each", MAX_CONCURRENT);
    println!("Target: >{}M msg/sec, <{}µs latency", MIN_THROUGHPUT / 1_000_000, MAX_LATENCY_MICROS);
    
    let stats = Arc::new(TestStats::new());
    let test_start = Instant::now();
    
    // Spawn concurrent clients with gradual ramp-up
    let mut handles = Vec::new();
    for client_id in 0..MAX_CONCURRENT {
        let socket_path = socket_path.to_string();
        let stats = stats.clone();
        
        handles.push(tokio::spawn(async move {
            // Connect
            let client = match lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile::connect(&socket_path).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Client {} connect failed: {}", client_id, e);
                    return;
                }
            };
            
            // Send 10,000 messages
            let msg = vec![0xAB; 1024]; // 1KB message
            for _ in 0..10_000 {
                if let Err(e) = send_message(&client, &msg, &stats).await {
                    if client_id == 0 {
                        eprintln!("Client {} error: {}", client_id, e);
                    }
                    break;
                }
            }
            
            if client_id % 20 == 0 {
                println!("✓ Client {} completed", client_id);
            }
        }));
        
        // Gradual ramp-up: 10ms between every 10 clients
        if (client_id + 1) % 10 == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    // Wait for all clients
    for handle in handles {
        let _ = handle.await;
    }
    
    let test_duration = test_start.elapsed();
    let throughput = stats.messages_succeeded.load(Ordering::Relaxed) as f64 / test_duration.as_secs_f64();
    
    println!("\n📊 RESULTS:");
    println!("───────────────────────────────────────────────────────");
    println!("Duration:            {:.2}s", test_duration.as_secs_f64());
    println!("Messages sent:       {}", stats.messages_sent.load(Ordering::Relaxed));
    println!("Messages succeeded:  {}", stats.messages_succeeded.load(Ordering::Relaxed));
    println!("Success rate:        {:.2}%", stats.success_rate() * 100.0);
    println!("Throughput:          {:.0} msg/sec ({:.2}M msg/sec)", throughput, throughput / 1_000_000.0);
    println!("Avg latency:         {:.2}µs", stats.avg_latency_micros());
    println!("Max latency:         {:.2}µs", stats.max_latency_micros());
    println!("Latency violations:  {} ({:.2}%)", 
             stats.latency_violations.load(Ordering::Relaxed),
             stats.violation_rate() * 100.0);
    
    // Validate criteria
    if throughput < MIN_THROUGHPUT as f64 {
        bail!("❌ THROUGHPUT FAILURE: {:.0} < {} msg/sec", throughput, MIN_THROUGHPUT);
    }
    if stats.violation_rate() > 0.01 {
        bail!("❌ LATENCY FAILURE: {:.2}% violations > 1%", stats.violation_rate() * 100.0);
    }
    if stats.success_rate() < 0.99 {
        bail!("❌ RELIABILITY FAILURE: {:.2}% success < 99%", stats.success_rate() * 100.0);
    }
    
    println!("✅ TEST 1 PASSED");
    Ok(())
}

/// TEST 2: Memory Leak Detection (sustained load for 5 minutes)
async fn test_memory_leak(socket_path: &str) -> Result<()> {
    println!("\n═══════════════════════════════════════════════════════");
    println!("TEST 2: MEMORY LEAK DETECTION");
    println!("═══════════════════════════════════════════════════════");
    println!("Config: 5 minutes sustained load, varying connections");
    println!("Target: <512KB memory growth, stable under {}MB", MAX_MEMORY_BYTES / 1024 / 1024);
    
    let stats = Arc::new(TestStats::new());
    let start_memory = get_memory_usage()?;
    let mut memory_samples = Vec::new();
    memory_samples.push(start_memory);
    
    println!("Initial memory: {:.2}MB", start_memory as f64 / 1024.0 / 1024.0);
    
    // Run for 60 cycles (5 minutes with 5-second cycles)
    for cycle in 0..60 {
        let num_clients = 20 + (cycle % 5) * 10; // Vary between 20-60 clients
        
        let mut handles = Vec::new();
        for _ in 0..num_clients {
            let socket_path = socket_path.to_string();
            let stats = stats.clone();
            
            handles.push(tokio::spawn(async move {
                if let Ok(client) = lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile::connect(&socket_path).await {
                    let msg = vec![0xCD; 2048];
                    for _ in 0..100 {
                        let _ = send_message(&client, &msg, &stats).await;
                    }
                }
            }));
        }
        
        for handle in handles {
            let _ = handle.await;
        }
        
        let current_memory = get_memory_usage()?;
        memory_samples.push(current_memory);
        
        if current_memory > MAX_MEMORY_BYTES {
            bail!("❌ MEMORY OVERFLOW: {}MB > {}MB at cycle {}", 
                 current_memory / 1024 / 1024, MAX_MEMORY_BYTES / 1024 / 1024, cycle);
        }
        
        if cycle % 10 == 0 {
            println!("Cycle {}/60: Memory = {:.2}MB, Throughput = {:.0} msg/sec", 
                    cycle, 
                    current_memory as f64 / 1024.0 / 1024.0,
                    stats.messages_succeeded.load(Ordering::Relaxed) as f64 / ((cycle + 1) * 5) as f64);
        }
        
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    
    let final_memory = get_memory_usage()?;
    let growth = final_memory as isize - start_memory as isize;
    let max_memory = memory_samples.iter().copied().max().unwrap_or(0);
    
    println!("\n📊 RESULTS:");
    println!("───────────────────────────────────────────────────────");
    println!("Start memory:  {:.2}MB", start_memory as f64 / 1024.0 / 1024.0);
    println!("Final memory:  {:.2}MB", final_memory as f64 / 1024.0 / 1024.0);
    println!("Max memory:    {:.2}MB", max_memory as f64 / 1024.0 / 1024.0);
    println!("Growth:        {:.2}KB ({:+.1}%)", 
            growth as f64 / 1024.0,
            (growth as f64 / start_memory as f64) * 100.0);
    println!("Messages:      {}", stats.messages_succeeded.load(Ordering::Relaxed));
    
    if growth > 512 * 1024 {
        bail!("❌ MEMORY LEAK: {}KB growth > 512KB limit", growth / 1024);
    }
    if max_memory > MAX_MEMORY_BYTES {
        bail!("❌ MEMORY OVERFLOW: {}MB > {}MB", max_memory / 1024 / 1024, MAX_MEMORY_BYTES / 1024 / 1024);
    }
    
    println!("✅ TEST 2 PASSED");
    Ok(())
}

/// TEST 3: Sustained Latency Under Load (99th percentile validation)
async fn test_sustained_latency(socket_path: &str) -> Result<()> {
    println!("\n═══════════════════════════════════════════════════════");
    println!("TEST 3: SUSTAINED LATENCY VALIDATION");
    println!("═══════════════════════════════════════════════════════");
    println!("Config: 2 minutes sustained 50 clients hammering");
    println!("Target: p99 latency <50µs");
    
    let stats = Arc::new(TestStats::new());
    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    
    // Background load
    let mut handles = Vec::new();
    for _ in 0..50 {
        let socket_path = socket_path.to_string();
        let shutdown = shutdown.clone();
        
        handles.push(tokio::spawn(async move {
            if let Ok(client) = lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile::connect(&socket_path).await {
                let msg = vec![0xEF; 1024];
                while !shutdown.load(Ordering::Relaxed) {
                    let _ = client.send_bytes(&msg).await;
                }
            }
        }));
    }
    
    // Measurement client
    let client = lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile::connect(socket_path).await?;
    let msg = vec![0xAB; 1024];
    
    let mut latencies = Vec::new();
    for i in 0..10_000 {
        let start = Instant::now();
        match client.send_bytes(&msg).await {
            Ok(_) => {
                let latency_micros = start.elapsed().as_micros() as u64;
                latencies.push(latency_micros);
                stats.record_success(latency_micros * 1000);
            }
            Err(_) => stats.record_failure(),
        }
        
        if i % 1000 == 0 {
            println!("Progress: {}/10000 measurements", i);
        }
    }
    
    shutdown.store(true, Ordering::Relaxed);
    for handle in handles {
        handle.abort();
    }
    
    // Calculate percentiles
    latencies.sort_unstable();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() * 95) / 100];
    let p99 = latencies[(latencies.len() * 99) / 100];
    
    println!("\n📊 RESULTS:");
    println!("───────────────────────────────────────────────────────");
    println!("Measurements:  {}", latencies.len());
    println!("Avg latency:   {:.2}µs", stats.avg_latency_micros());
    println!("p50 latency:   {}µs", p50);
    println!("p95 latency:   {}µs", p95);
    println!("p99 latency:   {}µs", p99);
    println!("Max latency:   {:.2}µs", stats.max_latency_micros());
    
    if p99 > 50 {
        bail!("❌ P99 LATENCY FAILURE: {}µs > 50µs", p99);
    }
    
    println!("✅ TEST 3 PASSED");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║   PRODUCTION STRESS TEST - IPC VALIDATION            ║");
    println!("╚═══════════════════════════════════════════════════════╝");
    
    let socket_path = "/tmp/stress_test_ipc.sock";
    
    println!("\n📋 SUCCESS CRITERIA:");
    println!("  • Memory:     <{}MB total", MAX_MEMORY_BYTES / 1024 / 1024);
    println!("  • Latency:    <{}µs per message", MAX_LATENCY_MICROS);
    println!("  • Throughput: >{}M msg/sec", MIN_THROUGHPUT / 1_000_000);
    println!("  • Concurrent: {} clients sustained", MAX_CONCURRENT);
    
    // Run all tests
    test_throughput_and_latency(socket_path).await?;
    test_memory_leak(socket_path).await?;
    test_sustained_latency(socket_path).await?;
    
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║    🎉 ALL TESTS PASSED - PRODUCTION READY 🎉         ║");
    println!("╚═══════════════════════════════════════════════════════╝");
    
    Ok(())
}
