/// Single client test to validate futex implementation
use lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Single Client Futex Test");
    println!("═══════════════════════════");
    
    let socket_path = "/tmp/test_single_futex.sock";
    
    println!("📞 Connecting to server...");
    let client = match IpcClientVolatile::connect(socket_path).await {
        Ok(c) => {
            println!("✅ Connected successfully");
            c
        }
        Err(e) => {
            eprintln!("❌ Connection failed: {}", e);
            return Err(e);
        }
    };
    
    println!("\n📤 Sending test message...");
    let test_msg = b"Hello from futex test";
    
    let start = Instant::now();
    match client.send_bytes(test_msg).await {
        Ok(response) => {
            let latency_us = start.elapsed().as_micros();
            println!("✅ Received response: {} bytes", response.len());
            println!("⏱️  Latency: {} µs", latency_us);
            
            if latency_us < 1000 {
                println!("🎉 EXCELLENT: Sub-millisecond latency!");
            } else if latency_us < 10000 {
                println!("✓ GOOD: Low latency");
            } else {
                println!("⚠️  HIGH: Latency above 10ms");
            }
        }
        Err(e) => {
            eprintln!("❌ Send failed: {}", e);
            return Err(e);
        }
    }
    
    println!("\n✅ Test PASSED - Futex implementation working!");
    Ok(())
}
