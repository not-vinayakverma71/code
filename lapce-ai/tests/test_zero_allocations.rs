/// Test zero allocations in hot path
/// Success Criteria: No heap allocations in hot path

use std::sync::Arc;
use std::alloc::{alloc_zeroed, Layout, GlobalAlloc, System};
use std::sync::atomic::{AtomicU64, Ordering};

use lapce_ai_rust::ipc::spsc_shm_ring::{SpscRing, RingHeader};
use lapce_ai_rust::ipc::shm_waiter_cross_os::ShmWaiter;

// Allocation tracking allocator
struct TrackingAllocator {
    inner: System,
    allocations: AtomicU64,
}

impl TrackingAllocator {
    const fn new() -> Self {
        Self {
            inner: System,
            allocations: AtomicU64::new(0),
        }
    }
    
    fn reset(&self) {
        self.allocations.store(0, Ordering::SeqCst);
    }
    
    fn allocation_count(&self) -> u64 {
        self.allocations.load(Ordering::SeqCst)
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocations.fetch_add(1, Ordering::SeqCst);
        self.inner.alloc(layout)
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();

#[tokio::test]
async fn test_zero_allocations_hot_path() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║ TEST: Zero Allocations in Hot Path                          ║");
    println!("║ Target: No heap allocations in hot path                     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Setup connection (allocations allowed here)
    let (send_ring, recv_ring, waiter) = create_test_connection().await;
    let msg = vec![0u8; 1024];
    
    // Warmup - ensure everything is initialized
    for _ in 0..100 {
        send_ring.try_write(&msg);
        waiter.wake_one(send_ring.write_seq_ptr());
        recv_ring.try_read();
    }
    
    // Reset allocation counter
    ALLOCATOR.reset();
    let allocs_before = ALLOCATOR.allocation_count();
    
    println!("Testing hot path (10,000 operations)...");
    
    // HOT PATH: This should have ZERO allocations
    for _ in 0..10_000 {
        // Write to ring (should be zero-copy)
        send_ring.try_write(&msg);
        
        // Wake waiter (should not allocate)
        waiter.wake_one(send_ring.write_seq_ptr());
        
        // Read from ring (should be zero-copy)
        recv_ring.try_read();
    }
    
    let allocs_after = ALLOCATOR.allocation_count();
    let hot_path_allocations = allocs_after - allocs_before;
    
    println!("\n📊 Results:");
    println!("  Operations: 10,000");
    println!("  Allocations in hot path: {}", hot_path_allocations);
    
    let passed = hot_path_allocations == 0;
    if passed {
        println!("\n  Status: ✅ PASSED - Zero allocations!");
    } else {
        println!("\n  Status: ❌ FAILED - {} allocations detected", hot_path_allocations);
    }
    
    if !passed {
        println!("\n  ⚠️  Hot path is NOT allocation-free!");
        println!("  This violates the 'Zero Allocations' success criterion.");
    }
    
    assert_eq!(hot_path_allocations, 0, 
        "Hot path must have zero allocations, found {}", hot_path_allocations);
}

async fn create_test_connection() -> (Arc<SpscRing>, Arc<SpscRing>, Arc<ShmWaiter>) {
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
