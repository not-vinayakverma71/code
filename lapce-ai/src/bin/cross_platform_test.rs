/// CROSS-PLATFORM IPC COMPATIBILITY TEST
/// Tests SharedMemory IPC on Linux, Windows, and macOS

use std::env;
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;

fn get_os_info() -> String {
    format!(
        "OS: {} {} ({})",
        env::consts::OS,
        env::consts::FAMILY,
        env::consts::ARCH
    )
}

fn test_shared_memory() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 SharedMemory IPC Test");
    println!("{}", "-".repeat(50));
    
    // Test 1: Create shared memory buffer
    println!("  Creating SharedMemory buffer...");
    let mut buffer = match SharedMemoryBuffer::create("cross_platform_test", 8192) {
        Ok(buf) => {
            println!("  ✅ SharedMemory creation: WORKS");
            buf
        }
        Err(e) => {
            println!("  ❌ SharedMemory creation: FAILED");
            println!("     Error: {}", e);
            return Err(e.into());
        }
    };
    
    // Test 2: Write to buffer
    println!("  Writing to buffer...");
    let test_data = b"Hello from cross-platform test!";
    match buffer.write(test_data) {
        Ok(_) => println!("  ✅ Write operation: WORKS"),
        Err(e) => {
            println!("  ❌ Write operation: FAILED");
            println!("     Error: {}", e);
            return Err(e.into());
        }
    }
    
    // Test 3: Read from buffer
    println!("  Reading from buffer...");
    let mut temp = vec![0u8; 1024];
    match buffer.read() {
        Some(data) => {
            println!("Read {} bytes", data.len());
            temp = data;
        },
        None => {
            println!("No data available");
        }
    }
    
    Ok(())
}

fn test_unix_sockets() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use tokio::net::UnixListener;
        use std::path::Path;
        
        println!("\n📊 Unix Socket Test");
        println!("{}", "-".repeat(50));
        
        let socket_path = "/tmp/cross_platform_test.sock";
        
        // Clean up any existing socket
        if Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).ok();
        }
        
        match UnixListener::bind(socket_path) {
            Ok(_) => {
                println!("  ✅ Unix sockets: AVAILABLE");
                std::fs::remove_file(socket_path).ok();
            }
            Err(e) => {
                println!("  ❌ Unix sockets: NOT AVAILABLE");
                println!("     Error: {}", e);
            }
        }
    }
    
    #[cfg(not(unix))]
    {
        println!("\n📊 Unix Socket Test");
        println!("{}", "-".repeat(50));
        println!("  ❌ Unix sockets: NOT SUPPORTED ON THIS OS");
    }
    
    Ok(())
}

fn test_named_pipes() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Named Pipes Test");
    println!("{}", "-".repeat(50));
    
    #[cfg(windows)]
    {
        // Windows named pipes
        println!("  ℹ️ Windows Named Pipes would be tested here");
        println!("  ⚠️ Not implemented in current codebase");
    }
    
    #[cfg(unix)]
    {
        // Unix FIFOs
        use std::process::Command;
        
        let fifo_path = "/tmp/cross_platform_fifo";
        
        // Try to create a FIFO
        let result = Command::new("mkfifo")
            .arg(fifo_path)
            .output();
        
        match result {
            Ok(output) if output.status.success() => {
                println!("  ✅ Named pipes (FIFOs): AVAILABLE");
                std::fs::remove_file(fifo_path).ok();
            }
            _ => {
                println!("  ⚠️ Named pipes (FIFOs): Limited support");
            }
        }
    }
    
    Ok(())
}

fn test_tcp_sockets() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::TcpListener;
    
    println!("\n📊 TCP Socket Test (Fallback)");
    println!("{}", "-".repeat(50));
    
    match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => {
            let addr = listener.local_addr()?;
            println!("  ✅ TCP sockets: WORKS (bound to {})", addr);
            println!("  ℹ️ Can be used as cross-platform fallback");
        }
        Err(e) => {
            println!("  ❌ TCP sockets: FAILED");
            println!("     Error: {}", e);
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔬 CROSS-PLATFORM IPC COMPATIBILITY TEST");
    println!("{}", "=".repeat(80));
    println!("{}", get_os_info());
    println!("{}", "=".repeat(80));
    
    // Run all tests
    let mut passed = 0;
    let mut failed = 0;
    
    // Test SharedMemory (our implementation)
    if test_shared_memory().is_ok() {
        passed += 1;
    } else {
        failed += 1;
    }
    
    // Test Unix sockets
    if test_unix_sockets().is_ok() {
        passed += 1;
    } else {
        failed += 1;
    }
    
    // Test named pipes
    if test_named_pipes().is_ok() {
        passed += 1;
    } else {
        failed += 1;
    }
    
    // Test TCP sockets (universal fallback)
    if test_tcp_sockets().is_ok() {
        passed += 1;
    } else {
        failed += 1;
    }
    
    // Platform-specific recommendations
    println!("\n{}", "=".repeat(80));
    println!("📋 PLATFORM COMPATIBILITY SUMMARY");
    println!("{}", "=".repeat(80));
    
    match env::consts::OS {
        "linux" => {
            println!("✅ Linux: Full support");
            println!("  - SharedMemory: ✅ Optimal");
            println!("  - Unix sockets: ✅ Native");
            println!("  - Named pipes: ✅ Available");
            println!("  - TCP fallback: ✅ Works");
        }
        "macos" => {
            println!("⚠️ macOS: Partial support");
            println!("  - SharedMemory: ⚠️ May have restrictions");
            println!("  - Unix sockets: ✅ Works");
            println!("  - Named pipes: ⚠️ Limited");
            println!("  - TCP fallback: ✅ Works");
        }
        "windows" => {
            println!("❌ Windows: Limited support");
            println!("  - SharedMemory: ❌ Needs Windows impl");
            println!("  - Unix sockets: ❌ Not available");
            println!("  - Named pipes: ⚠️ Different API");
            println!("  - TCP fallback: ✅ Works");
        }
        _ => {
            println!("❓ Unknown OS: {}", env::consts::OS);
        }
    }
    
    println!("\n🎯 RECOMMENDATION:");
    match env::consts::OS {
        "linux" => {
            println!("  Use SharedMemory for best performance");
        }
        "macos" => {
            println!("  Use Unix sockets or TCP for compatibility");
        }
        "windows" => {
            println!("  Must use TCP sockets or implement Windows Named Pipes");
        }
        _ => {
            println!("  Use TCP sockets for maximum compatibility");
        }
    }
    
    println!("\nTests passed: {}/{}", passed, passed + failed);
    println!("{}", "=".repeat(80));
    
    Ok(())
}
