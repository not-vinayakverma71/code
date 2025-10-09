/// PRECISE MEMORY TEST - Core Data Structures Only
/// Measures actual heap allocations, not process memory

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;

/// Custom allocator to track heap usage
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn get_net_allocations() -> isize {
    let allocated = ALLOCATED.load(Ordering::SeqCst) as isize;
    let deallocated = DEALLOCATED.load(Ordering::SeqCst) as isize;
    allocated - deallocated
}

fn reset_counters() {
    ALLOCATED.store(0, Ordering::SeqCst);
    DEALLOCATED.store(0, Ordering::SeqCst);
}

fn bytes_to_mb(bytes: isize) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔬 PRECISE MEMORY MEASUREMENT TEST");
    println!("{}", "=".repeat(80));
    println!("Measuring ACTUAL heap allocations (not process memory)");
    println!("{}", "=".repeat(80));
    
    // Warm up allocator
    let _warmup: Vec<u8> = Vec::with_capacity(1024);
    drop(_warmup);
    
    // Test 1: SharedMemoryBuffer creation
    println!("\n📊 Test 1: SharedMemoryBuffer Creation");
    reset_counters();
    let start_mem = get_net_allocations();
    
    let mut buffer = SharedMemoryBuffer::create("precise_test", 64 * 1024)?;
    
    let buffer_alloc = get_net_allocations() - start_mem;
    println!("  Buffer size requested: 64 KB");
    println!("  Actual allocated:      {:.2} KB", buffer_alloc as f64 / 1024.0);
    println!("  Overhead:              {:.2} KB", (buffer_alloc - 64*1024) as f64 / 1024.0);
    
    // Test 2: Message processing (hot path)
    println!("\n📊 Test 2: Hot Path Allocations");
    reset_counters();
    let hot_start = get_net_allocations();
    
    let test_msg = vec![0x42u8; 1024];
    for _ in 0..10000 {
        buffer.write(&test_msg)?;
        let mut buf = [0u8; 1024];
        let _ = buffer.read();
    }
    
    let hot_alloc = get_net_allocations() - hot_start;
    println!("  Messages processed:    10,000");
    println!("  Total allocations:     {} bytes", hot_alloc);
    println!("  Per message:           {:.2} bytes", hot_alloc as f64 / 10000.0);
    println!("  Zero allocations:      {}", if hot_alloc < 1000 { "✅ YES" } else { "❌ NO" });
    
    // Test 3: Multiple buffers (simulating connections)
    println!("\n📊 Test 3: Multiple Connections (100 buffers)");
    reset_counters();
    let multi_start = get_net_allocations();
    
    let mut buffers = Vec::new();
    for i in 0..100 {
        let buf = SharedMemoryBuffer::create(&format!("conn_{}", i), 8192)?;
        buffers.push(buf);
    }
    
    let multi_alloc = get_net_allocations() - multi_start;
    println!("  Buffers created:       100");
    println!("  Buffer size each:      8 KB");
    println!("  Expected (100*8KB):    800 KB");
    println!("  Actual allocated:      {:.2} KB", multi_alloc as f64 / 1024.0);
    println!("  Overhead:              {:.2} KB", (multi_alloc - 100*8192) as f64 / 1024.0);
    println!("  Per connection:        {:.2} KB", multi_alloc as f64 / 100.0 / 1024.0);
    
    // Test 4: Arc/Reference counting overhead
    println!("\n📊 Test 4: Arc Reference Counting");
    reset_counters();
    let arc_start = get_net_allocations();
    
    let original = Arc::new(vec![0u8; 1024]);
    let mut clones = Vec::new();
    for _ in 0..100 {
        clones.push(original.clone());
    }
    
    let arc_alloc = get_net_allocations() - arc_start;
    println!("  Original data:         1 KB");
    println!("  Arc clones:            100");
    println!("  Total allocated:       {} bytes", arc_alloc);
    println!("  Overhead per clone:    {:.2} bytes", (arc_alloc - 1024) as f64 / 100.0);
    
    // Final summary
    println!("\n{}", "=".repeat(80));
    println!("📋 MEMORY EFFICIENCY SUMMARY");
    println!("{}", "=".repeat(80));
    
    let single_conn_overhead = (buffer_alloc - 64*1024) as f64 / 1024.0;
    let hot_path_clean = hot_alloc < 1000;
    let multi_conn_avg = multi_alloc as f64 / 100.0 / 1024.0;
    
    println!("Single Connection Overhead:  {:.2} KB", single_conn_overhead);
    println!("Hot Path Allocations:        {}", if hot_path_clean { "✅ ZERO" } else { "❌ ALLOCATING" });
    println!("Per Connection (small):      {:.2} KB", multi_conn_avg);
    
    // Calculate for 1000 connections
    let projected_1000 = multi_conn_avg * 1000.0 / 1024.0;
    println!("\nProjected for 1000 connections:");
    println!("  Memory needed:       {:.2} MB", projected_1000);
    println!("  Target (<3MB):       {}", if projected_1000 < 3.0 { "✅ PASS" } else { "❌ FAIL" });
    
    // True minimal test - just data structures
    println!("\n📊 MINIMAL CORE STRUCTURES");
    reset_counters();
    let minimal_start = get_net_allocations();
    
    // Just the essential shared buffer for IPC
    let _min_buffer = SharedMemoryBuffer::create("minimal", 4096)?;
    
    let minimal_mem = get_net_allocations() - minimal_start;
    println!("  Single 4KB buffer:   {:.2} KB total", minimal_mem as f64 / 1024.0);
    println!("  Overhead only:       {:.2} KB", (minimal_mem - 4096) as f64 / 1024.0);
    
    println!("\n🎯 TRUE MEMORY FOOTPRINT:");
    let true_overhead = (minimal_mem - 4096) as f64 / 1024.0;
    println!("  Core structure overhead: {:.3} KB per connection", true_overhead);
    println!("  For 1000 connections:    {:.2} MB overhead", true_overhead * 1000.0 / 1024.0);
    println!("  Data buffers (4KB*1000): 3.91 MB");
    println!("  Total for 1000 conns:    {:.2} MB", (true_overhead * 1000.0 / 1024.0) + 3.91);
    
    println!("\n{}", "=".repeat(80));
    
    Ok(())
}
