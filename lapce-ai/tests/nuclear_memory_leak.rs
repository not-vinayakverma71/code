/// Nuclear Test 4: Memory Leak Detection
/// 120 cycles of varying load (100-500 connections)
/// Target: No memory growth >512KB from baseline

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};

// Debug mode: 10x faster
#[cfg(debug_assertions)]
const CYCLES: usize = 12;
#[cfg(debug_assertions)]
const MIN_CONNECTIONS: usize = 10;
#[cfg(debug_assertions)]
const MAX_CONNECTIONS: usize = 50;

// Release mode: full scale
#[cfg(not(debug_assertions))]
const CYCLES: usize = 120;
#[cfg(not(debug_assertions))]
const MIN_CONNECTIONS: usize = 100;
#[cfg(not(debug_assertions))]
const MAX_CONNECTIONS: usize = 500;

#[tokio::test(flavor = "multi_thread")]
async fn nuclear_memory_leak() {
    println!("\n🔍 NUCLEAR TEST 4: MEMORY LEAK DETECTION");
    println!("=========================================");
    println!("Cycles: {}", CYCLES);
    println!("Connection range: {}-{}", MIN_CONNECTIONS, MAX_CONNECTIONS);
    println!("Target: <512KB memory growth\n");
    
    let start_time = Instant::now();
    
    // Start raw SHM echo server
    let socket_path = "/tmp/nuc4.sock";
    let listener = Arc::new(SharedMemoryListener::bind(socket_path).unwrap());
    let server_handle = {
        let listener = listener.clone();
        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 256];
                        loop {
                            if stream.read_exact(&mut buf).await.is_err() { break; }
                            if stream.write_all(&buf).await.is_err() { break; }
                        }
                    });
                }
            }
        })
    };
    
    sleep(Duration::from_millis(500)).await;
    
    // Get baseline memory after warmup
    let baseline_memory = {
        // Warmup with some connections
        let mut warmup_handles = Vec::new();
        for _ in 0..10 {
            let handle = tokio::spawn(async move {
                let mut stream = SharedMemoryStream::connect(socket_path)
                    .await
                    .expect("Failed to connect");
                
                for _ in 0..100 {
                    let msg = vec![0u8; 256];
                    stream.write_all(&msg).await.unwrap();
                    let mut resp = vec![0u8; 256];
                    stream.read_exact(&mut resp).await.unwrap();
                }
            });
            warmup_handles.push(handle);
        }
        
        for handle in warmup_handles {
            handle.await.unwrap();
        }
        
        // Force GC and measure baseline
        drop_and_measure_memory()
    };
    
    println!("Baseline memory: {:.2} MB", baseline_memory);
    
    let mut memory_samples = Vec::new();
    
    // Run cycles with varying load
    for cycle in 0..CYCLES {
        // Vary connection count sinusoidally
        let ratio = (cycle as f64 * std::f64::consts::PI * 2.0 / CYCLES as f64).sin();
        let conn_count = MIN_CONNECTIONS + 
            ((MAX_CONNECTIONS - MIN_CONNECTIONS) as f64 * (ratio + 1.0) / 2.0) as usize;
        
        if cycle % 10 == 0 {
            println!("Cycle {}/{}: {} connections", cycle, CYCLES, conn_count);
        }
        
        let stop_signal = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();
        
        // Spawn connections
        for _ in 0..conn_count {
            let stop = stop_signal.clone();
            let handle = tokio::spawn(async move {
                let mut stream = SharedMemoryStream::connect(socket_path)
                    .await
                    .expect("Failed to connect");
                
                while !stop.load(Ordering::Relaxed) {
                    let msg = vec![0u8; 256];
                    stream.write_all(&msg).await.unwrap();
                    let mut resp = vec![0u8; 256];
                    stream.read_exact(&mut resp).await.unwrap();
                    
                    tokio::task::yield_now().await;
                }
            });
            handles.push(handle);
        }
        
        // Run for a bit
        sleep(Duration::from_millis(100)).await;
        
        // Stop connections
        stop_signal.store(true, Ordering::Relaxed);
        for handle in handles {
            handle.abort();
        }
        
        // Measure memory
        let current_memory = drop_and_measure_memory();
        memory_samples.push(current_memory);
        
        // Small delay between cycles
        sleep(Duration::from_millis(50)).await;
    }
    
    // Analyze memory growth
    let final_memory = *memory_samples.last().unwrap();
    let memory_growth_mb = final_memory - baseline_memory;
    let memory_growth_kb = memory_growth_mb * 1024.0;
    
    // Calculate trend
    let mut max_memory = baseline_memory;
    let mut min_memory = baseline_memory;
    for &mem in &memory_samples {
        if mem > max_memory {
            max_memory = mem;
        }
        if mem < min_memory {
            min_memory = mem;
        }
    }
    
    let total_time = start_time.elapsed();
    
    println!("\n📊 RESULTS");
    println!("==========");
    println!("Test duration: {:.2}s", total_time.as_secs_f64());
    println!("Cycles completed: {}", CYCLES);
    println!("\nMemory Analysis:");
    println!("  Baseline: {:.2} MB", baseline_memory);
    println!("  Final: {:.2} MB", final_memory);
    println!("  Growth: {:.2} KB", memory_growth_kb);
    println!("  Min: {:.2} MB", min_memory);
    println!("  Max: {:.2} MB", max_memory);
    println!("  Peak delta: {:.2} KB", (max_memory - baseline_memory) * 1024.0);
    
    // Validation
    if memory_growth_kb.abs() < 512.0 {
        println!("\n✅ SUCCESS: Memory growth {:.2}KB < 512KB", memory_growth_kb.abs());
    } else {
        println!("\n❌ FAILED: Memory growth {:.2}KB >= 512KB", memory_growth_kb.abs());
        panic!("Memory leak detected");
    }
    
    server_handle.abort();
}

fn drop_and_measure_memory() -> f64 {
    // Try to trigger GC
    drop(vec![0u8; 1_000_000]); // Allocate and drop to trigger cleanup
    std::thread::sleep(Duration::from_millis(100));
    
    use sysinfo::System;
    
    let mut sys = System::new();
    sys.refresh_all();
    
    let pid = sysinfo::Pid::from(std::process::id() as usize);
    if let Some(process) = sys.process(pid) {
        process.memory() as f64 / 1024.0 // KB to MB
    } else {
        0.0
    }
}
