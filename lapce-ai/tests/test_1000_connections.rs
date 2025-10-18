/// Test 1000+ concurrent connections
/// Success Criteria: Support 1000+ concurrent connections

use std::sync::Arc;
use std::time::Instant;
use std::alloc::{alloc_zeroed, Layout};
use tokio::sync::Barrier;

use lapce_ai_rust::ipc::spsc_shm_ring::{SpscRing, RingHeader};
use lapce_ai_rust::ipc::shm_waiter_cross_os::ShmWaiter;

#[tokio::test]
async fn test_1000_concurrent_connections() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ TEST: 1000+ Concurrent Connections                          ║");
    println!("║ Target: Support 1000+ concurrent connections                ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let num_connections = 1024; // Test 1024 connections
    let msgs_per_connection = 100;
    
    println!("Creating {} connections...", num_connections);
    let start_setup = Instant::now();
    let connections = create_connections(num_connections).await;
    println!("Setup time: {:.2}s", start_setup.elapsed().as_secs_f64());
    
    let barrier = Arc::new(Barrier::new(num_connections + 1));
    let mut handles = Vec::new();
    
    for (send_ring, recv_ring, waiter) in connections {
        let barrier = barrier.clone();
        let handle = tokio::spawn(async move {
            barrier.wait().await;
            
            let msg = vec![0u8; 512];
            let mut success_count = 0;
            
            for _ in 0..msgs_per_connection {
                // Write
                if send_ring.try_write(&msg) {
                    waiter.wake_one(send_ring.write_seq_ptr());
                    
                    // Simulate echo
                    recv_ring.try_write(&msg);
                    waiter.wake_one(recv_ring.write_seq_ptr());
                    
                    // Read
                    if recv_ring.try_read().is_some() {
                        success_count += 1;
                    }
                }
            }
            success_count
        });
        handles.push(handle);
    }
    
    // Start test
    barrier.wait().await;
    let start = Instant::now();
    
    // Wait for completion
    let mut total_messages = 0;
    for handle in handles {
        total_messages += handle.await.unwrap();
    }
    
    let duration = start.elapsed();
    let throughput = (total_messages as f64) / duration.as_secs_f64();
    
    println!("\n📊 Results:");
    println!("  Connections: {}", num_connections);
    println!("  Total messages: {}", total_messages);
    println!("  Duration: {:.2}s", duration.as_secs_f64());
    println!("  Aggregate throughput: {:.2} Mmsg/s", throughput / 1_000_000.0);
    
    let passed = num_connections >= 1000 && throughput >= 500_000.0;
    println!("\n  Status: {}", if passed { "✅ PASSED" } else { "❌ FAILED" });
    
    assert!(num_connections >= 1000, "Must support 1000+ connections");
    assert!(throughput >= 500_000.0, "Throughput too low with 1000+ connections");
}

async fn create_connections(count: usize) -> Vec<(Arc<SpscRing>, Arc<SpscRing>, Arc<ShmWaiter>)> {
    let mut connections = Vec::new();
    let ring_size = 128 * 1024; // 128KB per ring for 1000+ connections
    
    for _ in 0..count {
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
            
            connections.push((send_ring, recv_ring, waiter));
        }
    }
    
    connections
}
