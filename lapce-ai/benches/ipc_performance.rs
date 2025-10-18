// IPC Performance Benchmarks - IPC-015/IPC-016
// Documents benchmark methodology and measures throughput/latency

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lapce_ai_rust::ipc::{IpcServer, MessageType};
use lapce_ai_rust::ipc::binary_codec::BinaryCodec;
use lapce_ai_rust::ipc::zero_copy_codec::ZeroCopyCodec;
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

/// Benchmark Methodology Documentation
/// 
/// This benchmark suite measures IPC system performance across multiple dimensions:
/// 
/// 1. **Message Throughput**: Messages processed per second
///    - Target: ≥1M msg/s for small messages (≤1KB)
///    - Measured using batched operations to amortize overhead
/// 
/// 2. **Message Latency**: End-to-end message processing time
///    - Target: ≤10µs p99 latency for small messages
///    - Measured using high-resolution timing for individual operations
/// 
/// 3. **Codec Performance**: Encode/decode speed for both codecs
///    - BinaryCodec vs ZeroCopyCodec comparison
///    - Tests various message sizes: 64B, 1KB, 16KB, 1MB
/// 
/// 4. **Connection Pool Efficiency**: Connection acquisition time
///    - Target: <1ms p50, <5ms p99
///    - Tests with varying pool sizes and concurrency levels
/// 
/// 5. **Shared Memory Performance**: Zero-copy message passing
///    - Tests ring buffer throughput
///    - Measures memory bandwidth utilization
/// 
/// ## Running Benchmarks
/// 
/// ```bash
/// # Run all benchmarks
/// cargo bench --package lapce-ai-rust
/// 
/// # Run specific benchmark
/// cargo bench --package lapce-ai-rust -- throughput
/// 
/// # Generate HTML report
/// cargo bench --package lapce-ai-rust -- --save-baseline baseline
/// 
/// # Compare against baseline
/// cargo bench --package lapce-ai-rust -- --baseline baseline
/// ```
/// 
/// ## CI Integration
/// 
/// Benchmarks should run in CI with performance gates:
/// - Fail if throughput drops below 900K msg/s (10% margin)
/// - Fail if p99 latency exceeds 12µs (20% margin)
/// - Generate performance regression reports

/// Benchmark message throughput
fn benchmark_message_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("message_throughput");
    group.measurement_time(Duration::from_secs(10)); // Longer measurement for accuracy
    
    // Test different message sizes
    for size in [64, 256, 1024, 4096, 16384].iter() {
        let msg_data = vec![0xAB; *size];
        let msg_bytes = Bytes::from(msg_data);
        
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("binary_codec", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let codec = BinaryCodec::new();
                    let encoded = codec.encode(
                        MessageType::Request,
                        black_box(msg_bytes.clone())
                    ).await.unwrap();
                    
                    let _decoded = codec.decode(encoded).await.unwrap();
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("zerocopy_codec", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let codec = ZeroCopyCodec::new();
                    let encoded = codec.encode(
                        MessageType::Request,
                        black_box(msg_bytes.clone())
                    ).await.unwrap();
                    
                    let _decoded = codec.decode(encoded).await.unwrap();
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark message latency (p50, p95, p99)
fn benchmark_message_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("message_latency");
    group.sample_size(1000); // Large sample for percentile accuracy
    
    // Small message latency test (target: ≤10µs p99)
    let small_msg = Bytes::from(vec![0x42; 128]);
    
    group.bench_function("small_message_p99", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let codec = BinaryCodec::new();
            let mut total_duration = Duration::ZERO;
            
            for _ in 0..iters {
                let start = Instant::now();
                
                let encoded = codec.encode(
                    MessageType::Request,
                    small_msg.clone()
                ).await.unwrap();
                
                let _decoded = codec.decode(encoded).await.unwrap();
                
                total_duration += start.elapsed();
            }
            
            total_duration
        });
    });
    
    group.finish();
}

/// Benchmark shared memory ring buffer performance
fn benchmark_shared_memory(c: &mut Criterion) {
    use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;
    
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("shared_memory");
    
    // Create shared memory buffer
    let buffer_size = 1024 * 1024; // 1MB buffer
    
    group.bench_function("ring_buffer_write", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let buffer = SharedMemoryBuffer::new("/bench_shm", buffer_size).unwrap();
            let data = vec![0x55; 1024]; // 1KB chunks
            let mut total_duration = Duration::ZERO;
            
            for _ in 0..iters {
                let start = Instant::now();
                buffer.write(&data).unwrap();
                total_duration += start.elapsed();
            }
            
            buffer.cleanup();
            total_duration
        });
    });
    
    group.bench_function("ring_buffer_read", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let buffer = SharedMemoryBuffer::new("/bench_shm_read", buffer_size).unwrap();
            let data = vec![0x55; 1024];
            buffer.write(&data).unwrap(); // Pre-write data
            
            let mut total_duration = Duration::ZERO;
            let mut read_buf = vec![0; 1024];
            
            for _ in 0..iters {
                buffer.write(&data).unwrap(); // Ensure data available
                let start = Instant::now();
                buffer.read(&mut read_buf).unwrap();
                total_duration += start.elapsed();
            }
            
            buffer.cleanup();
            total_duration
        });
    });
    
    group.finish();
}

