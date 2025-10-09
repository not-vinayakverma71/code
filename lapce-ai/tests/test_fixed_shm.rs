// Day 1: Test fixed SharedMemory implementation
use lapce_ai_rust::ipc::shared_memory_complete::FixedSharedMemory;
use std::time::Instant;

#[test]
fn test_all_data_sizes() {
    println!("\n=== DAY 1: Testing Fixed SharedMemory ===\n");
    
    let mut shm = FixedSharedMemory::create("test_sizes", 4 * 1024 * 1024).unwrap();
    
    let test_sizes = vec![
        1, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536
    ];
    
    let mut results = vec![];
    
    for size in test_sizes {
        let data = vec![0xAB; size];
        let success = shm.write(&data);
        
        if success {
            let read = shm.read();
            let read_success = read.is_some() && read.unwrap() == data;
            results.push((size, true, read_success));
            println!("  {} bytes: Write ✓, Read {}", 
                     size, if read_success { "✓" } else { "✗" });
        } else {
            results.push((size, false, false));
            println!("  {} bytes: Write ✗", size);
        }
    }
    
    let success_rate = results.iter().filter(|(_, w, r)| *w && *r).count() as f64 
                       / results.len() as f64 * 100.0;
    
    println!("\nSuccess rate: {:.1}%", success_rate);
    assert!(success_rate > 95.0, "Success rate too low: {}%", success_rate);
}

#[test]
fn test_throughput() {
    println!("\n=== Testing Throughput ===\n");
    
    let mut shm = FixedSharedMemory::create("test_perf", 16 * 1024 * 1024).unwrap();
    
    let test_configs = vec![
        (64, 1_000_000),
        (256, 500_000),
        (1024, 200_000),
        (4096, 50_000),
    ];
    
    for (size, iterations) in test_configs {
        let data = vec![0xFF; size];
        let mut write_success = 0;
        let mut read_success = 0;
        
        let start = Instant::now();
        
        for _ in 0..iterations {
            if shm.write(&data) {
                write_success += 1;
                if shm.read().is_some() {
                    read_success += 1;
                }
            }
        }
        
        let elapsed = start.elapsed();
        let throughput = write_success as f64 / elapsed.as_secs_f64();
        let success_rate = (write_success as f64 / iterations as f64) * 100.0;
        
        println!("  Size: {} bytes", size);
        println!("    Throughput: {:.2} msg/sec ({:.2}M msg/sec)", 
                 throughput, throughput / 1_000_000.0);
        println!("    Success rate: {:.1}%", success_rate);
        println!("    Latency: {:.3}μs/op", 
                 elapsed.as_micros() as f64 / write_success as f64);
        
        if size <= 256 {
            assert!(throughput > 100_000.0, 
                    "Throughput too low for {} bytes: {}", size, throughput);
        }
    }
}

#[test]
fn test_concurrent_access() {
    use std::sync::Arc;
    use std::thread;
    
    println!("\n=== Testing Concurrent Access ===\n");
    
    let shm = Arc::new(std::sync::Mutex::new(
        FixedSharedMemory::create("test_concurrent", 8 * 1024 * 1024).unwrap()
    ));
    
    let mut handles = vec![];
    let iterations = 10000;
    let num_threads = 4;
    
    for thread_id in 0..num_threads {
        let shm_clone = shm.clone();
        
        handles.push(thread::spawn(move || {
            let mut success_count = 0;
            let data = vec![thread_id; 256];
            
            for _ in 0..iterations {
                let mut shm_guard = shm_clone.lock().unwrap();
                if shm_guard.write(&data) {
                    success_count += 1;
                    shm_guard.read();
                }
            }
            
            success_count
        }));
    }
    
    let mut total_success = 0;
    for handle in handles {
        total_success += handle.join().unwrap();
    }
    
    let success_rate = (total_success as f64 / (iterations * num_threads) as f64) * 100.0;
    println!("  Total operations: {}", iterations * num_threads);
    println!("  Successful: {}", total_success);
    println!("  Success rate: {:.1}%", success_rate);
    
    assert!(success_rate > 90.0, "Concurrent success rate too low");
}

#[test]
fn test_wrap_around() {
    println!("\n=== Testing Ring Buffer Wrap-Around ===\n");
    
    // Small buffer to force wrap-around
    let mut shm = FixedSharedMemory::create("test_wrap", 64 * 1024).unwrap();
    
    let chunk_size = 4096;
    let iterations = 100; // Will definitely wrap around
    let data = vec![0xCD; chunk_size];
    
    let mut write_count = 0;
    let mut read_count = 0;
    
    for i in 0..iterations {
        if shm.write(&data) {
            write_count += 1;
            
            if let Some(read_data) = shm.read() {
                read_count += 1;
                assert_eq!(read_data.len(), chunk_size, 
                          "Data corruption at iteration {}", i);
            }
        }
    }
    
    println!("  Writes: {}/{}", write_count, iterations);
    println!("  Reads: {}/{}", read_count, iterations);
    println!("  Wrap-around handling: ✓");
    
    assert!(write_count > 0, "No successful writes");
    assert_eq!(write_count, read_count, "Write/read mismatch");
}
