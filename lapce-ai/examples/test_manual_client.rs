/// Manual client test to debug volatile IPC
use lapce_ai_rust::ipc::ipc_client_volatile::IpcClientVolatile;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  MANUAL CLIENT TEST - Volatile IPC                      ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    
    let socket_path = "/tmp/test_eventfd.sock";
    
    println!("[MANUAL CLIENT] Connecting to {}...", socket_path);
    let client = IpcClientVolatile::connect(socket_path).await?;
    println!("[MANUAL CLIENT] ✓ Connected\n");
    
    // Send test message
    let message = b"Hello from manual test client!";
    println!("[MANUAL CLIENT] Sending message: {:?}\n", std::str::from_utf8(message)?);
    
    let response = client.send_bytes(message).await?;
    
    println!("\n[MANUAL CLIENT] ✓ Got response: {} bytes", response.len());
    println!("[MANUAL CLIENT] Response: {:?}", std::str::from_utf8(&response)?);
    
    assert_eq!(response, message, "Response should match request");
    
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  ✅ SUCCESS                                              ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    
    Ok(())
}
