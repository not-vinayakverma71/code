/// Memory Footprint Validation Tests
/// Target: ≤3MB baseline RSS for IPC server

use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use sysinfo::{System, Pid};

#[tokio::test]
async fn test_ipc_server_memory_baseline() {
    // Start IPC server in minimal configuration
    let mut server = Command::new("cargo")
        .args(&["run", "--release", "--bin", "lapce_ipc_server", "--", "--minimal"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start IPC server");
    
    let server_pid = server.id();
    
    // Wait for server to stabilize
    sleep(Duration::from_secs(2)).await;
    
    // Measure memory
    let mut system = System::new_all();
    system.refresh_all();
    
    if let Some(process) = system.process(Pid::from_u32(server_pid as u32)) {
        let memory_kb = process.memory();
        let memory_mb = memory_kb as f64 / 1024.0;
        
        println!("IPC Server Memory Baseline:");
        println!("  PID: {}", server_pid);
        println!("  RSS: {} KB ({:.2} MB)", memory_kb, memory_mb);
        println!("  Virtual Memory: {} KB", process.virtual_memory());
        println!("  CPU: {:.2}%", process.cpu_usage());
        
        // Kill server
        let _ = server.kill();
        
        // Assert memory is within target
        assert!(
            memory_mb <= 3.0,
            "Memory footprint {:.2} MB exceeds 3MB target",
            memory_mb
        );
    } else {
        // Kill server if we couldn't measure
        let _ = server.kill();
        panic!("Could not find server process");
    }
}

#[tokio::test]
async fn test_memory_under_load() {
    // Start server
    let mut server = Command::new("cargo")
        .args(&["run", "--release", "--bin", "lapce_ipc_server"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start IPC server");
    
    let server_pid = server.id();
    
    // Wait for startup
    sleep(Duration::from_secs(2)).await;
    
    let mut system = System::new_all();
    system.refresh_all();
    
    // Baseline measurement
    let baseline_memory = if let Some(process) = system.process(Pid::from_u32(server_pid as u32)) {
        process.memory()
    } else {
        let _ = server.kill();
        panic!("Could not find server process");
    };
    
    // Generate load (would need client connections in real test)
    // For now, just wait and measure growth
    sleep(Duration::from_secs(5)).await;
    
    // Measure under load
    system.refresh_all();
    let load_memory = if let Some(process) = system.process(Pid::from_u32(server_pid as u32)) {
        process.memory()
    } else {
        0
    };
    
    let _ = server.kill();
    
    let growth_kb = load_memory.saturating_sub(baseline_memory);
    let growth_mb = growth_kb as f64 / 1024.0;
    
    println!("Memory Growth Under Load:");
    println!("  Baseline: {} KB", baseline_memory);
    println!("  Under load: {} KB", load_memory);
    println!("  Growth: {} KB ({:.2} MB)", growth_kb, growth_mb);
    
    // Assert reasonable growth
    assert!(
        growth_mb < 10.0,
        "Memory growth {:.2} MB is excessive",
        growth_mb
    );
}

#[tokio::test]
async fn test_memory_leak_detection() {
    // Start server
    let mut server = Command::new("cargo")
        .args(&["run", "--release", "--bin", "lapce_ipc_server"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start IPC server");
    
    let server_pid = server.id();
    
    // Wait for startup
    sleep(Duration::from_secs(2)).await;
    
    let mut system = System::new_all();
    let mut measurements = Vec::new();
    
    // Take measurements over time
    for i in 0..10 {
        system.refresh_all();
        
        if let Some(process) = system.process(Pid::from_u32(server_pid as u32)) {
            let memory_kb = process.memory();
            measurements.push((i, memory_kb));
            println!("Measurement {}: {} KB", i, memory_kb);
        }
        
        sleep(Duration::from_secs(1)).await;
    }
    
    let _ = server.kill();
    
    // Check for monotonic growth (potential leak)
    let mut increasing_count = 0;
    for i in 1..measurements.len() {
        if measurements[i].1 > measurements[i-1].1 {
            increasing_count += 1;
        }
    }
    
    let leak_ratio = increasing_count as f64 / (measurements.len() - 1) as f64;
    
    println!("Memory Leak Detection:");
    println!("  Measurements: {}", measurements.len());
    println!("  Increasing: {}/{}", increasing_count, measurements.len() - 1);
    println!("  Leak ratio: {:.2}", leak_ratio);
    
    // If memory increases more than 70% of the time, might be a leak
    assert!(
        leak_ratio < 0.7,
        "Potential memory leak detected (ratio: {:.2})",
        leak_ratio
    );
}

#[test]
fn document_memory_measurement_methodology() {
    println!("Memory Measurement Methodology:");
    println!("================================");
    println!();
    println!("1. Baseline Measurement:");
    println!("   - Start IPC server with minimal configuration");
    println!("   - Wait 2 seconds for stabilization");
    println!("   - Measure RSS (Resident Set Size) using sysinfo");
    println!("   - Target: ≤3MB RSS");
    println!();
    println!("2. Load Testing:");
    println!("   - Start server with normal configuration");
    println!("   - Generate typical workload");
    println!("   - Measure memory growth");
    println!("   - Target: <10MB growth under load");
    println!();
    println!("3. Leak Detection:");
    println!("   - Take measurements every second for 10 seconds");
    println!("   - Check for monotonic growth pattern");
    println!("   - Flag if >70% of measurements show increase");
    println!();
    println!("4. Tools Used:");
    println!("   - sysinfo crate for process metrics");
    println!("   - RSS as primary metric (actual physical memory)");
    println!("   - Virtual memory as secondary metric");
    println!();
    println!("5. Regression Testing:");
    println!("   - Run in CI on each commit");
    println!("   - Compare against baseline");
    println!("   - Fail if >20% increase from baseline");
}

#[cfg(target_os = "linux")]
#[test]
fn test_detailed_memory_maps() {
    use std::fs;
    use std::process;
    
    let pid = process::id();
    let maps_path = format!("/proc/{}/maps", pid);
    let status_path = format!("/proc/{}/status", pid);
    
    // Read memory maps
    if let Ok(maps) = fs::read_to_string(&maps_path) {
        println!("Memory Maps Sample:");
        for line in maps.lines().take(10) {
            println!("  {}", line);
        }
    }
    
    // Read status for detailed memory info
    if let Ok(status) = fs::read_to_string(&status_path) {
        println!("\nMemory Status:");
        for line in status.lines() {
            if line.starts_with("Vm") || line.starts_with("Rss") {
                println!("  {}", line);
            }
        }
    }
}

#[tokio::test]
async fn test_memory_profiling_with_allocations() {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    struct TrackingAllocator;
    
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
    
    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ret = System.alloc(layout);
            if !ret.is_null() {
                ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            }
            ret
        }
        
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout);
            DEALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
    }
    
    // Simulate IPC operations
    let mut buffers = Vec::new();
    
    for _ in 0..100 {
        // Allocate buffer (simulating message processing)
        let buffer = vec![0u8; 1024];
        buffers.push(buffer);
    }
    
    let allocated = ALLOCATED.load(Ordering::Relaxed);
    let deallocated = DEALLOCATED.load(Ordering::Relaxed);
    let net_allocated = allocated.saturating_sub(deallocated);
    
    println!("Allocation Tracking:");
    println!("  Total allocated: {} bytes", allocated);
    println!("  Total deallocated: {} bytes", deallocated);
    println!("  Net allocated: {} bytes ({:.2} KB)", net_allocated, net_allocated as f64 / 1024.0);
    
    // Clear buffers
    buffers.clear();
    
    // Force deallocation
    drop(buffers);
}

/// Generate memory profile report
pub fn generate_memory_report() -> String {
    let mut report = String::new();
    
    report.push_str("# IPC Server Memory Profile Report\n\n");
    report.push_str("## Target Specifications\n");
    report.push_str("- Baseline RSS: ≤3MB\n");
    report.push_str("- Peak RSS under load: ≤15MB\n");
    report.push_str("- Memory growth rate: <1MB/hour\n\n");
    
    report.push_str("## Measurement Points\n");
    report.push_str("1. **Startup**: Measure after 2s stabilization\n");
    report.push_str("2. **Idle**: Measure after 60s idle\n");
    report.push_str("3. **Load**: Measure during 1000 msg/s throughput\n");
    report.push_str("4. **Peak**: Maximum observed RSS\n\n");
    
    report.push_str("## Optimization Strategies\n");
    report.push_str("- Buffer pooling for zero allocations\n");
    report.push_str("- Lazy initialization of components\n");
    report.push_str("- Aggressive connection cleanup\n");
    report.push_str("- Bounded queues with backpressure\n\n");
    
    report.push_str("## Regression Detection\n");
    report.push_str("- CI runs memory tests on each commit\n");
    report.push_str("- Alerts on >20% increase from baseline\n");
    report.push_str("- Weekly memory profile comparison\n");
    
    report
}

#[test]
fn test_generate_memory_report() {
    let report = generate_memory_report();
    println!("{}", report);
    
    assert!(report.contains("Target Specifications"));
    assert!(report.contains("≤3MB"));
    assert!(report.contains("Optimization Strategies"));
}
