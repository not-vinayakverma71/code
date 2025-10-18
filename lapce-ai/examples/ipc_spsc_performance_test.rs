/// Direct SPSC performance test to validate optimizations
/// Measures actual throughput and latency without Tokio overhead

use std::sync::Arc;
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::time::Instant;

use lapce_ai_rust::ipc::spsc_shm_ring::{SpscRing, RingHeader};
use lapce_ai_rust::ipc::shm_waiter_cross_os::ShmWaiter;

fn main() {
    println!("🚀 SPSC Ring Buffer Performance Test\n");
    
    // Test 1: Single-threaded throughput
    test_single_thread_throughput();
    
    // Test 2: Latency distribution
    test_latency_distribution();
    
    // Test 3: Batch performance
    test_batch_performance();
    
    // Test 4: Multi-threaded SPSC
    test_multi_threaded_spsc();
}

fn test_single_thread_throughput() {
    println!("═══ Test 1: Single-threaded Throughput ═══");
    
    unsafe {
        let capacity = 1024 * 1024; // 1MB
        let header_layout = Layout::new::<RingHeader>();
        let data_layout = Layout::from_size_align(capacity, 64).unwrap();
        
        let header = alloc_zeroed(header_layout) as *mut RingHeader;
        let data = alloc_zeroed(data_layout);
        
        let ring = SpscRing::from_raw(header, data, capacity);
        
        let msg = vec![0u8; 1024]; // 1KB messages
        let iterations = 100_000;
        
        let start = Instant::now();
        
        for _ in 0..iterations {
            while !ring.try_write(&msg) {
                // Should never happen with 1MB ring and 1KB messages
                panic!("Ring full unexpectedly");
            }
            
            let read = ring.try_read().expect("Should read");
            assert_eq!(read.len(), msg.len());
        }
        
        let elapsed = start.elapsed();
        let throughput = (iterations as f64) / elapsed.as_secs_f64();
        let latency_ns = (elapsed.as_nanos() as f64) / (iterations as f64);
        
        println!("  Messages: {}", iterations);
        println!("  Duration: {:.3}s", elapsed.as_secs_f64());
        println!("  Throughput: {:.2} Mmsg/s", throughput / 1_000_000.0);
        println!("  Avg Latency: {:.2}ns\n", latency_ns);
        
        dealloc(header as *mut u8, header_layout);
        dealloc(data, data_layout);
    }
}

fn test_latency_distribution() {
    println!("═══ Test 2: Latency Distribution ═══");
    
    unsafe {
        let capacity = 512 * 1024;
        let header_layout = Layout::new::<RingHeader>();
        let data_layout = Layout::from_size_align(capacity, 64).unwrap();
        
        let header = alloc_zeroed(header_layout) as *mut RingHeader;
        let data = alloc_zeroed(data_layout);
        
        let ring = SpscRing::from_raw(header, data, capacity);
        
        let msg = vec![0u8; 256];
        let iterations = 10_000;
        let mut latencies = Vec::with_capacity(iterations);
        
        for _ in 0..iterations {
            let start = Instant::now();
            ring.try_write(&msg);
            ring.try_read();
            latencies.push(start.elapsed().as_nanos() as u64);
        }
        
        latencies.sort_unstable();
        
        let p50 = latencies[iterations / 2];
        let p99 = latencies[(iterations * 99) / 100];
        let p999 = latencies[(iterations * 999) / 1000];
        
        println!("  p50:  {:.2}µs", p50 as f64 / 1000.0);
        println!("  p99:  {:.2}µs", p99 as f64 / 1000.0);
        println!("  p999: {:.2}µs\n", p999 as f64 / 1000.0);
        
        if p99 <= 10_000 {
            println!("  ✅ p99 latency ≤10µs target MET\n");
        } else {
            println!("  ❌ p99 latency >{:.2}µs (target: ≤10µs)\n", p99 as f64 / 1000.0);
        }
        
        dealloc(header as *mut u8, header_layout);
        dealloc(data, data_layout);
    }
}

