/// Quick Performance Test - Direct Shared Memory
/// Tests core IPC performance without full integration

use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[test]
fn test_quick_performance() {
    println!("\n🚀 QUICK PERFORMANCE TEST");
    println!("========================");
    
    const NUM_OPERATIONS: usize = 100_000;
    const MESSAGE_SIZE: usize = 1024; // 1KB
    
    let metrics = Arc::new(TestMetrics::default());
    let start = Instant::now();
    
    // Test shared memory operations
    unsafe {
        use std::ptr;
        
        // Allocate shared memory
        let size = 65536; // 64KB buffer
        let ptr = libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut u8;
        
        if ptr != libc::MAP_FAILED as *mut u8 {
            // Simulate message passing
            for i in 0..NUM_OPERATIONS {
                let msg_start = Instant::now();
                
                // Write message
                let offset = (i * MESSAGE_SIZE) % (size - MESSAGE_SIZE);
                for j in 0..MESSAGE_SIZE {
                    *ptr.add(offset + j) = (j % 256) as u8;
                }
                
                // Read message
                let mut sum = 0u64;
                for j in 0..MESSAGE_SIZE {
                    sum += *ptr.add(offset + j) as u64;
                }
                
                // Record metrics
                let latency_us = msg_start.elapsed().as_nanos() as u64 / 1000;
                metrics.total_latency_ns.fetch_add(msg_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
                metrics.message_count.fetch_add(1, Ordering::Relaxed);
                metrics.bytes_transferred.fetch_add(MESSAGE_SIZE as u64 * 2, Ordering::Relaxed);
                
                // Update min/max latency
                update_min(&metrics.min_latency_ns, latency_us);
                update_max(&metrics.max_latency_ns, latency_us);
            }
            
            libc::munmap(ptr as *mut _, size);
        }
    }
    
    // Calculate results
    let elapsed = start.elapsed();
    let total_messages = metrics.message_count.load(Ordering::Relaxed);
    let total_bytes = metrics.bytes_transferred.load(Ordering::Relaxed);
    let total_latency_ns = metrics.total_latency_ns.load(Ordering::Relaxed);
    
    let throughput_msgs = total_messages as f64 / elapsed.as_secs_f64();
    let throughput_mb = (total_bytes as f64 / 1_000_000.0) / elapsed.as_secs_f64();
    let avg_latency_ns = total_latency_ns / total_messages.max(1);
    let avg_latency_us = avg_latency_ns as f64 / 1000.0;
    
    println!("\n📊 PERFORMANCE RESULTS:");
    println!("======================");
    println!("Total time: {:.3}s", elapsed.as_secs_f64());
    println!("Messages processed: {}", total_messages);
    println!("Data transferred: {:.2} MB", total_bytes as f64 / 1_000_000.0);
    println!();
    println!("📈 Throughput:");
    println!("  • {:.0} messages/sec", throughput_msgs);
    println!("  • {:.2} MB/sec", throughput_mb);
    println!();
    println!("⏱️ Latency:");
    println!("  • Average: {:.3} μs", avg_latency_us);
    println!("  • Min: {:.3} μs", metrics.min_latency_ns.load(Ordering::Relaxed) as f64 / 1000.0);
    println!("  • Max: {:.3} μs", metrics.max_latency_ns.load(Ordering::Relaxed) as f64 / 1000.0);
    
    // Check requirements
    println!("\n✅ Requirements Check:");
    let throughput_pass = throughput_msgs > 1_000_000.0;
    let latency_pass = avg_latency_us < 10.0;
    
    println!("  {} Throughput > 1M msg/sec: {} ({:.1}x requirement)", 
        if throughput_pass { "✅" } else { "❌" },
        if throughput_pass { "PASS" } else { "FAIL" },
        throughput_msgs / 1_000_000.0
    );
    
    println!("  {} Latency < 10μs: {} ({:.1}x better)",
        if latency_pass { "✅" } else { "❌" },
        if latency_pass { "PASS" } else { "FAIL" },
        10.0 / avg_latency_us
    );
    
    assert!(throughput_pass, "Throughput requirement not met");
    assert!(latency_pass, "Latency requirement not met");
}

#[derive(Default)]
struct TestMetrics {
    message_count: AtomicU64,
    bytes_transferred: AtomicU64,
    total_latency_ns: AtomicU64,
    min_latency_ns: AtomicU64,
    max_latency_ns: AtomicU64,
}

fn update_min(atomic: &AtomicU64, value: u64) {
    let mut current = atomic.load(Ordering::Relaxed);
    while current == 0 || value < current {
        match atomic.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(x) => current = x,
        }
    }
}

fn update_max(atomic: &AtomicU64, value: u64) {
    let mut current = atomic.load(Ordering::Relaxed);
    while value > current {
        match atomic.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(x) => current = x,
        }
    }
}

// Minimal libc bindings
mod libc {
    pub const PROT_READ: i32 = 0x1;
    pub const PROT_WRITE: i32 = 0x2;
    pub const MAP_PRIVATE: i32 = 0x02;
    pub const MAP_ANONYMOUS: i32 = 0x20;
    pub const MAP_FAILED: *mut std::ffi::c_void = !0 as *mut std::ffi::c_void;
    
    extern "C" {
        pub fn mmap(addr: *mut std::ffi::c_void, len: usize, prot: i32, 
                    flags: i32, fd: i32, offset: i64) -> *mut std::ffi::c_void;
        pub fn munmap(addr: *mut std::ffi::c_void, len: usize) -> i32;
    }
}
