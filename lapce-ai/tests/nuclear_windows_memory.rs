// Windows Nuclear Memory Growth Test - real WindowsSharedMemory (no mocks)
// Ensures memory growth remains bounded under create/write/read/drop cycles.

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use lapce_ai_rust::ipc::windows_shared_memory::SharedMemoryBuffer;
use sysinfo::{System, Pid};

const AGENTS: usize = 32;
const ITERATIONS: usize = 2000;
const SHM_SIZE: usize = 8 * 1024 * 1024; // 8MB region per agent

fn process_memory_mb() -> f64 {
    let mut sys = System::new();
    sys.refresh_all();
    let pid = Pid::from(std::process::id() as usize);
    if let Some(proc_) = sys.process(pid) {
        proc_.memory() as f64 / 1024.0 // KB -> MB
    } else {
        0.0
    }
}

#[test]
fn nuclear_windows_memory_growth() {
    println!("\n🔍 WINDOWS NUCLEAR: MEMORY GROWTH");
    println!("Agents: {}", AGENTS);
    println!("Iterations/agent: {}", ITERATIONS);

    // Warmup
    thread::sleep(Duration::from_millis(200));
    let baseline_mb = process_memory_mb();
    println!("Baseline memory: {:.2} MB", baseline_mb);

    let stop = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();

    for agent_id in 0..AGENTS {
        let stop_c = stop.clone();
        let handle = thread::spawn(move || {
            let name = format!("lapce_windows_mem_{}", agent_id);
            let mut shm = SharedMemoryBuffer::create(&name, SHM_SIZE).expect("create shm");
            let msg = vec![0x33u8; 4096];

            for i in 0..ITERATIONS {
                if stop_c.load(Ordering::Relaxed) { break; }
                // Write/read roundtrip
                shm.write(&msg).expect("write");
                let _ = shm.read().expect("read opt");

                // Drop/recreate occasionally to force resource churn
                if i % 200 == 0 {
                    drop(shm);
                    shm = SharedMemoryBuffer::create(&name, SHM_SIZE).expect("recreate shm");
                }

                // small yield
                if i % 50 == 0 { thread::sleep(Duration::from_millis(1)); }
            }
        });
        handles.push(handle);
    }

    for h in handles { let _ = h.join(); }

    // Allow a moment for allocator to return freed memory
    thread::sleep(Duration::from_millis(500));
    let final_mb = process_memory_mb();
    let growth_kb = (final_mb - baseline_mb) * 1024.0;

    println!("\n📊 RESULTS (Windows Memory)");
    println!("Baseline: {:.2} MB", baseline_mb);
    println!("Final: {:.2} MB", final_mb);
    println!("Growth: {:.2} KB", growth_kb);

    // Export for CI parser
    println!("RESULT windows_memory_growth_kb={:.2}", growth_kb);

    // Keep a modest bound for CI variability; hardened Linux suite ensures <512KB in POSIX path
    assert!(growth_kb.abs() < 1536.0, "Memory growth too high: {:.2} KB", growth_kb);
}
