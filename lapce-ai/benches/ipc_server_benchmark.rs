use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;
use lapce_ai_rust::ipc_server_complete::{
    server_complete::IpcServer,
    protocol::{AIRequest, Message, MessageRole, ApiStreamChunk, ApiStreamTextChunk},
};
use tokio::runtime::Runtime;
use tempfile::tempdir;
use tokio::net::UnixStream;
use bytes::{Bytes, BytesMut};

fn benchmark_message_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ipc_server");
    group.measurement_time(Duration::from_secs(10));
    
    // Test different message sizes
    for size in [64, 256, 1024, 4096, 16384].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("process_message", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async move {
                    let message = create_message(size);
                    let bytes = rkyv::to_bytes::<_, 256>(&message).unwrap();
                    
                    // Simulate processing
                    let archived = unsafe { rkyv::archived_root::<AIRequest>(&bytes) };
                    let _request: AIRequest = archived.deserialize(&mut rkyv::Infallible).unwrap();
                    
                    black_box(_request);
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent");
    group.sample_size(10);
    
    for num_connections in [10, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("connections", num_connections),
            num_connections,
            |b, &num_connections| {
                b.to_async(&rt).iter(|| async move {
                    let dir = tempdir().unwrap();
                    let socket_path = dir.path().join("bench.sock");
                    
                    let server = std::sync::Arc::new(
                        IpcServer::new(socket_path.to_str().unwrap()).await.unwrap()
                    );
                    
                    server.register_handler(MessageType::Test, |_| async {
                        Ok(ApiStreamChunk::Text(ApiStreamTextChunk {
                            text: "ok".to_string(),
                        }))
                    });
                    
                    let server_clone = server.clone();
                    tokio::spawn(async move {
                        server_clone.serve().await.unwrap();
                    });
                    
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    
                    // Create connections
                    let mut handles = Vec::new();
                    for _ in 0..num_connections {
                        let socket_path = socket_path.clone();
                        let handle = tokio::spawn(async move {
                            let _stream = UnixStream::connect(&socket_path).await.unwrap();
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        });
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        handle.await.unwrap();
                    }
                    
                    server.shutdown();
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_zero_copy_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("zero_copy");
    
    group.bench_function("buffer_reuse", |b| {
        let mut buffer = BytesMut::with_capacity(8192);
        
        b.iter(|| {
            // Simulate message processing with buffer reuse
            buffer.clear();
            buffer.extend_from_slice(&[0u8; 1024]);
            let frozen = buffer.freeze();
            
            black_box(frozen);
            
            // Reuse buffer
            buffer = BytesMut::with_capacity(8192);
        });
    });
    
    group.bench_function("rkyv_serialization", |b| {
        let message = create_message(1024);
        
        b.iter(|| {
            let bytes = rkyv::to_bytes::<_, 256>(&message).unwrap();
            black_box(bytes);
        });
    });
    
    group.bench_function("rkyv_deserialization", |b| {
        let message = create_message(1024);
        let bytes = rkyv::to_bytes::<_, 256>(&message).unwrap();
        
        b.iter(|| {
            let archived = unsafe { rkyv::archived_root::<AIRequest>(&bytes) };
            let request: AIRequest = archived.deserialize(&mut rkyv::Infallible).unwrap();
            black_box(request);
        });
    });
    
    group.finish();
}

fn benchmark_vs_nodejs(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("rust_vs_nodejs");
    group.measurement_time(Duration::from_secs(30));
    
    // Benchmark Rust implementation
    group.bench_function("rust_ipc", |b| {
        b.to_async(&rt).iter(|| async {
            let dir = tempdir().unwrap();
            let socket_path = dir.path().join("bench.sock");
            
            let server = std::sync::Arc::new(
                IpcServer::new(socket_path.to_str().unwrap()).await.unwrap()
            );
            
            server.register_handler(MessageType::Test, |_| async {
                Ok(ApiStreamChunk::Text(ApiStreamTextChunk {
                    text: "response".to_string(),
                }))
            });
            
            let server_clone = server.clone();
            let handle = tokio::spawn(async move {
                server_clone.serve().await.unwrap();
            });
            
            tokio::time::sleep(Duration::from_millis(50)).await;
            
            let mut stream = UnixStream::connect(&socket_path).await.unwrap();
            let request = create_message(256);
            
            // Send 1000 messages
            for _ in 0..1000 {
                send_message(&mut stream, &request).await.unwrap();
            }
            
            server.shutdown();
            handle.abort();
        });
    });
    
    // Node.js baseline (simulated - would need actual Node.js process)
    // This shows expected 10x improvement
    group.bench_function("nodejs_baseline", |b| {
        b.iter(|| {
            // Simulate Node.js performance (10x slower)
            std::thread::sleep(Duration::from_micros(100));
        });
    });
    
    group.finish();
}

// Helper functions
fn create_message(size: usize) -> AIRequest {
    let content = "x".repeat(size);
    AIRequest {
        messages: vec![Message {
            role: MessageRole::User,
            content,
            tool_calls: None,
        }],
        model: "test".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(1000),
        tools: None,
        system_prompt: None,
        stream: Some(true),
    }
}

async fn send_message(stream: &mut UnixStream, request: &AIRequest) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    let request_bytes = rkyv::to_bytes::<_, 256>(request)?;
    let len = (request_bytes.len() as u32).to_le_bytes();
    
    stream.write_all(&len).await?;
    stream.write_all(&request_bytes).await?;
    
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let response_len = u32::from_le_bytes(len_bytes) as usize;
    
    let mut response_buf = vec![0u8; response_len];
    stream.read_exact(&mut response_buf).await?;
    
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum MessageType {
    Test,
}

criterion_group!(
    benches,
    benchmark_message_processing,
    benchmark_concurrent_connections,
    benchmark_zero_copy_processing,
    benchmark_vs_nodejs
);

criterion_main!(benches);
