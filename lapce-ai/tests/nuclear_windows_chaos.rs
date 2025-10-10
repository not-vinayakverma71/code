// Windows Nuclear Chaos Test - real WindowsSharedMemory (no mocks)
// Validates resilience under random failures and measures recovery metrics.

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use lapce_ai_rust::ipc::windows_shared_memory::WindowsSharedMemory;
use rand::Rng;

const AGENTS: usize = 64;         // concurrent chaos workers
const TEST_SECONDS: u64 = 20;     // duration to keep CI times reasonable
const SHM_SIZE: usize = 8 * 1024 * 1024; // 8MB per agent

#[test]
fn nuclear_windows_chaos() {
    println!("\n🌪️ WINDOWS NUCLEAR: CHAOS");
    println!("Agents: {}", AGENTS);
    println!("Duration: {}s", TEST_SECONDS);

    let stop = Arc::new(AtomicBool::new(false));
    let total_ops = Arc::new(AtomicU64::new(0));
    let failed_ops = Arc::new(AtomicU64::new(0));

    // Track recovery times (ms)
    let recovery_sum_ms = Arc::new(AtomicU64::new(0));
    let recovery_cnt = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();
    for agent_id in 0..AGENTS {
        let stop_c = stop.clone();
        let total = total_ops.clone();
        let failed = failed_ops.clone();
        let rec_sum = recovery_sum_ms.clone();
        let rec_cnt = recovery_cnt.clone();

        let handle = thread::spawn(move || {
            // Each agent uses its own SHM segment to avoid cross-thread ring buffer contention
            let shm_name = format!("lapce_windows_chaos_{}", agent_id);
            let mut shm = WindowsSharedMemory::create(&shm_name, SHM_SIZE).expect("create shm");

            let mut rng = rand::thread_rng();
            let mut last_failure: Option<Instant> = None;

            while !stop_c.load(Ordering::Relaxed) {
                total.fetch_add(1, Ordering::Relaxed);
                let op = rng.gen::<u8>() % 6;
                match op {
                    0 => {
                        // Normal small message
                        let msg = vec![0x42u8; 256];
                        if shm.write(&msg).is_ok() {
                            let _ = shm.read();
                            if last_failure.take().is_some() {
                                // recovered
                                let ms = last_failure.unwrap().elapsed().as_millis() as u64;
                                rec_sum.fetch_add(ms, Ordering::Relaxed);
                                rec_cnt.fetch_add(1, Ordering::Relaxed);
                            }
                        } else {
                            failed.fetch_add(1, Ordering::Relaxed);
                            if last_failure.is_none() { last_failure = Some(Instant::now()); }
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                    1 => {
                        // Large message (still fits)
                        let msg = vec![0xAAu8; 64 * 1024];
                        if shm.write(&msg).is_err() {
                            failed.fetch_add(1, Ordering::Relaxed);
                            if last_failure.is_none() { last_failure = Some(Instant::now()); }
                        } else {
                            let _ = shm.read();
                            if let Some(t) = last_failure.take() {
                                let ms = t.elapsed().as_millis() as u64;
                                rec_sum.fetch_add(ms, Ordering::Relaxed);
                                rec_cnt.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    2 => {
                        // Oversized message to trigger failure
                        let msg = vec![0xFFu8; SHM_SIZE];
                        if shm.write(&msg).is_err() {
                            failed.fetch_add(1, Ordering::Relaxed);
                            if last_failure.is_none() { last_failure = Some(Instant::now()); }
                        }
                    }
                    3 => {
                        // Drop and recreate (simulate abrupt connection drop)
                        drop(shm);
                        // short backoff
                        thread::sleep(Duration::from_millis(5));
                        shm = WindowsSharedMemory::create(&shm_name, SHM_SIZE).expect("recreate shm");
                        if let Some(t) = last_failure.take() {
                            let ms = t.elapsed().as_millis() as u64;
                            rec_sum.fetch_add(ms, Ordering::Relaxed);
                            rec_cnt.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    4 => {
                        // Slow response (simulate stall)
                        thread::sleep(Duration::from_millis(50));
                    }
                    _ => {
                        // Random tiny spam burst
                        for _ in 0..100 {
                            let msg = vec![0x11u8; 64];
                            let _ = shm.write(&msg);
                            let _ = shm.read();
                        }
                    }
                }
                // small jitter
                thread::sleep(Duration::from_micros((rng.gen::<u32>() % 500) as u64));
            }
        });
        handles.push(handle);
    }

    // Duration
    thread::sleep(Duration::from_secs(TEST_SECONDS));
    stop.store(true, Ordering::Relaxed);

    for h in handles { let _ = h.join(); }

    let total = total_ops.load(Ordering::Relaxed);
    let failed = failed_ops.load(Ordering::Relaxed);
    let failure_rate = if total > 0 { failed as f64 / total as f64 * 100.0 } else { 0.0 };

    let rec_count = recovery_cnt.load(Ordering::Relaxed);
    let avg_recovery_ms = if rec_count > 0 {
        recovery_sum_ms.load(Ordering::Relaxed) as f64 / rec_count as f64
    } else { 0.0 };

    println!("\n📊 RESULTS (Windows Chaos)");
    println!("Total operations: {}", total);
    println!("Failed operations: {}", failed);
    println!("Failure rate: {:.3}%", failure_rate);
    println!("Avg recovery: {:.2}ms ({} samples)", avg_recovery_ms, rec_count);

    // Export RESULT lines for CI parser
    println!("RESULT windows_chaos_failure_rate_pct={:.3}", failure_rate);
    println!("RESULT windows_chaos_avg_recovery_ms={:.2}", avg_recovery_ms);

    assert!(failure_rate < 1.0, "Failure rate too high: {:.3}%", failure_rate);
    assert!(avg_recovery_ms < 100.0, "Avg recovery too slow: {:.2}ms", avg_recovery_ms);
}
