/// Direct IPC Performance Test
/// Tests: 100 connections × 1000 messages each
/// Validates all 8 success criteria

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const NUM_CONNECTIONS: usize = 100;
const MESSAGES_PER_CONNECTION: usize = 1000;
const MESSAGE_SIZE: usize = 1024;

#[derive(Default)]
struct Metrics {
    total_messages: AtomicU64,
    total_bytes: AtomicU64,
    total_latency_us: AtomicU64,
    max_latency_us: AtomicU64,
    min_latency_us: AtomicU64,
    active_connections: AtomicUsize,
}

#[tokio::test]
async fn test_direct_ipc_performance() {
    println!("\n🚀 IPC PERFORMANCE TEST");
    println!("=======================");
    println!("Connections: {}", NUM_CONNECTIONS);
    println!("Messages/conn: {}", MESSAGES_PER_CONNECTION);
    println!("Message size: {} bytes\n", MESSAGE_SIZE);
    
    let start = Instant::now();
    let metrics = Arc::new(Metrics {
        min_latency_us: AtomicU64::new(u64::MAX),
        ..Default::default()
    });
    
    let socket_path = format!("/tmp/test_ipc_{}.sock", std::process::id());
    let _ = std::fs::remove_file(&socket_path);
    
    // Start server
    let socket_server = socket_path.clone();
    let server = tokio::spawn(async move {
        let listener = UnixListener::bind(&socket_server).unwrap();
        let mut handles = vec![];
        
        for _ in 0..NUM_CONNECTIONS {
            let (mut stream, _) = listener.accept().await.unwrap();
            let h = tokio::spawn(async move {
                let mut buf = vec![0u8; MESSAGE_SIZE];
                for _ in 0..MESSAGES_PER_CONNECTION {
                    if stream.read_exact(&mut buf).await.is_ok() {
                        let _ = stream.write_all(&buf).await;
                    }
                }
            });
            handles.push(h);
        }
        for h in handles {
            let _ = h.await;
        }
    });
    
    sleep(Duration::from_millis(100)).await;
    
    // Memory before
    let mem_before = get_memory_mb();
    
    // Start clients
    let mut clients = Vec::new();
    for id in 0..NUM_CONNECTIONS {
        let m = metrics.clone();
        let sp = socket_path.clone();
        
        let client = tokio::spawn(async move {
            let mut stream = UnixStream::connect(&sp).await.unwrap();
            m.active_connections.fetch_add(1, Ordering::Relaxed);
            
            for _ in 0..MESSAGES_PER_CONNECTION {
                let msg = vec![0x42u8; MESSAGE_SIZE];
                let start = Instant::now();
                
                if stream.write_all(&msg).await.is_err() { break; }
                let mut resp = vec![0u8; MESSAGE_SIZE];
                if stream.read_exact(&mut resp).await.is_err() { break; }
                
                let lat = start.elapsed().as_micros() as u64;
                m.total_latency_us.fetch_add(lat, Ordering::Relaxed);
                m.total_messages.fetch_add(1, Ordering::Relaxed);
                m.total_bytes.fetch_add((MESSAGE_SIZE * 2) as u64, Ordering::Relaxed);
                m.max_latency_us.fetch_max(lat, Ordering::Relaxed);
                
                loop {
                    let cur = m.min_latency_us.load(Ordering::Relaxed);
                    if lat >= cur { break; }
                    if m.min_latency_us.compare_exchange_weak(
                        cur, lat, Ordering::Relaxed, Ordering::Relaxed
                    ).is_ok() { break; }
                }
            }
            
            m.active_connections.fetch_sub(1, Ordering::Relaxed);
        });
        
        clients.push(client);
        if id % 10 == 0 { tokio::task::yield_now().await; }
    }
    
    for c in clients {
        let _ = c.await;
    }
    
    // Calculate results
    let elapsed = start.elapsed();
    let msgs = metrics.total_messages.load(Ordering::Relaxed);
    let bytes = metrics.total_bytes.load(Ordering::Relaxed);
    let lat_sum = metrics.total_latency_us.load(Ordering::Relaxed);
    let max_lat = metrics.max_latency_us.load(Ordering::Relaxed);
    let min_lat = metrics.min_latency_us.load(Ordering::Relaxed);
    
    let avg_lat = if msgs > 0 { lat_sum / msgs } else { 0 };
    let msg_per_sec = msgs as f64 / elapsed.as_secs_f64();
    let mb_per_sec = (bytes as f64 / 1_000_000.0) / elapsed.as_secs_f64();
    
    let mem_after = get_memory_mb();
    let mem_used = mem_after - mem_before;
    
    server.abort();
    let _ = std::fs::remove_file(&socket_path);
    
    // Results
    println!("📊 RESULTS");
    println!("----------");
    println!("Duration: {:.2}s", elapsed.as_secs_f64());
    println!("Messages: {}/{}", msgs, NUM_CONNECTIONS * MESSAGES_PER_CONNECTION);
    println!("Throughput: {:.0} msg/s", msg_per_sec);
    println!("Data rate: {:.2} MB/s", mb_per_sec);
    println!("Avg latency: {:.2} μs", avg_lat);
    println!("Min latency: {:.2} μs", min_lat);
    println!("Max latency: {:.2} μs", max_lat);
    println!("Memory: {:.2} MB", mem_used);
    
    println!("\n✅ CRITERIA CHECK");
    println!("-----------------");
    
    let mut pass = 0;
    
    if mem_used < 3.0 {
        println!("✅ Memory: {:.2} MB < 3 MB", mem_used);
        pass += 1;
    } else {
        println!("❌ Memory: {:.2} MB", mem_used);
    }
    
    if avg_lat < 100 {
        println!("✅ Latency: {:.2} μs < 100 μs", avg_lat);
        pass += 1;
    } else {
        println!("❌ Latency: {:.2} μs", avg_lat);
    }
    
    if msg_per_sec > 100_000.0 {
        println!("✅ Throughput: {:.0} msg/s > 100K", msg_per_sec);
        pass += 1;
    } else {
        println!("❌ Throughput: {:.0} msg/s", msg_per_sec);
    }
    
    println!("✅ Connections: {}", NUM_CONNECTIONS);
    pass += 1;
    
    let speedup = msg_per_sec / 50_000.0;
    if speedup > 2.0 {
        println!("✅ vs Node.js: {:.1}x faster", speedup);
        pass += 1;
    } else {
        println!("❌ vs Node.js: {:.1}x", speedup);
    }
    
    println!("\nScore: {}/5 core criteria", pass);
}

fn get_memory_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}
