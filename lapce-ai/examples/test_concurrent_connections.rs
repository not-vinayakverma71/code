/// Test 1000+ concurrent connections
use lapce_ai_rust::shared_memory_complete::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    println!("TESTING 1000+ CONCURRENT CONNECTIONS");
    println!("{}", "=".repeat(60));
    
    let server = Arc::new(SharedMemoryIpcServer::new());
    let connected = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let target_connections = 1000;
    
    server.start();
    
    println!("\nCreating {} concurrent connections...", target_connections);
    let start = Instant::now();
    
    let mut handles = vec![];
    
    for i in 0..target_connections {
        let server_clone = server.clone();
        let connected_clone = connected.clone();
        let failed_clone = failed.clone();
        
        let handle = thread::spawn(move || {
            // Each connection gets its own channel
            let channel_id = server_clone.create_channel(1024 * 1024); // 1MB per connection
            
            // Try to send messages
            let test_data = format!("Connection {} test", i).into_bytes();
            let mut success = true;
            
            for j in 0..100 {
                if !server_clone.send(channel_id, &test_data) {
                    success = false;
                    break;
                }
                
                // Simulate some work
                thread::sleep(Duration::from_micros(10));
            }
            
            if success {
                connected_clone.fetch_add(1, Ordering::Relaxed);
            } else {
                failed_clone.fetch_add(1, Ordering::Relaxed);
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    let successful = connected.load(Ordering::Relaxed);
    let failed_count = failed.load(Ordering::Relaxed);
    
    println!("\n📊 RESULTS:");
    println!("   Target connections: {}", target_connections);
    println!("   Successful: {}", successful);
    println!("   Failed: {}", failed_count);
    println!("   Success rate: {:.1}%", (successful as f64 / target_connections as f64) * 100.0);
    println!("   Time taken: {:?}", elapsed);
    println!("   Avg connection time: {:?}", elapsed / target_connections as u32);
    
    println!("\n✅ VERDICT:");
    let passed = successful >= 1000;
    println!("   1000+ connections: {}", if passed { "✅ PASS" } else { "❌ FAIL" });
    
    if !passed {
        println!("\n❌ FAILURE ANALYSIS:");
        println!("   Only {} of {} connections succeeded", successful, target_connections);
        println!("   This may be due to:");
        println!("   - Memory limitations");
        println!("   - Thread pool exhaustion");
        println!("   - Buffer capacity issues");
    }
}
