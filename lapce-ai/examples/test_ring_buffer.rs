use lapce_ai_rust::ipc::shared_memory_complete::UltraFastSharedMemory;

fn main() -> anyhow::Result<()> {
    println!("Testing ring buffer fix...\n");
    
    let mut shm = UltraFastSharedMemory::new("test", 1024 * 1024)?;
    
    // Test all sizes
    let test_cases = vec![
        (64, "64B"),
        (128, "128B"),
        (256, "256B"),
        (512, "512B"),
        (1024, "1KB"),
        (2048, "2KB"),
        (4096, "4KB"),
        (8192, "8KB"),
        (16384, "16KB"),
    ];
    
    for (size, label) in test_cases {
        let data = vec![0xAB; size];
        
        // Write test
        if shm.write_data(&data) {
            println!("✅ {} write successful", label);
        } else {
            println!("❌ {} write FAILED", label);
        }
    }
    
    println!("\nNow testing read back...");
    
    // Create consumer
    let mut consumer = UltraFastSharedMemory::open("test", 1024 * 1024)?;
    
    for (size, label) in vec![(64, "64B"), (128, "128B"), (256, "256B"), (512, "512B"), 
                               (1024, "1KB"), (2048, "2KB"), (4096, "4KB"), 
                               (8192, "8KB"), (16384, "16KB")] {
        if let Some(data) = consumer.read_zero_copy() {
            if data.len() == size && data[0] == 0xAB {
                println!("✅ {} read successful", label);
                consumer.commit_read(data.len());
            } else {
                println!("❌ {} read wrong size ({}) or value", label, data.len());
            }
        } else {
            println!("❌ {} read failed", label);
        }
    }
    
    Ok(())
}
