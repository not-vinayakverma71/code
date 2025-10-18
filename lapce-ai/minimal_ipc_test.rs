#!/usr/bin/env rustc --edition 2021

/// Minimal IPC Performance Test - No external dependencies
/// Direct compilation: rustc --edition 2021 -O minimal_ipc_test.rs
/// Run: ./minimal_ipc_test

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Instant;

fn main() {
    println!("\n🚀 MINIMAL IPC PERFORMANCE TEST");
    println!("================================");
    
    let socket_path = "/tmp/minimal_ipc_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    let metrics = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    
    // Server thread
    let server = thread::spawn(move || {
        let listener = UnixListener::bind(socket_path).unwrap();
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1024];
        
        for _ in 0..100000 {
            stream.read_exact(&mut buf).unwrap();
            stream.write_all(&buf).unwrap();
        }
    });
    
    thread::sleep(std::time::Duration::from_millis(50));
    
    // Client
    let metrics_clone = metrics.clone();
    let client = thread::spawn(move || {
        let mut stream = UnixStream::connect("/tmp/minimal_ipc_test.sock").unwrap();
        let msg = [42u8; 1024];
        let mut resp = [0u8; 1024];
        
        for _ in 0..100000 {
            let msg_start = Instant::now();
            stream.write_all(&msg).unwrap();
            stream.read_exact(&mut resp).unwrap();
            let latency = msg_start.elapsed().as_micros() as u64;
            metrics_clone.fetch_add(latency, Ordering::Relaxed);
        }
    });
    
    client.join().unwrap();
    server.join().unwrap();
    
    let elapsed = start.elapsed();
    let total_latency = metrics.load(Ordering::Relaxed);
    let avg_latency = total_latency / 100000;
    let throughput = 100000.0 / elapsed.as_secs_f64();
    
    println!("\n📊 RESULTS:");
    println!("Duration: {:.2}s", elapsed.as_secs_f64());
    println!("Messages: 100000");
    println!("Throughput: {:.0} msg/s", throughput);
    println!("Avg latency: {} μs", avg_latency);
    
    println!("\n✅ CRITERIA CHECK:");
    if throughput > 100_000.0 {
        println!("✅ Throughput > 100K msg/s");
    } else {
        println!("❌ Throughput < 100K msg/s");
    }
    
    if avg_latency < 100 {
        println!("✅ Latency < 100 μs");
    } else {
        println!("❌ Latency > 100 μs");
    }
    
    let _ = std::fs::remove_file("/tmp/minimal_ipc_test.sock");
}
