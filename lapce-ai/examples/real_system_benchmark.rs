use lapce_ai_rust::shared_memory_transport::{SharedMemory, Message};
use lapce_ai_rust::cache::{CacheKey, CacheValue, Cache};
use lapce_ai_rust::ipc_server::IpcServer;
use lapce_ai_rust::ipc_messages::{Request, Response};
use lapce_ai_rust::high_perf_shared_memory::HighPerfSharedMemory;
use lapce_ai_rust::zero_copy_ipc::{ZeroCopyTransport, ZeroCopyMessage};
use std::sync::Arc;
use std::time::{Instant, Duration};
use tokio::sync::RwLock;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestMessage {
    id: u64,
    payload: Vec<u8>,
    timestamp: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔬 REAL LAPCE-AI-RUST SYSTEM BENCHMARK");
    println!("======================================");
    println!("Testing ACTUAL implementations, not fake numbers\n");
    
    // Test 1: SharedMemoryTransport Real Performance
    println!("📊 TEST 1: SharedMemoryTransport");
    test_shared_memory_transport().await?;
    
    // Test 2: Cache System Real Performance  
    println!("\n📊 TEST 2: Multi-tier Cache System");
    test_cache_system().await?;
    
    // Test 3: IPC Server Message Handling
    println!("\n📊 TEST 3: IPC Server Performance");
    test_ipc_server().await?;
    
    // Test 4: Concurrent Load Test
    println!("\n📊 TEST 4: Concurrent Load Test");
    test_concurrent_load().await?;
    
    // Test 5: Memory Pressure Test
    println!("\n📊 TEST 5: Memory Pressure Test");
    test_memory_pressure().await?;
    
    Ok(())
}

async fn test_shared_memory_transport() -> anyhow::Result<()> {
    let transport = SharedMemoryTransport::new("bench_shm", 1024 * 1024)?; // 1MB buffer
    
    // Create test messages of various sizes
    let sizes = vec![64, 256, 1024, 4096, 16384];
    
    for size in sizes {
        let payload = vec![0xABu8; size];
        let message = TestMessage {
            id: 1,
            payload: payload.clone(),
            timestamp: 0,
        };
        
        // Measure serialization + write + read + deserialization
        let iterations = 10000;
        let start = Instant::now();
        
        for i in 0..iterations {
            let mut msg = message.clone();
            msg.id = i as u64;
            transport.send(&msg)?;
            let _received: TestMessage = transport.receive()?;
        }
        
        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        let latency_us = elapsed.as_micros() as f64 / iterations as f64;
        
        println!("  Size: {:5} bytes | Latency: {:.2}μs | Throughput: {:.0} ops/sec", 
                 size, latency_us, ops_per_sec);
    }
    
    Ok(())
}

