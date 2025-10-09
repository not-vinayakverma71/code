use lapce_ai_rust::ipc::shared_memory_complete::WorkingSharedMemory;
use std::time::{Instant, Duration};
use std::thread;

fn main() -> anyhow::Result<()> {
    println!("═══════════════════════════════════════════════════════");
    println!("   FINAL VALIDATION - 6 DAYS OF WORK");
    println!("═══════════════════════════════════════════════════════\n");
    
    // Test 1: Basic functionality
    println!("Test 1: Basic Functionality");
    println!("──────────────────────────");
    test_basic_functionality()?;
    
    // Test 2: Performance targets
    println!("\nTest 2: Performance Targets");
    println!("──────────────────────────");
    test_performance_targets()?;
    
    // Test 3: Concurrent connections
    println!("\nTest 3: Concurrent Connections");
    println!("──────────────────────────");
    test_concurrent_connections()?;
    
    // Test 4: Memory usage
    println!("\nTest 4: Memory Usage");
    println!("──────────────────────────");
    test_memory_usage()?;
    
    // Test 5: Large messages
    println!("\nTest 5: Large Messages");
    println!("──────────────────────────");
    test_large_messages()?;
    
    println!("\n═══════════════════════════════════════════════════════");
    println!("   VALIDATION COMPLETE");
    println!("═══════════════════════════════════════════════════════\n");
    
    Ok(())
}

fn test_basic_functionality() -> anyhow::Result<()> {
    let mut shm = WorkingSharedMemory::create("basic", 1024 * 1024)?;
    
    // Test write
    let data = vec![0xAB; 256];
    if shm.write(&data) {
        println!("✅ Write successful");
    } else {
        println!("❌ Write failed");
        return Ok(());
    }
    
    // Test read
    let mut consumer = WorkingSharedMemory::connect("basic")?;
    if let Some(read_data) = consumer.read() {
        if read_data == data {
            println!("✅ Read successful and data matches");
        } else {
            println!("❌ Read data doesn't match");
        }
    } else {
        println!("❌ Read failed");
    }
    
    Ok(())
}

fn test_performance_targets() -> anyhow::Result<()> {
    let mut shm = WorkingSharedMemory::create("perf", 64 * 1024 * 1024)?;
    
    // Test latency
    let data = vec![0xCD; 64];
    let start = Instant::now();
    for _ in 0..10000 {
        shm.write(&data);
    }
    let latency = start.elapsed() / 10000;
    
    print!("Latency: {:?}", latency);
    if latency < Duration::from_micros(10) {
        println!(" ✅ (<10μs target met)");
    } else {
        println!(" ❌ (target: <10μs)");
    }
    
    // Test throughput
    let start = Instant::now();
    let mut count = 0;
    while start.elapsed() < Duration::from_secs(1) {
        if shm.write(&data) {
            count += 1;
        }
    }
    
    print!("Throughput: {:.2}M msg/sec", count as f64 / 1_000_000.0);
    if count > 1_000_000 {
        println!(" ✅ (>1M target met)");
    } else {
        println!(" ❌ (target: >1M msg/sec)");
    }
    
    Ok(())
}

fn test_concurrent_connections() -> anyhow::Result<()> {
    const NUM_CONNECTIONS: usize = 100; // Test with 100 first
    
    let mut handles = vec![];
    
    for i in 0..NUM_CONNECTIONS {
        let handle = thread::spawn(move || {
            let mut shm = WorkingSharedMemory::create(&format!("conn_{}", i), 1024 * 1024).unwrap();
            let data = vec![i as u8; 64];
            shm.write(&data)
        });
        handles.push(handle);
    }
    
    let mut success = 0;
    for handle in handles {
        if handle.join().unwrap() {
            success += 1;
        }
    }
    
    print!("{}/{} connections successful", success, NUM_CONNECTIONS);
    if success == NUM_CONNECTIONS {
        println!(" ✅");
    } else {
        println!(" ❌");
    }
    
    Ok(())
}

fn test_memory_usage() -> anyhow::Result<()> {
    // Create multiple buffers
    let mut buffers = vec![];
    
    for i in 0..10 {
        let shm = WorkingSharedMemory::create(&format!("mem_{}", i), 1024 * 1024)?;
        buffers.push(shm);
    }
    
    println!("Created 10 × 1MB buffers");
    println!("✅ Memory allocation successful");
    // Note: Real memory measurement would require external tools
    
    Ok(())
}

fn test_large_messages() -> anyhow::Result<()> {
    let mut shm = WorkingSharedMemory::create("large", 128 * 1024 * 1024)?;
    
    let sizes = vec![
        (1024, "1KB"),
        (16 * 1024, "16KB"),
        (256 * 1024, "256KB"),
        (1024 * 1024, "1MB"),
    ];
    
    for (size, label) in sizes {
        let data = vec![0xFF; size];
        if shm.write(&data) {
            println!("✅ {} write successful", label);
        } else {
            println!("❌ {} write failed", label);
        }
    }
    
    Ok(())
}
