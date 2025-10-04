/// Test for zero allocations after warmup
use lapce_ai_rust::shared_memory_ipc::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct AllocCounter;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for AllocCounter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: AllocCounter = AllocCounter;

fn main() {
    println!("TESTING ZERO ALLOCATIONS");
    println!("{}", "=".repeat(60));
    
    // Create server and warm up
    let server = SharedMemoryIpcServer::new();
    let channel_id = server.create_channel(10 * 1024 * 1024);
    server.start();
    
    // Warmup phase
    println!("\nWarmup phase (1000 messages)...");
    let data = vec![0u8; 100];
    for _ in 0..1000 {
        server.send(channel_id, &data);
    }
    
    // Reset counters after warmup
    let allocs_before = ALLOC_COUNT.load(Ordering::Relaxed);
    let deallocs_before = DEALLOC_COUNT.load(Ordering::Relaxed);
    
    // Test phase - should have minimal allocations
    println!("\nTest phase (10000 messages)...");
    let test_messages = 10000;
    
    for _ in 0..test_messages {
        server.send(channel_id, &data);
    }
    
    let allocs_after = ALLOC_COUNT.load(Ordering::Relaxed);
    let deallocs_after = DEALLOC_COUNT.load(Ordering::Relaxed);
    
    let allocs_during = allocs_after - allocs_before;
    let deallocs_during = deallocs_after - deallocs_before;
    let allocs_per_msg = allocs_during as f64 / test_messages as f64;
    
    println!("\n📊 RESULTS:");
    println!("   Messages sent: {}", test_messages);
    println!("   Allocations: {}", allocs_during);
    println!("   Deallocations: {}", deallocs_during);
    println!("   Allocations per message: {:.2}", allocs_per_msg);
    println!("   Net allocations: {}", allocs_during as i64 - deallocs_during as i64);
    
    println!("\n✅ VERDICT:");
    let zero_alloc = allocs_per_msg < 0.1; // Less than 0.1 allocations per message
    println!("   Zero allocations: {}", if zero_alloc { "✅ PASS" } else { "❌ FAIL" });
    
    if !zero_alloc {
        println!("\n❌ ALLOCATION ANALYSIS:");
        println!("   Each message causes {:.2} allocations", allocs_per_msg);
        println!("   This indicates:");
        println!("   - Buffer pooling may not be working");
        println!("   - Unnecessary string/vec allocations");
        println!("   - Need better memory management");
    }
}