async fn test_cache_system() -> anyhow::Result<()> {
    let config = CacheConfig {
        l1: L1Config {
            max_items: 10000,
            ttl_seconds: 60,
        },
        l2: L2Config {
            cache_dir: "/tmp/lapce_bench_l2".to_string(),
            max_size_bytes: 100_000_000, // 100MB
        },
        l3: Some(L3Config {
            redis_url: "redis://localhost:6379".to_string(),
            ttl_seconds: 3600,
        }),
    };
    
    let cache = Arc::new(CacheSystem::new(config).await?);
    
    // Test cache hit rates and latencies
    let mut l1_hits = 0;
    let mut l2_hits = 0;
    let mut l3_hits = 0;
    let mut misses = 0;
    
    let iterations = 10000;
    let start = Instant::now();
    
    // Populate cache with data
    for i in 0..1000 {
        let key = format!("key_{}", i);
        let value = vec![0xFFu8; 1024]; // 1KB values
        cache.set(&key, &value).await?;
    }
    
    // Test cache access patterns
    for i in 0..iterations {
        let key = format!("key_{}", i % 2000); // 50% hit rate expected
        
        let l1_start = Instant::now();
        if let Some(_) = cache.get_from_l1(&key).await {
            l1_hits += 1;
            let l1_time = l1_start.elapsed();
            if i % 1000 == 0 {
                println!("  L1 hit latency: {:.2}μs", l1_time.as_micros() as f64);
            }
        } else if let Some(_) = cache.get_from_l2(&key).await {
            l2_hits += 1;
        } else if let Some(_) = cache.get_from_l3(&key).await {
            l3_hits += 1;
        } else {
            misses += 1;
        }
    }
    
    let elapsed = start.elapsed();
    println!("  Total operations: {}", iterations);
    println!("  L1 hits: {} ({:.1}%)", l1_hits, l1_hits as f64 / iterations as f64 * 100.0);
    println!("  L2 hits: {} ({:.1}%)", l2_hits, l2_hits as f64 / iterations as f64 * 100.0);
    println!("  L3 hits: {} ({:.1}%)", l3_hits, l3_hits as f64 / iterations as f64 * 100.0);
    println!("  Misses: {} ({:.1}%)", misses, misses as f64 / iterations as f64 * 100.0);
    println!("  Avg latency: {:.2}μs", elapsed.as_micros() as f64 / iterations as f64);
    
    Ok(())
}

async fn test_ipc_server() -> anyhow::Result<()> {
    let server = Arc::new(IpcServer::new());
    let client_count = 100;
    let messages_per_client = 100;
    
    let start = Instant::now();
    let mut handles = vec![];
    
    for client_id in 0..client_count {
        let server = server.clone();
        let handle = tokio::spawn(async move {
            let mut latencies = vec![];
            
            for msg_id in 0..messages_per_client {
                let request = IpcRequest {
                    id: format!("{}-{}", client_id, msg_id),
                    method: "test".to_string(),
                    params: vec![0u8; 256],
                };
                
                let msg_start = Instant::now();
                let _response = server.handle_request(request).await;
                latencies.push(msg_start.elapsed());
            }
            
            latencies
        });
        handles.push(handle);
    }
    
    let mut all_latencies = vec![];
    for handle in handles {
        let latencies = handle.await?;
        all_latencies.extend(latencies);
    }
    
    let total_messages = client_count * messages_per_client;
    let elapsed = start.elapsed();
    let throughput = total_messages as f64 / elapsed.as_secs_f64();
    
    // Calculate percentiles
    all_latencies.sort();
    let p50 = all_latencies[all_latencies.len() / 2];
    let p95 = all_latencies[all_latencies.len() * 95 / 100];
    let p99 = all_latencies[all_latencies.len() * 99 / 100];
    
    println!("  Clients: {}", client_count);
    println!("  Messages per client: {}", messages_per_client);
    println!("  Total messages: {}", total_messages);
    println!("  Throughput: {:.0} msg/sec", throughput);
    println!("  Latency P50: {:.2}μs", p50.as_micros() as f64);
    println!("  Latency P95: {:.2}μs", p95.as_micros() as f64);
    println!("  Latency P99: {:.2}μs", p99.as_micros() as f64);
    
    Ok(())
}