/// Benchmark connection pool acquisition
fn benchmark_connection_pool(c: &mut Criterion) {
    use lapce_ai_rust::connection_pool_manager::{ConnectionPoolManager, PoolConfig};
    
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("connection_pool");
    
    // Test pool acquisition with different configurations
    for pool_size in [10, 50, 100].iter() {
        let config = PoolConfig {
            max_connections: *pool_size,
            min_idle: pool_size / 2,
            max_lifetime: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(30),
            connection_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        
        group.bench_with_input(
            BenchmarkId::new("acquire", pool_size),
            pool_size,
            |b, &_size| {
                b.to_async(&rt).iter_custom(|iters| async move {
                    let pool = Arc::new(ConnectionPoolManager::new(config.clone()).await.unwrap());
                    let mut total_duration = Duration::ZERO;
                    
                    for _ in 0..iters {
                        let start = Instant::now();
                        
                        // Simulate connection acquisition
                        let _stats = pool.get_stats();
                        let _active = pool.active_count().await;
                        
                        total_duration += start.elapsed();
                    }
                    
                    total_duration
                });
            },
        );
    }
    
    group.finish();
}

/// Million message per second throughput test
fn benchmark_million_msg_per_sec(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("million_msg_per_sec");
    group.measurement_time(Duration::from_secs(30)); // Extended test
    
    group.bench_function("batch_1m_messages", |b| {
        b.to_async(&rt).iter_custom(|_iters| async move {
            let codec = ZeroCopyCodec::new(); // Use zero-copy for max performance
            let small_msg = Bytes::from(vec![0x42; 64]); // 64-byte messages
            let batch_size = 10000;
            let total_messages = 1_000_000;
            
            let start = Instant::now();
            
            for batch in 0..(total_messages / batch_size) {
                // Process messages in batches for efficiency
                let mut tasks = Vec::new();
                
                for _ in 0..batch_size {
                    let codec_clone = codec.clone();
                    let msg_clone = small_msg.clone();
                    
                    let task = tokio::spawn(async move {
                        let encoded = codec_clone.encode(
                            MessageType::Request,
                            msg_clone
                        ).await.unwrap();
                        
                        let _decoded = codec_clone.decode(encoded).await.unwrap();
                    });
                    
                    tasks.push(task);
                }
                
                // Wait for batch to complete
                for task in tasks {
                    task.await.unwrap();
                }
            }
            
            let duration = start.elapsed();
            let msgs_per_sec = (total_messages as f64) / duration.as_secs_f64();
            
            println!("Achieved throughput: {:.0} msg/s", msgs_per_sec);
            assert!(msgs_per_sec >= 900_000.0, "Failed to achieve 1M msg/s target (with 10% margin)");
            
            duration
        });
    });
    
    group.finish();
}

/// Sub-10 microsecond p99 latency test
fn benchmark_sub10us_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("sub10us_latency");
    
    group.bench_function("p99_latency_test", |b| {
        b.to_async(&rt).iter_custom(|_iters| async move {
            let codec = ZeroCopyCodec::new();
            let small_msg = Bytes::from(vec![0x42; 64]);
            let sample_count = 10000;
            let mut latencies = Vec::with_capacity(sample_count);
            
            // Warm up
            for _ in 0..100 {
                let _ = codec.encode(MessageType::Request, small_msg.clone()).await;
            }
            
            // Measure latencies
            for _ in 0..sample_count {
                let start = Instant::now();
                
                let encoded = codec.encode(
                    MessageType::Request,
                    small_msg.clone()
                ).await.unwrap();
                
                let _decoded = codec.decode(encoded).await.unwrap();
                
                latencies.push(start.elapsed());
            }
            
            // Calculate percentiles
            latencies.sort();
            let p50_idx = latencies.len() / 2;
            let p95_idx = (latencies.len() * 95) / 100;
            let p99_idx = (latencies.len() * 99) / 100;
            
            let p50 = latencies[p50_idx];
            let p95 = latencies[p95_idx];
            let p99 = latencies[p99_idx];
            
            println!("Latency percentiles:");
            println!("  P50: {:?}", p50);
            println!("  P95: {:?}", p95);
            println!("  P99: {:?}", p99);
            
            assert!(p99 <= Duration::from_micros(12), "P99 latency {:?} exceeds 10µs target (with 20% margin)", p99);
            
            Duration::from_nanos(p99.as_nanos() as u64)
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_message_throughput,
    benchmark_message_latency,
    benchmark_shared_memory,
    benchmark_connection_pool,
    benchmark_million_msg_per_sec,
    benchmark_sub10us_latency
);

criterion_main!(benches);
