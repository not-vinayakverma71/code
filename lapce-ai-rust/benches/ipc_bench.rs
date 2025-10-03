use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use lapce_ai_rust::ipc_server_real::{IPCServer, IPCClient, IPCMessage};
use std::time::Duration;
use tokio::runtime::Runtime;

fn ipc_latency_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // Start server
    let socket_path = "/tmp/lapce_ipc_bench.sock";
    let _ = std::fs::remove_file(socket_path); // Clean up
    
    let server = rt.block_on(async {
        IPCServer::new(socket_path).await.unwrap()
    });
    
    // Start server in background
    rt.spawn(async move {
        server.run().await.unwrap();
    });
    
    std::thread::sleep(Duration::from_millis(100)); // Let server start
    
    // Connect client
    let client = rt.block_on(async {
        IPCClient::connect(socket_path).await.unwrap()
    });
    
    let mut group = c.benchmark_group("ipc_latency");
    
    // Small message (100 bytes)
    group.throughput(Throughput::Bytes(100));
    group.bench_function("small_message", |b| {
        b.to_async(&rt).iter(|| async {
            let msg = IPCMessage::Request {
                id: 1,
                method: "test".to_string(),
                params: vec![0u8; 100],
            };
            let _ = client.send_request(black_box(msg)).await;
        });
    });
    
    // Medium message (10KB)
    group.throughput(Throughput::Bytes(10_000));
    group.bench_function("medium_message", |b| {
        b.to_async(&rt).iter(|| async {
            let msg = IPCMessage::Request {
                id: 1,
                method: "test".to_string(),
                params: vec![0u8; 10_000],
            };
            let _ = client.send_request(black_box(msg)).await;
        });
    });
    
    // Large message (1MB)
    group.throughput(Throughput::Bytes(1_000_000));
    group.bench_function("large_message", |b| {
        b.to_async(&rt).iter(|| async {
            let msg = IPCMessage::Request {
                id: 1,
                method: "test".to_string(),
                params: vec![0u8; 1_000_000],
            };
            let _ = client.send_request(black_box(msg)).await;
        });
    });
    
    group.finish();
}

fn ipc_throughput_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let socket_path = "/tmp/lapce_ipc_throughput.sock";
    let _ = std::fs::remove_file(socket_path);
    
    let server = rt.block_on(async {
        IPCServer::new(socket_path).await.unwrap()
    });
    
    rt.spawn(async move {
        server.run().await.unwrap();
    });
    
    std::thread::sleep(Duration::from_millis(100));
    
    let client = rt.block_on(async {
        IPCClient::connect(socket_path).await.unwrap()
    });
    
    let mut group = c.benchmark_group("ipc_throughput");
    group.measurement_time(Duration::from_secs(10));
    
    // Messages per second
    group.bench_function("messages_per_sec", |b| {
        b.to_async(&rt).iter(|| async {
            for _ in 0..100 {
                let msg = IPCMessage::Request {
                    id: 1,
                    method: "test".to_string(),
                    params: vec![0u8; 100],
                };
                let _ = client.send_request(black_box(msg)).await;
            }
        });
    });
    
    group.finish();
}

criterion_group!(benches, ipc_latency_benchmark, ipc_throughput_benchmark);
criterion_main!(benches);
