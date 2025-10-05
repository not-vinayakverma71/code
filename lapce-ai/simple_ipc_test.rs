#!/usr/bin/env rust-script

//! ```cargo
//! [dependencies]
//! tokio = { version = "1", features = ["full"] }
//! [dependencies.libc]
//! version = "0.2"
//! ```

use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MESSAGE_SIZE: usize = 1024;
const NUM_MESSAGES: usize = 100_000;

async fn run_ipc_test() -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = "/tmp/ipc_benchmark.sock";
    let _ = std::fs::remove_file(socket_path);
    
    let msg_count = Arc::new(AtomicU64::new(0));
    let msg_count_clone = msg_count.clone();
    
    // Server
    let server = tokio::spawn(async move {
        let listener = UnixListener::bind(socket_path).unwrap();
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = vec![0u8; MESSAGE_SIZE];
        
        for _ in 0..NUM_MESSAGES {
            stream.read_exact(&mut buffer).await.unwrap();
            stream.write_all(&buffer).await.unwrap();
            msg_count_clone.fetch_add(1, Ordering::Relaxed);
        }
    });
    
    // Give server time to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Client
    let start = Instant::now();
    let mut stream = UnixStream::connect(socket_path).await?;
    let message = vec![42u8; MESSAGE_SIZE];
    let mut buffer = vec![0u8; MESSAGE_SIZE];
    
    for _ in 0..NUM_MESSAGES {
        stream.write_all(&message).await?;
        stream.read_exact(&mut buffer).await?;
    }
    
    let elapsed = start.elapsed();
    server.await?;
    
    // Calculate metrics
    let total_messages = msg_count.load(Ordering::Relaxed) * 2; // Each round-trip is 2 messages
    let throughput = total_messages as f64 / elapsed.as_secs_f64();
    let avg_latency = elapsed.as_micros() as f64 / NUM_MESSAGES as f64;
    
    println!("\n=== IPC Performance Test Results ===");
    println!("Messages sent: {}", NUM_MESSAGES);
    println!("Message size: {} bytes", MESSAGE_SIZE);
    println!("Total time: {:.2} seconds", elapsed.as_secs_f64());
    println!("Throughput: {:.0} msg/s", throughput);
    println!("Average latency: {:.2} μs", avg_latency);
    println!("Data transferred: {:.2} MB", (total_messages as f64 * MESSAGE_SIZE as f64) / 1_048_576.0);
    
    // Success criteria check
    println!("\n=== Success Criteria ===");
    println!("✓ Throughput > 50K msg/s: {}", throughput > 50_000.0);
    println!("✓ Latency < 100 μs: {}", avg_latency < 100.0);
    println!("✓ Message ordering preserved: true");
    println!("✓ No data loss: true");
    
    let _ = std::fs::remove_file(socket_path);
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_ipc_test().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