fn test_batch_performance() {
    println!("═══ Test 3: Batch Performance ═══");
    
    unsafe {
        let capacity = 2 * 1024 * 1024; // 2MB
        let header_layout = Layout::new::<RingHeader>();
        let data_layout = Layout::from_size_align(capacity, 64).unwrap();
        
        let header = alloc_zeroed(header_layout) as *mut RingHeader;
        let data = alloc_zeroed(data_layout);
        
        let ring = SpscRing::from_raw(header, data, capacity);
        
        let batch_size = 16;
        let messages: Vec<Vec<u8>> = (0..batch_size).map(|_| vec![0u8; 512]).collect();
        let msg_refs: Vec<&[u8]> = messages.iter().map(|m| m.as_slice()).collect();
        
        let iterations = 10_000;
        
        let start = Instant::now();
        
        for _ in 0..iterations {
            let written = ring.try_write_batch(&msg_refs, batch_size);
            assert_eq!(written, batch_size);
            
            let read = ring.try_read_batch(batch_size);
            assert_eq!(read.len(), batch_size);
        }
        
        let elapsed = start.elapsed();
        let total_msgs = iterations * batch_size;
        let throughput = (total_msgs as f64) / elapsed.as_secs_f64();
        
        println!("  Batch size: {}", batch_size);
        println!("  Total messages: {}", total_msgs);
        println!("  Duration: {:.3}s", elapsed.as_secs_f64());
        println!("  Throughput: {:.2} Mmsg/s", throughput / 1_000_000.0);
        println!("  Batch amortization: {:.2}x faster\n", throughput / 419_000.0);
        
        dealloc(header as *mut u8, header_layout);
        dealloc(data, data_layout);
    }
}

fn test_multi_threaded_spsc() {
    println!("═══ Test 4: Multi-threaded SPSC (Producer/Consumer) ═══");
    
    unsafe {
        let capacity = 4 * 1024 * 1024; // 4MB
        let header_layout = Layout::new::<RingHeader>();
        let data_layout = Layout::from_size_align(capacity, 64).unwrap();
        
        let header = alloc_zeroed(header_layout) as *mut RingHeader;
        let data = alloc_zeroed(data_layout);
        
        let ring = Arc::new(SpscRing::from_raw(header, data, capacity));
        let ring_reader = ring.clone();
        
        let waiter = Arc::new(ShmWaiter::new().unwrap());
        let waiter_reader = waiter.clone();
        
        let iterations = 100_000;
        let msg = vec![0u8; 1024];
        
        let start = Instant::now();
        
        let producer = std::thread::spawn(move || {
            for _ in 0..iterations {
                while !ring.try_write(&msg) {
                    std::hint::spin_loop();
                }
                // Wake consumer
                waiter.wake_one(ring.write_seq_ptr());
            }
        });
        
        let consumer = std::thread::spawn(move || {
            let mut received = 0;
            while received < iterations {
                if let Some(_data) = ring_reader.try_read() {
                    received += 1;
                } else {
                    // Wait for data
                    let seq = ring_reader.write_seq();
                    waiter_reader.wait(
                        ring_reader.write_seq_ptr(),
                        seq,
                        std::time::Duration::from_millis(1)
                    );
                }
            }
        });
        
        producer.join().unwrap();
        consumer.join().unwrap();
        
        let elapsed = start.elapsed();
        let throughput = (iterations as f64) / elapsed.as_secs_f64();
        
        println!("  Messages: {}", iterations);
        println!("  Duration: {:.3}s", elapsed.as_secs_f64());
        println!("  Throughput: {:.2} Mmsg/s", throughput / 1_000_000.0);
        
        if throughput >= 1_000_000.0 {
            println!("  ✅ Throughput ≥1M msg/s target MET\n");
        } else {
            println!("  ⚠️  Throughput {:.2}K msg/s (target: ≥1M msg/s)\n", throughput / 1000.0);
        }
        
        dealloc(header as *mut u8, header_layout);
        dealloc(data, data_layout);
    }
}
