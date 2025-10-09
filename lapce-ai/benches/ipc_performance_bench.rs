/// IPC Performance Benchmarks
/// Target: ≥1M msg/s throughput and ≤10µs p99 latency

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use lapce_ai_rust::ipc::{
    binary_codec::{BinaryCodec, Message, MessageType, MessagePayload, CompletionRequest},
    ipc::shared_memory_complete::SharedMemoryBuffer,
};
use std::time::Duration;
use bytes::Bytes;

fn create_test_message(size: usize) -> Message {
    Message {
        id: 12345,
        msg_type: MessageType::CompletionRequest,
        payload: MessagePayload::CompletionRequest(CompletionRequest {
            prompt: "x".repeat(size),
            model: "test-model".to_string(),
            max_tokens: 100,
            temperature: 0.7,
            stream: false,
        }),
        timestamp: 1234567890,
    }
}

fn benchmark_codec_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_throughput");
    group.measurement_time(Duration::from_secs(10));
    
    for size in &[64, 256, 1024, 4096, 16384] {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut codec = BinaryCodec::new();
            let msg = create_test_message(size);
            
            b.iter(|| {
                let encoded = codec.encode(&msg).unwrap();
                let _decoded = codec.decode(&encoded).unwrap();
                black_box(encoded);
            });
        });
    }
    group.finish();
}

fn benchmark_codec_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_latency_us");
    
    for size in &[64, 256, 1024] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut codec = BinaryCodec::new();
            let msg = create_test_message(size);
            
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let encoded = codec.encode(&msg).unwrap();
                    let _decoded = codec.decode(&encoded).unwrap();
                    black_box(encoded);
                }
                start.elapsed()
            });
        });
    }
    group.finish();
}

fn benchmark_shared_memory_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("shm_throughput");
    group.measurement_time(Duration::from_secs(10));
    
    for size in &[64, 256, 1024, 4096] {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut buffer = SharedMemoryBuffer::create("/bench_shm", 4 * 1024 * 1024).unwrap();
            let data = vec![0u8; size];
            
            b.iter(|| {
                buffer.write(&data).unwrap();
                let read_data = buffer.read().unwrap();
                black_box(read_data);
            });
        });
    }
    group.finish();
}

fn benchmark_shared_memory_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("shm_latency_us");
    
    for size in &[64, 256, 1024] {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut buffer = SharedMemoryBuffer::create(&format!("/bench_shm_{}", size), 1024 * 1024).unwrap();
            let data = vec![0u8; size];
            
            b.iter_custom(|iters| {
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    buffer.write(&data).unwrap();
                    let read_data = buffer.read().unwrap();
                    black_box(read_data);
                }
                start.elapsed()
            });
        });
    }
    group.finish();
}

fn benchmark_end_to_end_messages_per_second(c: &mut Criterion) {
    c.bench_function("messages_per_second", |b| {
        let mut codec = BinaryCodec::new();
        let mut buffer = SharedMemoryBuffer::create("/bench_e2e", 4 * 1024 * 1024).unwrap();
        let msg = create_test_message(256);
        
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for _ in 0..iters {
                // Encode
                let encoded = codec.encode(&msg).unwrap();
                // Write to shared memory
                buffer.write(&encoded).unwrap();
                // Read from shared memory
                let read_data = buffer.read().unwrap();
                // Decode
                let _decoded = codec.decode(&read_data).unwrap();
            }
            start.elapsed()
        });
    });
}

fn benchmark_p99_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("p99_latency");
    group.sample_size(1000);
    group.measurement_time(Duration::from_secs(30));
    
    group.bench_function("end_to_end_p99", |b| {
        let mut codec = BinaryCodec::new();
        let mut buffer = SharedMemoryBuffer::create("/bench_p99", 4 * 1024 * 1024).unwrap();
        let msg = create_test_message(256);
        
        b.iter(|| {
            let start = std::time::Instant::now();
            
            // Full IPC round trip
            let encoded = codec.encode(&msg).unwrap();
            buffer.write(&encoded).unwrap();
            let read_data = buffer.read().unwrap();
            let _decoded = codec.decode(&read_data).unwrap();
            
            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });
    group.finish();
}

fn benchmark_concurrent_throughput(c: &mut Criterion) {
    use std::sync::Arc;
    use parking_lot::RwLock;
    use std::thread;
    
    let mut group = c.benchmark_group("concurrent_throughput");
    group.measurement_time(Duration::from_secs(10));
    
    for num_threads in &[1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            num_threads,
            |b, &num_threads| {
                let buffer = Arc::new(RwLock::new(
                    SharedMemoryBuffer::create("/bench_concurrent", 8 * 1024 * 1024).unwrap()
                ));
                let data = vec![0u8; 256];
                
                b.iter_custom(|iters| {
                    let iters_per_thread = iters / num_threads as u64;
                    let mut handles = vec![];
                    
                    let start = std::time::Instant::now();
                    
                    for _ in 0..num_threads {
                        let buffer_clone = buffer.clone();
                        let data_clone = data.clone();
                        
                        let handle = thread::spawn(move || {
                            for _ in 0..iters_per_thread {
                                let mut buf = buffer_clone.write();
                                buf.write(&data_clone).unwrap();
                                let _read = buf.read().unwrap();
                            }
                        });
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        handle.join().unwrap();
                    }
                    
                    start.elapsed()
                });
            }
        );
    }
    group.finish();
}

// Benchmark to verify we meet the 1M msg/s target
fn verify_performance_targets(c: &mut Criterion) {
    c.bench_function("verify_1m_msgs_per_sec", |b| {
        let mut codec = BinaryCodec::new();
        let msg = create_test_message(256);
        
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let encoded = codec.encode(&msg).unwrap();
                let _decoded = codec.decode(&encoded).unwrap();
            }
            let elapsed = start.elapsed();
            
            // Calculate messages per second
            let msgs_per_sec = iters as f64 / elapsed.as_secs_f64();
            
            // Log if we don't meet target
            if msgs_per_sec < 1_000_000.0 {
                eprintln!("WARNING: Only achieving {:.0} msgs/sec, target is 1M", msgs_per_sec);
            }
            
            elapsed
        });
    });
}

criterion_group!(
    benches,
    benchmark_codec_throughput,
    benchmark_codec_latency,
    benchmark_shared_memory_throughput,
    benchmark_shared_memory_latency,
    benchmark_end_to_end_messages_per_second,
    benchmark_p99_latency,
    benchmark_concurrent_throughput,
    verify_performance_targets
);

criterion_main!(benches);
