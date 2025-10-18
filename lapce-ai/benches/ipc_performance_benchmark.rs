use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use tokio::runtime::Runtime;
use lapce_ai_rust::ipc_server_complete::{
    server_final::IpcServer,
    protocol::{AIRequest, Message, MessageRole},
    server_zero_copy::{ZeroCopyServer, MessageType, ConnectionMetrics},
    buffer_pool_optimized::{BufferPool, ThreadSafeBufferPool},
    connection_pool_optimized::ConnectionPool,
};

/// Benchmark message processing - EXACT from documentation
fn bench_message_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_processing");
    let rt = Runtime::new().unwrap();
    
    // Setup server
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("bench.sock");
    
    let server = rt.block_on(async {
        let server = ZeroCopyServer::new();
        
        // Register echo handler
        server.register_handler(MessageType::Request, |data| async move {
            Ok(data) // Echo handler
        });
        
        server
    });
    
    // Test messages of different sizes
    for size in [64, 256, 1024, 4096, 16384].iter() {
        let test_message = vec![0u8, 0, 0, 0]; // MessageType header
        let mut full_message = test_message.clone();
        full_message.extend(vec![42u8; *size]); // Payload
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("process_message", size),
            size,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let mut metrics = ConnectionMetrics::default();
                    let result = server.process_message(&full_message, &mut metrics).await.unwrap();
                    black_box(result);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark latency - verify < 10μs per message
fn bench_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency");
    let rt = Runtime::new().unwrap();
    
    let server = rt.block_on(async {
        let server = ZeroCopyServer::new();
        server.register_handler(MessageType::Request, |data| async move {
            Ok(data)
        });
        server
    });
    
    let test_message = vec![0u8, 0, 0, 0, 42]; // Minimal message
    
    group.bench_function("single_message_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let start = Instant::now();
            let mut metrics = ConnectionMetrics::default();
            let _ = server.process_message(&test_message, &mut metrics).await.unwrap();
            let elapsed = start.elapsed();
            
            // Verify < 10μs
            assert!(elapsed.as_micros() < 10, 
                   "Latency {} μs exceeds 10μs requirement", elapsed.as_micros());
            
            black_box(elapsed);
        });
    });
    
    group.finish();
}

/// Benchmark throughput - verify > 1M messages/second
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    let rt = Runtime::new().unwrap();
    
    let server = rt.block_on(async {
        let server = ZeroCopyServer::new();
        server.register_handler(MessageType::Request, |data| async move {
            Ok(data)
        });
        server
    });
    
    let test_message = vec![0u8, 0, 0, 0, 42]; // Minimal message
    
    group.bench_function("messages_per_second", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let start = Instant::now();
            let mut metrics = ConnectionMetrics::default();
            
            for _ in 0..iters {
                let _ = server.process_message(&test_message, &mut metrics).await.unwrap();
            }
            
            let elapsed = start.elapsed();
            let messages_per_sec = (iters as f64) / elapsed.as_secs_f64();
            
            // Verify > 1M messages/second
            println!("Throughput: {:.0} messages/second", messages_per_sec);
            assert!(messages_per_sec > 1_000_000.0, 
                   "Throughput {:.0} msg/s below 1M requirement", messages_per_sec);
            
            elapsed
        });
    });
    
    group.finish();
}

/// Benchmark buffer pool performance
fn bench_buffer_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_pool");
    
    // Test acquire/release cycle
    group.bench_function("acquire_release_small", |b| {
        let mut pool = BufferPool::new();
        
        b.iter(|| {
            let buffer = pool.acquire(1024);
            black_box(&buffer);
            pool.release(buffer);
        });
    });
    
    group.bench_function("acquire_release_medium", |b| {
        let mut pool = BufferPool::new();
        
        b.iter(|| {
            let buffer = pool.acquire(32768);
            black_box(&buffer);
            pool.release(buffer);
        });
    });
    
    group.bench_function("acquire_release_large", |b| {
        let mut pool = BufferPool::new();
        
        b.iter(|| {
            let buffer = pool.acquire(524288);
            black_box(&buffer);
            pool.release(buffer);
        });
    });
    
    // Test thread-safe pool
    group.bench_function("thread_safe_pool", |b| {
        let pool = ThreadSafeBufferPool::new();
        
        b.iter(|| {
            let buffer = pool.acquire(4096);
            black_box(&buffer);
            pool.release(buffer);
        });
    });
    
    group.finish();
}

/// Benchmark connection pool
fn bench_connection_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_pool");
    let rt = Runtime::new().unwrap();
    
    group.bench_function("acquire_release", |b| {
        b.to_async(&rt).iter(|| async {
            let pool = ConnectionPool::new(100, Duration::from_secs(300));
            
            // Simulate connection operations
            let dir = tempdir().unwrap();
            let socket_path = dir.path().join("test.sock");
            let _listener = tokio::net::UnixListener::bind(&socket_path).unwrap();
            
            if let Ok(conn) = pool.create_new(socket_path.to_str().unwrap()).await {
                pool.release(conn);
            }
        });
    });
    
    group.finish();
}

/// Verify memory footprint is 2-3MB
fn bench_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");
    let rt = Runtime::new().unwrap();
    
    group.bench_function("total_memory_usage", |b| {
        b.to_async(&rt).iter(|| async {
            // Measure memory before
            let before = get_memory_usage();
            
            // Create all components
            let _server = ZeroCopyServer::new();
            let _buffer_pool = BufferPool::new();
            let _conn_pool = ConnectionPool::new(100, Duration::from_secs(300));
            
            // Measure after
            let after = get_memory_usage();
            let used_mb = (after - before) as f64 / (1024.0 * 1024.0);
            
            // Verify 2-3MB
            println!("Memory usage: {:.2} MB", used_mb);
            assert!(used_mb >= 2.0 && used_mb <= 3.0, 
                   "Memory {:.2}MB outside 2-3MB range", used_mb);
            
            black_box(used_mb);
        });
    });
    
    group.finish();
}

fn get_memory_usage() -> usize {
    // Simple approximation - in production use jemalloc stats
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
    
    struct TrackingAllocator;
    
    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            System.alloc(layout)
        }
        
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
            System.dealloc(ptr, layout)
        }
    }
    
    ALLOCATED.load(Ordering::Relaxed)
}

criterion_group!(
    benches,
    bench_message_processing,
    bench_latency,
    bench_throughput,
    bench_buffer_pool,
    bench_connection_pool,
    bench_memory_footprint
);

criterion_main!(benches);
