/// Performance benchmarking suite
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use tokio::net::UnixStream;
use std::time::Duration;
use lapce_ai_rust::{
    simple_ipc_server::SimpleIpcServer,
    production_ipc_server::ProductionIpcServer,
    common_types::{UnifiedMessage, MessageMetadata},
    buffer_pool::BufferPool,
    compression::CompressionHandler,
    rate_limiter::RateLimiter,
};
use serde_json::json;

fn bench_message_serialization(c: &mut Criterion) {
    let msg = UnifiedMessage::Request {
        id: "bench_123".to_string(),
        method: "echo".to_string(),
        params: json!({"data": "x".repeat(1024)}),
        metadata: MessageMetadata::default(),
    };
    
    c.bench_function("serialize_message", |b| {
        b.iter(|| {
            let _ = serde_json::to_vec(&msg).unwrap();
        });
    });
    
    let serialized = serde_json::to_vec(&msg).unwrap();
    c.bench_function("deserialize_message", |b| {
        b.iter(|| {
            let _: UnifiedMessage = serde_json::from_slice(&serialized).unwrap();
        });
    });
}

fn bench_buffer_pool(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = BufferPool::new();
    
    c.bench_function("buffer_acquire_release", |b| {
        b.to_async(&rt).iter(|| async {
            let buffer = pool.acquire().await;
            pool.release(buffer).await;
        });
    });
}

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
    let handler = CompressionHandler::new();
    
    for size in [100, 1024, 10240, 102400].iter() {
        let data = vec![0u8; *size];
        
        group.bench_with_input(BenchmarkId::new("compress", size), size, |b, _| {
            b.iter(|| {
                let _ = handler.compress(&data).unwrap();
            });
        });
        
        let compressed = handler.compress(&data).unwrap();
        group.bench_with_input(BenchmarkId::new("decompress", size), size, |b, _| {
            b.iter(|| {
                let _ = handler.decompress(&compressed).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_rate_limiter(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let limiter = RateLimiter::new_with_window(1000, Duration::from_secs(1), 100);
    
    c.bench_function("rate_limit_check", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = limiter.check("client_1", 1).await;
        });
    });
}

fn bench_echo_server(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    // Start server
    let socket_path = "/tmp/bench_ipc.sock";
    let _ = std::fs::remove_file(socket_path);
    
    let server = SimpleIpcServer::new(socket_path.to_string());
    rt.spawn(async move {
        let _ = server.start().await;
    });
    
    // Wait for server to start
    std::thread::sleep(Duration::from_millis(100));
    
    c.bench_function("echo_round_trip", |b| {
        b.to_async(&rt).iter(|| async {
            let mut stream = UnixStream::connect(socket_path).await.unwrap();
            let msg = b"Hello, benchmark!";
            
            // Send
            stream.write_all(&(msg.len() as u32).to_le_bytes()).await.unwrap();
            stream.write_all(msg).await.unwrap();
            
            // Receive
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).await.unwrap();
            let len = u32::from_le_bytes(len_buf) as usize;
            
            let mut response = vec![0u8; len];
            stream.read_exact(&mut response).await.unwrap();
        });
    });
    
    let _ = std::fs::remove_file(socket_path);
}

fn bench_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let socket_path = "/tmp/bench_concurrent.sock";
    let _ = std::fs::remove_file(socket_path);
    
    let server = SimpleIpcServer::new(socket_path.to_string());
    rt.spawn(async move {
        let _ = server.start().await;
    });
    
    std::thread::sleep(Duration::from_millis(100));
    
    let mut group = c.benchmark_group("concurrent_connections");
    
    for num_clients in [1, 10, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("connections", num_clients),
            num_clients,
            |b, &num| {
                b.to_async(&rt).iter(|| async move {
                    let mut handles = Vec::new();
                    
                    for _ in 0..num {
                        let handle = tokio::spawn(async move {
                            let _ = UnixStream::connect(socket_path).await;
                        });
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        let _ = handle.await;
                    }
                });
            },
        );
    }
    
    group.finish();
    let _ = std::fs::remove_file(socket_path);
}

fn bench_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let socket_path = "/tmp/bench_throughput.sock";
    let _ = std::fs::remove_file(socket_path);
    
    let server = ProductionIpcServer::new(socket_path.to_string());
    rt.spawn(async move {
        let _ = server.start().await;
    });
    
    std::thread::sleep(Duration::from_millis(100));
    
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("messages_per_second", |b| {
        b.to_async(&rt).iter(|| async {
            let mut stream = UnixStream::connect(socket_path).await.unwrap();
            
            for _ in 0..100 {
                let msg = b"test message";
                stream.write_all(&(msg.len() as u32).to_le_bytes()).await.unwrap();
                stream.write_all(msg).await.unwrap();
                
                let mut len_buf = [0u8; 4];
                stream.read_exact(&mut len_buf).await.unwrap();
                let len = u32::from_le_bytes(len_buf) as usize;
                
                let mut response = vec![0u8; len];
                stream.read_exact(&mut response).await.unwrap();
            }
        });
    });
    
    group.finish();
    let _ = std::fs::remove_file(socket_path);
}

criterion_group!(
    benches,
    bench_message_serialization,
    bench_buffer_pool,
    bench_compression,
    bench_rate_limiter,
    bench_echo_server,
    bench_concurrent_connections,
    bench_throughput
);
criterion_main!(benches);
