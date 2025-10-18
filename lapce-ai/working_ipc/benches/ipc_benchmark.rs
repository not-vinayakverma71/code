use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use working_ipc::*;
use tempfile::tempdir;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

fn benchmark_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("message_latency", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let socket_path = dir.path().join("bench.sock").to_str().unwrap().to_string();
                
                let server = Arc::new(IpcServer::new(socket_path.clone()));
                let server_clone = server.clone();
                
                let handle = tokio::spawn(async move {
                    let _ = server_clone.listen().await;
                });
                
                tokio::time::sleep(Duration::from_millis(10)).await;
                
                let mut client = IpcClient::new(socket_path);
                let start = Instant::now();
                client.connect().await.unwrap();
                let latency = start.elapsed();
                
                handle.abort();
                black_box(latency)
            })
        });
    });
}

fn benchmark_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("throughput");
    for message_count in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*message_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(message_count),
            message_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let dir = tempdir().unwrap();
                        let socket_path = dir.path().join("bench.sock").to_str().unwrap().to_string();
                        
                        let server = Arc::new(IpcServer::new(socket_path.clone()));
                        let server_clone = server.clone();
                        
                        let handle = tokio::spawn(async move {
                            let _ = server_clone.listen().await;
                        });
                        
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        
                        let mut client = IpcClient::new(socket_path.clone());
                        client.connect().await.unwrap();
                        
                        let start = Instant::now();
                        for i in 0..count {
                            let message = IpcMessage::TaskCommand {
                                origin: IpcOrigin::Client,
                                client_id: client.client_id().unwrap().clone(),
                                data: serde_json::json!({"index": i}),
                            };
                            client.send_message(message).await.unwrap();
                        }
                        let elapsed = start.elapsed();
                        
                        handle.abort();
                        black_box(elapsed)
                    })
                });
            },
        );
    }
    group.finish();
}

fn benchmark_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_connections");
    for client_count in [10, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(client_count),
            client_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let dir = tempdir().unwrap();
                        let socket_path = dir.path().join("bench.sock").to_str().unwrap().to_string();
                        
                        let server = Arc::new(IpcServer::new(socket_path.clone()));
                        let server_clone = server.clone();
                        
                        let handle = tokio::spawn(async move {
                            let _ = server_clone.listen().await;
                        });
                        
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        
                        let start = Instant::now();
                        let mut handles = vec![];
                        
                        for _ in 0..count {
                            let path = socket_path.clone();
                            let h = tokio::spawn(async move {
                                let mut client = IpcClient::new(path);
                                client.connect().await.unwrap();
                                client
                            });
                            handles.push(h);
                        }
                        
                        let mut clients = vec![];
                        for h in handles {
                            clients.push(h.await.unwrap());
                        }
                        let elapsed = start.elapsed();
                        
                        handle.abort();
                        black_box((elapsed, clients.len()))
                    })
                });
            },
        );
    }
    group.finish();
}

fn benchmark_memory(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("memory_usage_single_connection", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempdir().unwrap();
                let socket_path = dir.path().join("bench.sock").to_str().unwrap().to_string();
                
                // Measure memory before
                let before = get_memory_usage();
                
                let server = Arc::new(IpcServer::new(socket_path.clone()));
                let server_clone = server.clone();
                
                let handle = tokio::spawn(async move {
                    let _ = server_clone.listen().await;
                });
                
                tokio::time::sleep(Duration::from_millis(10)).await;
                
                let mut client = IpcClient::new(socket_path);
                client.connect().await.unwrap();
                
                // Send some messages
                for i in 0..100 {
                    let message = IpcMessage::TaskCommand {
                        origin: IpcOrigin::Client,
                        client_id: client.client_id().unwrap().clone(),
                        data: serde_json::json!({"index": i}),
                    };
                    client.send_message(message).await.unwrap();
                }
                
                // Measure memory after
                let after = get_memory_usage();
                let memory_used = after.saturating_sub(before);
                
                handle.abort();
                black_box(memory_used)
            })
        });
    });
}

fn get_memory_usage() -> usize {
    // Read from /proc/self/status
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<usize>() {
                        return kb * 1024; // Convert to bytes
                    }
                }
            }
        }
    }
    0
}

criterion_group!(
    benches,
    benchmark_latency,
    benchmark_throughput,
    benchmark_concurrent_connections,
    benchmark_memory
);
criterion_main!(benches);
