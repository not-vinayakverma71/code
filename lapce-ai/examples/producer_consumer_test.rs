use lapce_ai_rust::working_shared_memory::WorkingSharedMemory;
use std::time::Instant;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

fn main() -> anyhow::Result<()> {
    println!("Producer-Consumer Performance Test\n");
    
    // Shared counters
    let produced = Arc::new(AtomicU64::new(0));
    let consumed = Arc::new(AtomicU64::new(0));
    
    let test_cases = vec![
        (64, 1_000_000, "64B × 1M"),
        (256, 500_000, "256B × 500K"),
        (1024, 200_000, "1KB × 200K"),
        (4096, 50_000, "4KB × 50K"),
    ];
    
    for (size, target, label) in test_cases {
        // Reset counters
        produced.store(0, Ordering::Release);
        consumed.store(0, Ordering::Release);
        
        // Create shared memory
        let mut producer_shm = WorkingSharedMemory::create(&format!("pc_{}", size), 32 * 1024 * 1024)?;
        
        let produced_clone = produced.clone();
        let consumed_clone = consumed.clone();
        
        // Producer thread
        let producer_thread = thread::spawn(move || {
            let data = vec![0xAB; size];
            let mut count = 0;
            let start = Instant::now();
            
            while count < target {
                if producer_shm.write(&data) {
                    count += 1;
                    produced_clone.store(count as u64, Ordering::Release);
                } else {
                    // Buffer full, yield
                    thread::yield_now();
                }
            }
            
            let elapsed = start.elapsed();
            (count, elapsed)
        });
        
        // Consumer thread
        let consumer_thread = thread::spawn(move || {
            let mut consumer_shm = WorkingSharedMemory::connect(&format!("pc_{}", size)).unwrap();
            let mut count = 0;
            let start = Instant::now();
            
            while count < target {
                if let Some(data) = consumer_shm.read() {
                    if data.len() == size && data[0] == 0xAB {
                        count += 1;
                        consumed_clone.store(count as u64, Ordering::Release);
                    }
                } else {
                    // Buffer empty, yield
                    thread::yield_now();
                }
            }
            
            let elapsed = start.elapsed();
            (count, elapsed)
        });
        
        // Wait for both
        let (prod_count, prod_time) = producer_thread.join().unwrap();
        let (cons_count, cons_time) = consumer_thread.join().unwrap();
        
        let prod_throughput = prod_count as f64 / prod_time.as_secs_f64();
        let cons_throughput = cons_count as f64 / cons_time.as_secs_f64();
        
        println!("=== {} ===", label);
        println!("Producer: {} msgs in {:.2}s = {:.2}M msg/sec", 
                 prod_count, prod_time.as_secs_f64(), prod_throughput / 1_000_000.0);
        println!("Consumer: {} msgs in {:.2}s = {:.2}M msg/sec",
                 cons_count, cons_time.as_secs_f64(), cons_throughput / 1_000_000.0);
        
        if prod_throughput > 1_000_000.0 && cons_throughput > 1_000_000.0 {
            println!("✅ Both >1M msg/sec!");
        } else if prod_throughput > 1_000_000.0 || cons_throughput > 1_000_000.0 {
            println!("⚠️ Only one side >1M msg/sec");
        } else {
            println!("❌ Below 1M msg/sec target");
        }
        println!();
    }
    
    Ok(())
}
