/// Test Zero Allocations - HOUR 9
use lapce_ai_rust::zero_copy_ipc::ZeroCopyIpcServer;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{GlobalAlloc, Layout, System};
use std::time::Instant;

/// Custom allocator to track allocations
struct TrackingAllocator {
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
    bytes_allocated: AtomicUsize,
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.bytes_allocated.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator {
    allocations: AtomicUsize::new(0),
    deallocations: AtomicUsize::new(0),
    bytes_allocated: AtomicUsize::new(0),
};

fn main() {
    println!("Testing Zero Allocations in Hot Path");
    println!("=====================================\n");
    
    // Setup phase - allocations allowed
    println!("1. SETUP PHASE (allocations allowed)");
    let allocs_before_setup = ALLOCATOR.allocations.load(Ordering::Relaxed);
    
    let mut server = ZeroCopyIpcServer::new();
    let channel_id = server.create_channel(20); // 1MB buffer
    let data = vec![0u8; 100]; // Test message
    let mut recv_buffer = Vec::with_capacity(200);
    
    // Warm up - fill buffers
    for _ in 0..1000 {
        server.send(channel_id, &data);
        server.recv(channel_id, &mut recv_buffer);
    }
    
    let allocs_after_setup = ALLOCATOR.allocations.load(Ordering::Relaxed);
    println!("   Setup allocations: {}", allocs_after_setup - allocs_before_setup);
    println!("   Total bytes allocated: {} KB", ALLOCATOR.bytes_allocated.load(Ordering::Relaxed) / 1024);
    
    // Hot path test - no allocations should occur
    println!("\n2. HOT PATH TEST (zero allocations expected)");
    let allocs_before_hot = ALLOCATOR.allocations.load(Ordering::Relaxed);
    let iterations = 1_000_000;
    
    let start = Instant::now();
    
    for _ in 0..iterations {
        // Send should not allocate
        server.send(channel_id, &data);
    }
    
    let elapsed = start.elapsed();
    let allocs_after_hot = ALLOCATOR.allocations.load(Ordering::Relaxed);
    let hot_path_allocations = allocs_after_hot - allocs_before_hot;
    
    println!("   Messages sent: {}", iterations);
    println!("   Time: {:?}", elapsed);
    println!("   Throughput: {:.0} msg/sec", iterations as f64 / elapsed.as_secs_f64());
    println!("   Allocations in hot path: {}", hot_path_allocations);
    println!("   Allocations per message: {:.4}", hot_path_allocations as f64 / iterations as f64);
    
    if hot_path_allocations == 0 {
        println!("   Status: ✅ PASS - Zero allocations!");
    } else if hot_path_allocations < iterations / 1000 {
        println!("   Status: ⚠️ PARTIAL - Very few allocations ({:.2} per 1000 messages)", 
                 (hot_path_allocations as f64 / iterations as f64) * 1000.0);
    } else {
        println!("   Status: ❌ FAIL - Too many allocations");
    }
    
    // Test receive path
    println!("\n3. RECEIVE PATH TEST");
    
    // First fill the buffer
    for _ in 0..1000 {
        server.send(channel_id, &data);
    }
    
    // Now test receive
    let recv_allocs_before = ALLOCATOR.allocations.load(Ordering::Relaxed);
    let mut received = 0;
    recv_buffer.clear();
    
    for _ in 0..1000 {
        recv_buffer.clear(); // Reuse the buffer
        if server.recv(channel_id, &mut recv_buffer) {
            received += 1;
        } else {
            break;
        }
    }
    
    let recv_allocs_after = ALLOCATOR.allocations.load(Ordering::Relaxed);
    let recv_allocations = recv_allocs_after - recv_allocs_before;
    
    println!("   Messages received: {}", received);
    println!("   Allocations in receive: {}", recv_allocations);
    println!("   Allocations per message: {:.4}", recv_allocations as f64 / received as f64);
    
    if recv_allocations == 0 {
        println!("   Status: ✅ PASS - Zero allocations!");
    } else if recv_allocations < received / 100 {
        println!("   Status: ⚠️ PARTIAL - Few allocations");
    } else {
        println!("   Status: ❌ FAIL - Too many allocations");
    }
    
    // Summary
    println!("\n=== REQUIREMENT #5: ZERO ALLOCATIONS ===");
    println!("Target: No heap allocations in hot path");
    println!("Send path allocations: {}", hot_path_allocations);
    println!("Recv path allocations: {}", recv_allocations);
    
    if hot_path_allocations == 0 && recv_allocations == 0 {
        println!("Overall Status: ✅ PASS - True zero allocations!");
    } else if hot_path_allocations < 10 && recv_allocations < 10 {
        println!("Overall Status: ⚠️ PARTIAL - Nearly zero allocations");
    } else {
        println!("Overall Status: ❌ FAIL - Allocations detected in hot path");
    }
    
    println!("\nTotal allocations: {}", ALLOCATOR.allocations.load(Ordering::Relaxed));
    println!("Total deallocations: {}", ALLOCATOR.deallocations.load(Ordering::Relaxed));
    println!("Total bytes allocated: {} KB", ALLOCATOR.bytes_allocated.load(Ordering::Relaxed) / 1024);
}
