use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use lapce_ai_rust::shared_memory_transport::{SharedMemoryTransport, SharedMemoryListener, SharedMemoryStream};
use std::time::Duration;
use tokio::runtime::Runtime;
use bytes::Bytes;

fn benchmark_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("latency");
    group.measurement_time(Duration::from_secs(10));
    
    // Small message (100 bytes)
    group.throughput(Throughput::Bytes(100));
    group.bench_function("small_message_100B", |b| {
        b.to_async(&rt).iter(|| async {
            let transport = SharedMemoryTransport::new(1024 * 1024).unwrap();
            let msg = vec![0u8; 100];
            
            transport.send(&msg).await.unwrap();
            let received = transport.recv().await.unwrap();
            black_box(received);
        });
    });
    
    // Medium message (4KB)
    group.throughput(Throughput::Bytes(4096));
    group.bench_function("medium_message_4KB", |b| {
        b.to_async(&rt).iter(|| async {
            let transport = SharedMemoryTransport::new(1024 * 1024).unwrap();
            let msg = vec![0u8; 4096];
            
            transport.send(&msg).await.unwrap();
            let received = transport.recv().await.unwrap();
            black_box(received);
        });
    });
    
    // Large message (64KB)
    group.throughput(Throughput::Bytes(65536));
    group.bench_function("large_message_64KB", |b| {
        b.to_async(&rt).iter(|| async {
            let transport = SharedMemoryTransport::new(1024 * 1024).unwrap();
            let msg = vec![0u8; 65536];
            
            transport.send(&msg).await.unwrap();
            let received = transport.recv().await.unwrap();
            black_box(received);
        });
    });
    
    group.finish();
}

fn benchmark_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);
    
    // Measure messages per second
    group.bench_function("messages_per_second", |b| {
        b.to_async(&rt).iter(|| async {
            let transport = SharedMemoryTransport::new(1024 * 1024).unwrap();
            let msg = vec![0u8; 1024]; // 1KB message
            
            // Send 1000 messages
            for _ in 0..1000 {
                transport.send(&msg).await.unwrap();
                let _ = transport.recv().await.unwrap();
            }
        });
    });
    
    group.finish();
}

fn benchmark_memory(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory");
    
    // Memory allocation benchmark
    group.bench_function("transport_creation", |b| {
        b.iter(|| {
            let transport = SharedMemoryTransport::new(1024 * 1024).unwrap();
            black_box(transport);
        });
    });
    
    // Zero-copy verification
    group.bench_function("zero_copy_send", |b| {
        b.to_async(&rt).iter(|| async {
            let transport = SharedMemoryTransport::new(1024 * 1024).unwrap();
            let msg = Bytes::from_static(b"Hello, zero-copy world!");
            
            // This should not allocate
            transport.send(msg.as_ref()).await.unwrap();
            let _ = transport.recv().await.unwrap();
        });
    });
    
    group.finish();
}

fn benchmark_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);
    
    // Test with 100 concurrent connections
    group.bench_function("100_connections", |b| {
        b.to_async(&rt).iter(|| async {
            let mut handles = vec![];
            
            for i in 0..100 {
                let handle = tokio::spawn(async move {
                    let transport = SharedMemoryTransport::new(1024 * 1024).unwrap();
                    let msg = format!("Message from connection {}", i).into_bytes();
                    
                    transport.send(&msg).await.unwrap();
                    let _ = transport.recv().await.unwrap();
                });
                handles.push(handle);
            }
            
            for handle in handles {
                handle.await.unwrap();
            }
        });
    });
    
    // Test with 1000 concurrent connections
    group.bench_function("1000_connections", |b| {
        b.to_async(&rt).iter(|| async {
            let mut handles = vec![];
            
            for i in 0..1000 {
                let handle = tokio::spawn(async move {
                    let transport = SharedMemoryTransport::new(256 * 1024).unwrap(); // Smaller buffer for more connections
                    let msg = format!("Msg {}", i).into_bytes();
                    
                    transport.send(&msg).await.unwrap();
                    let _ = transport.recv().await.unwrap();
                });
                handles.push(handle);
            }
            
            for handle in handles {
                handle.await.unwrap();
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_latency,
    benchmark_throughput,
    benchmark_memory,
    benchmark_concurrent_connections
);
criterion_main!(benches);
