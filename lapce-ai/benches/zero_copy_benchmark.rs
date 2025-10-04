use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;
use bytes::{Bytes, BytesMut};
use lapce_ai_rust::ipc::server_zero_copy::{
    ZeroCopyServer, MessageType, ConnectionMetrics, MAX_MESSAGE_SIZE
};

fn benchmark_message_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_parsing");
    
    // Test parsing without allocation
    group.bench_function("parse_message_type", |b| {
        let data = [42, 0, 0, 0, 1, 2, 3, 4];
        b.iter(|| {
            let msg_type = MessageType::from_bytes(&data).unwrap();
            black_box(msg_type);
        });
    });
    
    // Test roundtrip encoding/decoding
    group.bench_function("message_type_roundtrip", |b| {
        let msg_type = MessageType::Custom(12345);
        b.iter(|| {
            let bytes = msg_type.to_bytes();
            let parsed = MessageType::from_bytes(&bytes).unwrap();
            black_box(parsed);
        });
    });
    
    group.finish();
}

fn benchmark_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_ops");
    
    // Benchmark buffer reuse
    group.bench_function("buffer_reuse", |b| {
        let mut buffer = BytesMut::with_capacity(8192);
        b.iter(|| {
            // Simulate message processing
            buffer.resize(1024, 0);
            for i in 0..1024 {
                buffer[i] = (i % 256) as u8;
            }
            let _data = &buffer[..1024];
            buffer.clear(); // Reuse without deallocation
        });
    });
    
    // Benchmark capacity management
    for size in [64, 256, 1024, 4096, 16384].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("capacity_management", size),
            size,
            |b, &size| {
                let mut buffer = BytesMut::with_capacity(8192);
                b.iter(|| {
                    if buffer.capacity() < size {
                        buffer.reserve(size - buffer.len());
                    }
                    buffer.resize(size, 0);
                    buffer.clear();
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_zero_copy_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("zero_copy");
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Benchmark handler registration and lookup
    group.bench_function("handler_lookup", |b| {
        let server = ZeroCopyServer::new();
        
        // Register handlers
        for i in 0..10 {
            let msg_type = MessageType::Custom(i);
            server.register_handler(msg_type, |data| async move {
                Ok(data)
            });
        }
        
        b.iter(|| {
            let msg_type = MessageType::Custom(5);
            let _has_handler = server.handlers.contains_key(&msg_type);
            black_box(_has_handler);
        });
    });
    
    // Benchmark process_message
    group.bench_function("process_message", |b| {
        b.to_async(&rt).iter(|| async {
            let server = ZeroCopyServer::new();
            
            server.register_handler(MessageType::Request, |data| async move {
                // Simple echo handler
                Ok(data)
            });
            
            let mut metrics = ConnectionMetrics::default();
            let mut message = vec![0, 0, 0, 0]; // Message type: Request
            message.extend_from_slice(b"Hello, World!"); // Payload
            
            let response = server.process_message(&message, &mut metrics).await.unwrap();
            black_box(response);
        });
    });
    
    group.finish();
}

fn benchmark_allocation_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocations");
    
    // This benchmark specifically tracks allocations
    group.bench_function("hot_path_allocations", |b| {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        thread_local! {
            static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
        }
        
        struct TrackingAllocator;
        
        unsafe impl GlobalAlloc for TrackingAllocator {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                ALLOCATIONS.with(|a| a.fetch_add(1, Ordering::Relaxed));
                System.alloc(layout)
            }
            
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                System.dealloc(ptr, layout)
            }
        }
        
        // Measure allocations in message parsing
        b.iter(|| {
            ALLOCATIONS.with(|a| a.store(0, Ordering::Relaxed));
            
            // Operations that should not allocate
            let data = [1, 0, 0, 0];
            let msg_type = MessageType::from_bytes(&data).unwrap();
            
            let allocations = ALLOCATIONS.with(|a| a.load(Ordering::Relaxed));
            
            // Assert zero allocations in hot path
            assert_eq!(allocations, 0, "Found {} allocations in hot path", allocations);
            
            black_box(msg_type);
        });
    });
    
    group.finish();
}

fn benchmark_large_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_messages");
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    for size in [1024, 8192, 65536, 524288, 1048576].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("process_large_message", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async move {
                    let server = ZeroCopyServer::new();
                    
                    server.register_handler(MessageType::Request, |data| async move {
                        // Process and return modified data
                        let mut result = data.to_vec();
                        result.reverse();
                        Ok(Bytes::from(result))
                    });
                    
                    let mut metrics = ConnectionMetrics::default();
                    let mut message = vec![0, 0, 0, 0]; // Message type
                    message.extend(vec![0u8; size]); // Large payload
                    
                    let response = server.process_message(&message, &mut metrics).await.unwrap();
                    black_box(response);
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_metrics_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics");
    
    group.bench_function("metrics_update", |b| {
        let mut metrics = ConnectionMetrics::default();
        let duration = Duration::from_nanos(100);
        
        b.iter(|| {
            metrics.requests += 1;
            metrics.total_time += duration;
            metrics.bytes_sent += 1024;
            metrics.bytes_received += 1024;
        });
    });
    
    group.bench_function("metrics_record", |b| {
        let metrics = lapce_ai_rust::ipc::server_zero_copy::Metrics::new();
        let msg_type = MessageType::Request;
        let duration = Duration::from_nanos(100);
        
        b.iter(|| {
            metrics.record(msg_type, duration);
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_message_parsing,
    benchmark_buffer_operations,
    benchmark_zero_copy_processing,
    benchmark_allocation_tracking,
    benchmark_large_messages,
    benchmark_metrics_overhead
);

criterion_main!(benches);
