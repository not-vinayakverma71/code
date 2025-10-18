// Windows Nuclear Transport Tests - real Windows shared memory
// Provides throughput and latency coverage using WindowsSharedMemory.

#![cfg(target_os = "windows")]

use std::time::{Duration, Instant};
use lapce_ai_rust::ipc::windows_shared_memory::SharedMemoryBuffer;

const SHM_NAME: &str = "lapce_windows_nuclear";

#[test]
fn windows_transport_throughput() {
    let mut shm = SharedMemoryBuffer::create(SHM_NAME, 8 * 1024 * 1024).expect("create shm");

    let msg = vec![0xABu8; 1024]; // 1KB messages
    let iters = 1_000_000; // 1M messages

    let start = Instant::now();
    for _ in 0..iters {
        shm.write(&msg).expect("write");
        let _ = shm.read().expect("read");
    }
    let dur = start.elapsed();

    let msgs_per_sec = (iters as f64) / dur.as_secs_f64();
    let mb_per_sec = (iters as f64 * (msg.len() as f64) / (1024.0 * 1024.0)) / dur.as_secs_f64();

    println!("RESULT throughput_msgs_per_sec={:.0}", msgs_per_sec);
    println!("RESULT throughput_MB_per_sec={:.2}", mb_per_sec);

    // Very relaxed gate for CI variability; Linux hardened pipeline enforces strict gates
    assert!(msgs_per_sec > 500_000.0, "Windows transport must exceed 500k msg/sec (got {:.0})", msgs_per_sec);
}

#[test]
fn windows_transport_latency() {
    let mut shm = SharedMemoryBuffer::create(SHM_NAME, 8 * 1024 * 1024).expect("create shm");

    let msg = vec![0xCDu8; 256];
    let samples = 50_000;
    let mut latencies = Vec::with_capacity(samples);

    // Warmup
    for _ in 0..1000 {
        shm.write(&msg).expect("write");
        let _ = shm.read().expect("read");
    }

    for _ in 0..samples {
        let t0 = Instant::now();
        shm.write(&msg).expect("write");
        let _ = shm.read().expect("read");
        latencies.push(t0.elapsed());
    }

    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p99 = latencies[latencies.len() * 99 / 100];

    println!("RESULT latency_p50_us={}", p50.as_micros());
    println!("RESULT latency_p99_us={}", p99.as_micros());

    // Relaxed gates for Windows transport
    assert!(p99.as_micros() < 50, "Windows transport p99 must be < 50us (got {}us)", p99.as_micros());
}