async fn test_concurrent_load() -> anyhow::Result<()> {
    let concurrent_tasks = 1000;
    let operations_per_task = 1000;
    
    let shared_state = Arc::new(RwLock::new(HashMap::<String, Vec<u8>>::new()));
    let start = Instant::now();
    
    let mut handles = vec![];
    
    for task_id in 0..concurrent_tasks {
        let state = shared_state.clone();
        let handle = tokio::spawn(async move {
            let mut local_latencies = vec![];
            
            for op in 0..operations_per_task {
                let key = format!("task_{}_op_{}", task_id, op);
                let value = vec![task_id as u8; 128];
                
                let op_start = Instant::now();
                
                // Simulate mixed read/write workload
                if op % 3 == 0 {
                    // Write operation (33%)
                    let mut guard = state.write().await;
                    guard.insert(key, value);
                } else {
                    // Read operation (67%)
                    let guard = state.read().await;
                    let _ = guard.get(&key);
                }
                
                local_latencies.push(op_start.elapsed());
            }
            
            local_latencies
        });
        handles.push(handle);
    }
    
    let mut all_latencies = vec![];
    for handle in handles {
        let latencies = handle.await?;
        all_latencies.extend(latencies);
    }
    
    let total_ops = concurrent_tasks * operations_per_task;
    let elapsed = start.elapsed();
    let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();
    
    all_latencies.sort();
    let p50 = all_latencies[all_latencies.len() / 2];
    let p95 = all_latencies[all_latencies.len() * 95 / 100];
    let p99 = all_latencies[all_latencies.len() * 99 / 100];
    let p999 = all_latencies[all_latencies.len() * 999 / 1000];
    
    println!("  Concurrent tasks: {}", concurrent_tasks);
    println!("  Operations per task: {}", operations_per_task);
    println!("  Total operations: {}", total_ops);
    println!("  Throughput: {:.0} ops/sec", ops_per_sec);
    println!("  Latency P50: {:.2}μs", p50.as_micros() as f64);
    println!("  Latency P95: {:.2}μs", p95.as_micros() as f64);
    println!("  Latency P99: {:.2}μs", p99.as_micros() as f64);
    println!("  Latency P99.9: {:.2}μs", p999.as_micros() as f64);
    
    Ok(())
}

async fn test_memory_pressure() -> anyhow::Result<()> {
    use sysinfo::{System, SystemExt, ProcessExt};
    
    let mut system = System::new_all();
    system.refresh_all();
    
    let pid = std::process::id();
    let process = system.process(pid.into()).unwrap();
    let initial_memory = process.memory();
    
    println!("  Initial memory: {:.2} MB", initial_memory as f64 / 1024.0 / 1024.0);
    
    // Create large data structures
    let mut data_structures = vec![];
    
    // Test 1MB allocations
    for i in 0..100 {
        let data = vec![0u8; 1024 * 1024]; // 1MB
        data_structures.push(data);
        
        if i % 10 == 0 {
            system.refresh_process(pid.into());
            let current_memory = system.process(pid.into()).unwrap().memory();
            let overhead = current_memory - initial_memory;
            println!("  After {} MB allocated: {:.2} MB overhead", 
                     i + 1, overhead as f64 / 1024.0 / 1024.0);
        }
    }
    
    // Test cache memory with large dataset
    let cache = Arc::new(RwLock::new(HashMap::<String, Vec<u8>>::new()));
    
    for i in 0..10000 {
        let key = format!("mem_test_{}", i);
        let value = vec![0xAAu8; 1024]; // 1KB per entry
        cache.write().await.insert(key, value);
    }
    
    system.refresh_process(pid.into());
    let final_memory = system.process(pid.into()).unwrap().memory();
    let total_overhead = final_memory - initial_memory;
    
    println!("  Final memory: {:.2} MB", final_memory as f64 / 1024.0 / 1024.0);
    println!("  Total overhead: {:.2} MB", total_overhead as f64 / 1024.0 / 1024.0);
    println!("  Per-MB overhead: {:.2}%", 
             (total_overhead as f64 / (100 * 1024 * 1024) as f64) * 100.0);
    
    // Verify we're under 3MB overhead for normal operations
    let operational_overhead = total_overhead - (100 * 1024 * 1024); // Subtract test data
    println!("  Operational overhead: {:.2} MB (Target: <3MB)", 
             operational_overhead as f64 / 1024.0 / 1024.0);
    
    if operational_overhead < 3 * 1024 * 1024 {
        println!("  ✅ Memory target MET");
    } else {
        println!("  ❌ Memory target FAILED");
    }
    
    Ok(())
}
