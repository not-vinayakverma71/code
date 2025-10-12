use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let base_path = format!("/tmp/ipc_min_{}", std::process::id());
    let listener = Arc::new(SharedMemoryListener::bind(&base_path).await?);
    
    // Simple echo server - 1 connection only
    let _srv = {
        let listener = listener.clone();
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                println!("[SERVER] Connection accepted");
                let mut buf = vec![0u8; 1024];
                let mut count = 0;
                loop {
                    match stream.read_exact(&mut buf).await {
                        Ok(_) => {
                            count += 1;
                            if count % 100 == 0 {
                                println!("[SERVER] Echoed {} messages", count);
                            }
                            if stream.write_all(&buf).await.is_err() {
                                println!("[SERVER] Write failed");
                                break;
                            }
                        }
                        Err(e) => {
                            println!("[SERVER] Read error: {}", e);
                            break;
                        }
                    }
                }
            }
        })
    };
    
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    
    // Single client - send 500 messages
    println!("[CLIENT] Connecting...");
    let mut stream = SharedMemoryStream::connect(&base_path).await?;
    println!("[CLIENT] Connected");
    
    let buf = vec![0xABu8; 1024];
    let mut recv = vec![0u8; 1024];
    
    let start = Instant::now();
    for i in 0..500 {
        if i % 100 == 0 {
            println!("[CLIENT] Sending message {}", i);
        }
        
        match stream.write_all(&buf).await {
            Ok(_) => {},
            Err(e) => {
                println!("[CLIENT] Write failed at {}: {}", i, e);
                break;
            }
        }
        
        match stream.read_exact(&mut recv).await {
            Ok(_) => {},
            Err(e) => {
                println!("[CLIENT] Read failed at {}: {}", i, e);
                break;
            }
        }
    }
    
    let elapsed = start.elapsed();
    println!("\n✅ Completed 500 round-trips in {:.2}s", elapsed.as_secs_f64());
    println!("Throughput: {:.0} msg/s", 500.0 / elapsed.as_secs_f64());
    
    Ok(())
}
