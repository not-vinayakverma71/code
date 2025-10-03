/// Performance Regression Test Suite
/// Ensures performance doesn't degrade with changes
use std::time::{Duration, Instant};
use lapce_ai_rust::shared_memory_complete::SharedMemoryBuffer;

#[test]
fn test_regression_latency() {
    let mut buffer = SharedMemoryBuffer::create("regression_latency", 4 * 1024 * 1024).unwrap();
    let data = vec![0x42u8; 256];
    
    // Warmup
    for _ in 0..10000 {
        buffer.write(&data).unwrap();
        buffer.read().unwrap();
    }
    
    // Measure
    let iterations = 100000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        buffer.write(&data).unwrap();
        buffer.read().unwrap();
    }
    
    let duration = start.elapsed();
    let latency_us = duration.as_micros() as f64 / (iterations * 2) as f64;
    
    // Regression thresholds
    assert!(latency_us < 1.0, "Latency regression: {:.3}μs (expected < 1.0μs)", latency_us);
}

#[test]
fn test_regression_throughput() {
    let mut buffer = SharedMemoryBuffer::create("regression_throughput", 4 * 1024 * 1024).unwrap();
    let data = vec![0xAAu8; 1024];
    
    let iterations = 500000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        buffer.write(&data).unwrap();
        buffer.read().unwrap();
    }
    
    let duration = start.elapsed();
    let throughput = (iterations * 2) as f64 / duration.as_secs_f64();
    
    // Regression threshold: must maintain >10M ops/sec
    assert!(throughput > 10_000_000.0, 
            "Throughput regression: {:.2}M ops/sec (expected > 10M)", 
            throughput / 1_000_000.0);
}

#[test]
fn test_regression_memory() {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    struct TrackingAllocator;
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    
    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
            System.alloc(layout)
        }
        
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
            System.dealloc(ptr, layout)
        }
    }
    
    // Measure memory for 100 operations
    let initial = ALLOCATED.load(Ordering::SeqCst);
    
    let mut buffer = SharedMemoryBuffer::create("regression_memory", 1024 * 1024).unwrap();
    let data = vec![0u8; 256];
    
    for _ in 0..100 {
        buffer.write(&data).unwrap();
        buffer.read().unwrap();
    }
    
    let final_mem = ALLOCATED.load(Ordering::SeqCst);
    let used = final_mem.saturating_sub(initial);
    
    // Should use < 10KB for 100 operations (essentially zero allocations)
    assert!(used < 10240, "Memory regression: {} bytes used (expected < 10KB)", used);
}

#[test]
fn test_regression_concurrent_throughput() {
    use std::sync::Arc;
    use std::thread;
    
    let threads = 4;
    let ops_per_thread = 100000;
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for _ in 0..threads {
        let handle = thread::spawn(move || {
            let mut buffer = SharedMemoryBuffer::create(
                &format!("regression_concurrent_{}", thread::current().id().as_u64()),
                1024 * 1024
            ).unwrap();
            let data = vec![0x55u8; 256];
            
            for _ in 0..ops_per_thread {
                buffer.write(&data).unwrap();
                buffer.read().unwrap();
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start.elapsed();
    let total_ops = threads * ops_per_thread * 2;
    let throughput = total_ops as f64 / duration.as_secs_f64();
    
    // Concurrent throughput should be > 20M ops/sec
    assert!(throughput > 20_000_000.0,
            "Concurrent throughput regression: {:.2}M ops/sec (expected > 20M)",
            throughput / 1_000_000.0);
}

#[test]
fn test_regression_large_messages() {
    let mut buffer = SharedMemoryBuffer::create("regression_large", 16 * 1024 * 1024).unwrap();
    
    // Test various sizes
    for size in [1024, 4096, 16384, 65536, 262144] {
        let data = vec![0xFFu8; size];
        let iterations = 10000;
        
        let start = Instant::now();
        for _ in 0..iterations {
            buffer.write(&data).unwrap();
            buffer.read().unwrap();
        }
        let duration = start.elapsed();
        
        let throughput_mb = (size * iterations * 2) as f64 / 1_048_576.0 / duration.as_secs_f64();
        
        // Should maintain > 1GB/s for all sizes
        assert!(throughput_mb > 1000.0,
                "Large message regression at {}KB: {:.2}MB/s (expected > 1GB/s)",
                size / 1024, throughput_mb);
    }
}
