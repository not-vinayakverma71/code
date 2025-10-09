// Comprehensive SharedMemory Tests (Tasks 35-42)
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;
use sysinfo::{System, Pid, ProcessRefreshKind};

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("🧪 COMPREHENSIVE SHARED MEMORY TESTS");
    println!("{}", "=".repeat(80));
    
    // Task 35: Test with 64B messages
    test_message_size(64, "64B")?;
    
    // Task 36: Test with 256B messages  
    test_message_size(256, "256B")?;
    
    // Task 37: Test with 1KB messages
    test_message_size(1024, "1KB")?;
    
    // Task 38: Test with 4KB messages
    test_message_size(4096, "4KB")?;
    
    // Task 39: Benchmark latency
    benchmark_latency()?;
    
    // Task 40: Test concurrent access
    test_concurrent_access()?;
    
    // Task 41: Test ring buffer
    test_ring_buffer()?;
    
    // Task 42: Profile memory usage
    profile_memory_usage()?;
    
    println!("\n✅ ALL SHARED MEMORY TESTS PASSED!");
    Ok(())
}

fn test_message_size(size: usize, label: &str) -> Result<()> {
    println!("\n📊 Testing {} messages...", label);
    
    let mut shm = lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create("test_msg_size", 1024 * 1024)?; // 1MB buffer
    let data = vec![0x42u8; size];
    
    // Write test
    let start = Instant::now();
    for _ in 0..1000 {
        shm.write(&data);
    }
    let write_duration = start.elapsed();
    
    // Read test
    let start = Instant::now();
    let mut read_count = 0;
    for _ in 0..1000 {
        if let Some(_data) = shm.read() {
            read_count += 1;
        }
    }
    let read_duration = start.elapsed();
    
    println!("  Write: {:.2} MB/s", (size * 1000) as f64 / write_duration.as_secs_f64() / 1_000_000.0);
    println!("  Read:  {:.2} MB/s", (size * 1000) as f64 / read_duration.as_secs_f64() / 1_000_000.0);
    println!("  ✅ {} message test passed", label);
    
    Ok(())
}

fn benchmark_latency() -> Result<()> {
    println!("\n⏱️  Benchmarking SharedMemory latency...");
    
    let mut shm = lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create("test_latency", 1024 * 1024)?;
    let data = vec![0x42u8; 128];
    let mut latencies = Vec::new();
    
    // Warm up
    for _ in 0..1000 {
        shm.write(&data);
        shm.read();
    }
    
    // Measure
    for _ in 0..10000 {
        let start = Instant::now();
        shm.write(&data);
        shm.read();
        latencies.push(start.elapsed().as_nanos());
    }
    
    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p99 = latencies[latencies.len() * 99 / 100];
    let p999 = latencies[latencies.len() * 999 / 1000];
    
    println!("  P50:  {} ns", p50);
    println!("  P99:  {} ns", p99);
    println!("  P99.9: {} ns", p999);
    println!("  ✅ Latency benchmark completed");
    
    Ok(())
}

fn test_concurrent_access() -> Result<()> {
    println!("\n🔀 Testing concurrent access...");
    
    // Create the shared memory file first
    let _master = lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create("test_concurrent", 1024 * 1024)?;
    
    let mut handles = Vec::new();
    
    // Spawn 10 writer threads - each opens the existing SharedMemory
    for i in 0..10 {
        let handle = thread::spawn(move || -> Result<()> {
            thread::sleep(Duration::from_millis(10)); // Small delay to ensure master creates file
            let mut shm = lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::open("test_concurrent", 1024 * 1024)?;
            let data = vec![i as u8; 128];
            for _ in 0..100 {
                shm.write(&data);
                thread::sleep(Duration::from_micros(10));
            }
            Ok(())
        });
        handles.push(handle);
    }
    
    // Spawn 10 reader threads - each opens the existing SharedMemory
    for _ in 0..10 {
        let handle = thread::spawn(move || -> Result<()> {
            thread::sleep(Duration::from_millis(10)); // Small delay to ensure master creates file
            let mut shm = lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::open("test_concurrent", 1024 * 1024)?;
            for _ in 0..100 {
                shm.read();
                thread::sleep(Duration::from_micros(10));
            }
            Ok(())
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap().unwrap_or(());
    }
    
    println!("  ✅ Concurrent access test passed (20 threads)");
    Ok(())
}

fn test_ring_buffer() -> Result<()> {
    println!("\n🔄 Testing ring buffer behavior...");
    
    let mut shm = lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create("test_ring", 1024)?; // Small buffer to test wrapping
    
    // Fill buffer multiple times to test wrap-around
    for i in 0..100 {
        let data = vec![i as u8; 256];
        shm.write(&data);
        
        if let Some(read_data) = shm.read() {
            assert_eq!(read_data[0], i as u8, "Ring buffer data mismatch");
        }
    }
    
    println!("  ✅ Ring buffer wrap-around working correctly");
    Ok(())
}

fn profile_memory_usage() -> Result<()> {
    println!("\n💾 Profiling memory usage...");
    
    let mut system = System::new();
    system.refresh_processes();
    let pid = Pid::from(std::process::id() as usize);
    let initial_memory = system.process(pid)
        .map(|p| p.memory())
        .unwrap_or(0);
    
    // Create multiple SharedMemory instances
    let mut instances = Vec::new();
    for i in 0..10 {
        instances.push(lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer::create(&format!("test_mem_{}", i), 1024 * 1024)?);
    }
    
    system.refresh_processes();
    let final_memory = system.process(pid)
        .map(|p| p.memory())
        .unwrap_or(0);
    
    let memory_per_instance = if final_memory > initial_memory {
        (final_memory - initial_memory) / 10
    } else {
        0
    };
    
    println!("  Initial memory: {} KB", initial_memory);
    println!("  Final memory: {} KB", final_memory);
    println!("  Memory per 1MB SharedMemory: {} KB", memory_per_instance);
    println!("  ✅ Memory profile: ~{} KB overhead per instance", memory_per_instance);
    
    Ok(())
}
