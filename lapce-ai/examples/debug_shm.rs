use lapce_ai_rust::ultra_fast_shared_memory::UltraFastSharedMemory;

fn main() -> anyhow::Result<()> {
    println!("Debug SharedMemory Issues\n");
    
    // Test different sizes
    let mut shm = UltraFastSharedMemory::new("debug", 1024 * 1024)?; // 1MB
    
    println!("Buffer size: 1MB");
    
    // Test 64B
    let data_64 = vec![0xAB; 64];
    if shm.write_data(&data_64) {
        println!("✓ 64B write successful");
    } else {
        println!("✗ 64B write failed");
    }
    
    // Test 256B
    let data_256 = vec![0xCD; 256];
    if shm.write_data(&data_256) {
        println!("✓ 256B write successful");
    } else {
        println!("✗ 256B write failed");
    }
    
    // Test 1KB
    let data_1k = vec![0xEF; 1024];
    if shm.write_data(&data_1k) {
        println!("✓ 1KB write successful");
    } else {
        println!("✗ 1KB write failed");
    }
    
    // Test 4KB
    let data_4k = vec![0x12; 4096];
    if shm.write_data(&data_4k) {
        println!("✓ 4KB write successful");
    } else {
        println!("✗ 4KB write failed");
    }
    
    // Test consumer
    println!("\nCreating consumer...");
    let mut consumer = UltraFastSharedMemory::open("debug", 1024 * 1024)?;
    
    if let Some(data) = consumer.read_zero_copy() {
        println!("✓ Consumer read {} bytes", data.len());
        consumer.commit_read(data.len());
    } else {
        println!("✗ Consumer read failed");
    }
    
    Ok(())
}
