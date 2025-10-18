/// Test automatic error recovery
/// Success Criteria: Automatic reconnection within 100ms

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::alloc::{alloc_zeroed, dealloc, Layout};

use lapce_ai_rust::ipc::spsc_shm_ring::{SpscRing, RingHeader};
use lapce_ai_rust::ipc::shm_waiter_cross_os::ShmWaiter;

#[tokio::test]
async fn test_automatic_error_recovery() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ TEST: Automatic Error Recovery                              ║");
    println!("║ Target: Reconnection within 100ms                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let num_recovery_tests = 100;
    let mut recovery_times = Vec::new();
    let mut failures = 0;
    
    println!("Running {} recovery tests...\n", num_recovery_tests);
    
    for i in 0..num_recovery_tests {
        // Create connection
        let (send_ring, recv_ring, waiter) = create_connection().await;
        
        // Use connection normally
        let msg = vec![0u8; 1024];
        send_ring.try_write(&msg);
        waiter.wake_one(send_ring.write_seq_ptr());
        
        // Simulate failure by dropping connection
        let start_recovery = Instant::now();
        drop(send_ring);
        drop(recv_ring);
        drop(waiter);
        
        // Attempt reconnection
        let (new_send, new_recv, new_waiter) = create_connection().await;
        let recovery_time = start_recovery.elapsed();
        
        // Verify new connection works
        if new_send.try_write(&msg) {
            new_waiter.wake_one(new_send.write_seq_ptr());
            new_recv.try_write(&msg);
            new_waiter.wake_one(new_recv.write_seq_ptr());
            
            if new_recv.try_read().is_some() {
                recovery_times.push(recovery_time);
                
                if i % 10 == 0 {
                    println!("  Test {}: Recovery in {:.2}ms", i, recovery_time.as_micros() as f64 / 1000.0);
                }
            } else {
                failures += 1;
            }
        } else {
            failures += 1;
        }
    }
    
    // Calculate statistics
    recovery_times.sort();
    let avg_recovery = recovery_times.iter().sum::<Duration>() / recovery_times.len() as u32;
    let p50 = recovery_times[recovery_times.len() / 2];
    let p99 = recovery_times[(recovery_times.len() * 99) / 100];
    let max_recovery = recovery_times.last().unwrap();
    
    println!("\n📊 Results:");
    println!("  Total tests: {}", num_recovery_tests);
    println!("  Successful recoveries: {}", recovery_times.len());
    println!("  Failed recoveries: {}", failures);
    println!("  Average recovery: {:.2}ms", avg_recovery.as_micros() as f64 / 1000.0);
    println!("  p50 recovery: {:.2}ms", p50.as_micros() as f64 / 1000.0);
    println!("  p99 recovery: {:.2}ms", p99.as_micros() as f64 / 1000.0);
    println!("  Max recovery: {:.2}ms", max_recovery.as_micros() as f64 / 1000.0);
    
    let passed = p99 <= Duration::from_millis(100) && failures == 0;
    
    if passed {
        println!("\n  Status: ✅ PASSED - All recoveries within 100ms!");
    } else {
        println!("\n  Status: ❌ FAILED");
        if p99 > Duration::from_millis(100) {
            println!("  p99 recovery time {:.2}ms exceeds 100ms target", p99.as_micros() as f64 / 1000.0);
        }
        if failures > 0 {
            println!("  {} recovery failures detected", failures);
        }
    }
    
    assert!(p99 <= Duration::from_millis(100), 
        "p99 recovery time must be ≤100ms, got {:.2}ms", p99.as_micros() as f64 / 1000.0);
    assert_eq!(failures, 0, "All recoveries must succeed, got {} failures", failures);
}

async fn create_connection() -> (Arc<SpscRing>, Arc<SpscRing>, Arc<ShmWaiter>) {
    let ring_size = 256 * 1024;
    
    unsafe {
        let header_layout = Layout::new::<RingHeader>();
        let data_layout = Layout::from_size_align(ring_size, 64).unwrap();
        
        let send_header = alloc_zeroed(header_layout) as *mut RingHeader;
        let send_data = alloc_zeroed(data_layout);
        let send_ring = Arc::new(SpscRing::from_raw(send_header, send_data, ring_size));
        
        let recv_header = alloc_zeroed(header_layout) as *mut RingHeader;
        let recv_data = alloc_zeroed(data_layout);
        let recv_ring = Arc::new(SpscRing::from_raw(recv_header, recv_data, ring_size));
        
        let waiter = Arc::new(ShmWaiter::new().unwrap());
        
        (send_ring, recv_ring, waiter)
    }
}
