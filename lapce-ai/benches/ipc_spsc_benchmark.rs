/// Benchmark for optimized SPSC IPC transport
/// Target: ≥1M msg/s, p99 ≤10µs across Linux/Windows/macOS

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use std::alloc::{alloc_zeroed, dealloc, Layout};

// Import from lapce-ai-rust
use lapce_ai_rust::ipc::spsc_shm_ring::{SpscRing, RingHeader};
use lapce_ai_rust::ipc::shm_waiter_cross_os::ShmWaiter;

fn bench_spsc_single_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("spsc_single_thread");
    
    for msg_size in [64, 256, 1024, 4096].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(msg_size),
            msg_size,
            |b, &size| {
                unsafe {
                    let capacity = 64 * 1024;
                    let header_layout = Layout::new::<RingHeader>();
                    let data_layout = Layout::from_size_align(capacity, 64).unwrap();
                    
                    let header = alloc_zeroed(header_layout) as *mut RingHeader;
                    let data = alloc_zeroed(data_layout);
                    
                    let ring = SpscRing::from_raw(header, data, capacity);
                    let msg = vec![0u8; size];
                    
                    b.iter(|| {
                        assert!(ring.try_write(black_box(&msg)));
                        let read = ring.try_read().expect("Should read");
                        black_box(read);
                    });
                    
                    dealloc(header as *mut u8, header_layout);
                    dealloc(data, data_layout);
                }
            },
        );
    }
    
    group.finish();
}

fn bench_spsc_batch_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("spsc_batch_write");
    
    for batch_size in [8, 16, 32].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &size| {
                unsafe {
                    let capacity = 256 * 1024;
                    let header_layout = Layout::new::<RingHeader>();
                    let data_layout = Layout::from_size_align(capacity, 64).unwrap();
                    
                    let header = alloc_zeroed(header_layout) as *mut RingHeader;
                    let data = alloc_zeroed(data_layout);
                    
                    let ring = SpscRing::from_raw(header, data, capacity);
                    
                    let messages: Vec<Vec<u8>> = (0..size)
                        .map(|_| vec![0u8; 256])
                        .collect();
                    let msg_refs: Vec<&[u8]> = messages.iter().map(|m| m.as_slice()).collect();
                    
                    b.iter(|| {
                        let written = ring.try_write_batch(black_box(&msg_refs), size);
                        assert_eq!(written, size);
                        
                        let read = ring.try_read_batch(size);
                        assert_eq!(read.len(), size);
                        black_box(read);
                    });
                    
                    dealloc(header as *mut u8, header_layout);
                    dealloc(data, data_layout);
                }
            },
        );
    }
    
    group.finish();
}

fn bench_waiter_wake_latency(c: &mut Criterion) {
    c.bench_function("waiter_wake_latency", |b| {
        let waiter = ShmWaiter::new().unwrap();
        let seq = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let seq_ptr = Arc::as_ptr(&seq);
        
        b.iter(|| {
            let current = seq.load(std::sync::atomic::Ordering::Relaxed);
            seq.store(current + 1, std::sync::atomic::Ordering::Release);
            waiter.wake_one(black_box(seq_ptr));
        });
    });
}

fn bench_spsc_throughput(c: &mut Criterion) {
    c.bench_function("spsc_throughput_1k_msgs", |b| {
        unsafe {
            let capacity = 1024 * 1024; // 1MB ring
            let header_layout = Layout::new::<RingHeader>();
            let data_layout = Layout::from_size_align(capacity, 64).unwrap();
            
            let header = alloc_zeroed(header_layout) as *mut RingHeader;
            let data = alloc_zeroed(data_layout);
            
            let ring = Arc::new(SpscRing::from_raw(header, data, capacity));
            let ring_clone = ring.clone();
            
            let msg = vec![0u8; 1024];
            
            b.iter(|| {
                // Write 1000 messages
                for _ in 0..1000 {
                    while !ring.try_write(black_box(&msg)) {
                        // Spin if full
                        std::hint::spin_loop();
                    }
                }
                
                // Read 1000 messages
                for _ in 0..1000 {
                    while ring_clone.try_read().is_none() {
                        // Spin if empty
                        std::hint::spin_loop();
                    }
                }
            });
            
            dealloc(header as *mut u8, header_layout);
            dealloc(data, data_layout);
        }
    });
}

criterion_group!(
    benches,
    bench_spsc_single_thread,
    bench_spsc_batch_write,
    bench_waiter_wake_latency,
    bench_spsc_throughput
);
criterion_main!(benches);
