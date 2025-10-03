fn main() {
    println!("Testing basic shared memory...");
    
    // Simple test to see if RealSharedMemory works
    use lapce_ai_rust::real_shared_memory::RealSharedMemory;
    
    match RealSharedMemory::create("simple", 1024 * 1024) {
        Ok(mut shm) => {
            println!("✓ Created 1MB shared memory");
            
            let data = vec![0xAB; 64];
            if shm.write(&data) {
                println!("✓ Wrote 64 bytes");
            } else {
                println!("✗ Write failed");
            }
        }
        Err(e) => {
            println!("✗ Failed to create shared memory: {}", e);
        }
    }
}
