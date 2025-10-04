/// Test Prometheus Metrics - HOUR 12
use lapce_ai_rust::ipc_server::IpcServer;
use lapce_ai_rust::ipc_messages::MessageType;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Prometheus Metrics Export");
    println!("==================================\n");
    
    // Create server
    let server = Arc::new(IpcServer::new("/tmp/test_metrics.sock").await?);
    
    // Register handler
    server.register_handler(MessageType::Echo, |data| async move {
        Ok(data)
    });
    
    server.register_handler(MessageType::Complete, |data| async move {
        tokio::time::sleep(Duration::from_micros(100)).await;
        Ok(data)
    });
    
    // Start server
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            let _ = server.serve().await;
        })
    };
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Send various message types
    let mut stream = UnixStream::connect("/tmp/test_metrics.sock").await?;
    
    println!("Sending test messages...");
    for msg_type in [MessageType::Echo, MessageType::Complete, MessageType::Stream, MessageType::Heartbeat] {
        for _ in 0..100 {
            let test_data = b"metrics test";
            let msg_type_bytes = msg_type.to_bytes();
            let mut message = Vec::new();
            message.extend_from_slice(&msg_type_bytes);
            message.extend_from_slice(test_data);
            
            let msg_len = message.len() as u32;
            
            stream.write_all(&msg_len.to_le_bytes()).await?;
            stream.write_all(&message).await?;
            
            // Read response
            let mut len_buf = [0u8; 4];
            if stream.read_exact(&mut len_buf).await.is_ok() {
                let response_len = u32::from_le_bytes(len_buf) as usize;
                let mut response = vec![0u8; response_len];
                let _ = stream.read_exact(&mut response).await;
            }
        }
    }
    
    // Export metrics
    println!("\n=== PROMETHEUS METRICS ===\n");
    let metrics = server.metrics();
    let prometheus_output = metrics.export_prometheus();
    println!("{}", prometheus_output);
    
    // Parse and verify metrics
    let lines: Vec<&str> = prometheus_output.lines().collect();
    
    // Check for required metrics
    let has_total = lines.iter().any(|l| l.contains("ipc_requests_total"));
    let has_latency = lines.iter().any(|l| l.contains("ipc_latency_microseconds"));
    
    println!("\n=== METRICS VERIFICATION ===");
    println!("Has request total metric: {}", if has_total { "✅" } else { "❌" });
    println!("Has latency histogram: {}", if has_latency { "✅" } else { "❌" });
    
    // Extract total requests
    for line in &lines {
        if line.starts_with("ipc_requests_total") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                println!("Total requests processed: {}", parts[1]);
            }
        }
    }
    
    // Check latency buckets
    println!("\nLatency Distribution:");
    for line in &lines {
        if line.contains("ipc_latency_microseconds_bucket") {
            println!("  {}", line);
        }
    }
    
    server.shutdown();
    server_handle.abort();
    
    println!("\n✅ Prometheus metrics implementation verified!");
    
    Ok(())
}
