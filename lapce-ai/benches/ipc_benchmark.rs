use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use lapce_ai_rust::ipc_server::IpcServer;
use lapce_ai_rust::cross_platform_ipc::SharedMemoryTransport;
use lapce_ai_rust::ipc_config::IpcConfig;
use lapce_ai_rust::ipc_messages::MessageType;
use std::sync::Arc;
use std::time::Duration;
use bytes::Bytes;
use tokio::runtime::Runtime;

fn bench_message_processing(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    
    // Setup server
    let server = runtime.block_on(async {
        let mut config = ServerConfig::default();
        config.socket_path = "/tmp/bench.sock".to_string();
        Arc::new(IpcServer::new(config).await.unwrap())
    });
    
    // Register test handler
    server.register_handler(MessageType::Custom(1), |data| async move {
        Ok(data)
    });
    
    let mut group = c.benchmark_group("message_processing");
    
    // Test different message sizes
    for size in [64, 256, 1024, 4096, 16384].iter() {
        let test_data = vec![0u8; *size];
        let message = Message::new(
            MessageType::Custom(1),
            1,
            Bytes::from(test_data.clone())
        );
        let msg_bytes = message.to_bytes().unwrap();
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                b.iter(|| {
                    runtime.block_on(async {
                        let _result = Message::from_bytes(&msg_bytes);
                    })
                });
            }
        );
    }
    
    group.finish();
}

fn bench_handler_dispatch(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    
    let server = runtime.block_on(async {
        let mut config = ServerConfig::default();
        config.socket_path = "/tmp/bench2.sock".to_string();
        Arc::new(IpcServer::new(config).await.unwrap())
    });
    
    // Register multiple handlers
    for i in 0..100 {
        let msg_type = MessageType::Custom(i);
        server.register_handler(msg_type, |data| async move {
            Ok(data)
        });
    }
    
    c.bench_function("handler_dispatch", |b| {
        b.iter(|| {
            let _ = server.handler_count();
        });
    });
}

fn bench_buffer_pool(c: &mut Criterion) {
    use lapce_ai_rust::ipc::SharedBufferPool;
    
    let pool = SharedBufferPool::new();
    
    let mut group = c.benchmark_group("buffer_pool");
    
    for size in [1024, 4096, 65536].iter() {
        group.bench_with_input(
            BenchmarkId::new("acquire_release", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let buffer = pool.acquire(size);
                    pool.release(buffer);
                });
            }
        );
    }
    
    group.finish();
}

fn bench_metrics(c: &mut Criterion) {
    use lapce_ai_rust::ipc::Metrics;
    
    let metrics = Metrics::new();
    
    c.bench_function("metrics_record", |b| {
        b.iter(|| {
            metrics.record(
                MessageType::Ping,
                Duration::from_micros(black_box(100))
            );
        });
    });
    
    c.bench_function("metrics_snapshot", |b| {
        b.iter(|| {
            let _ = metrics.snapshot();
        });
    });
}

fn bench_zero_allocation(c: &mut Criterion) {
    use bytes::BytesMut;
    
    let mut buffer = BytesMut::with_capacity(8192);
    
    c.bench_function("zero_copy_processing", |b| {
        b.iter(|| {
            // Simulate zero-copy message processing
            buffer.clear();
            buffer.resize(1024, 0);
            
            // Process without allocation
            let _data = &buffer[..1024];
            
            buffer.clear();
        });
    });
}

criterion_group!(
    benches,
    bench_message_processing,
    bench_handler_dispatch,
    bench_buffer_pool,
    bench_metrics,
    bench_zero_allocation
);

criterion_main!(benches);
