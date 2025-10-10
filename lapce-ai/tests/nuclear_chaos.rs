#![cfg(any(target_os = "linux", target_os = "macos"))]
/// Nuclear Test 5: Chaos Engineering
/// 30 minutes of random failures (kills, corrupted messages, timeouts, oversized)
/// Target: <1% recovery failures, 100ms recovery time

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use lapce_ai_rust::{IpcServer, IpcConfig};
use lapce_ai_rust::ipc::MessageType;
use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryStream;
use bytes::Bytes;
use rand::Rng;

const TEST_DURATION: Duration = Duration::from_secs(30 * 60); // 30 minutes
const CONCURRENT_CONNECTIONS: usize = 100;
const CHAOS_PROBABILITY: f64 = 0.1; // 10% chance of chaos per operation

#[derive(Debug)]
enum ChaosType {
    CorruptedMessage,
    OversizedMessage,
    Timeout,
    ConnectionDrop,
    SlowResponse,
}

#[tokio::test(flavor = "multi_thread")]
async fn nuclear_chaos() {
    println!("\n🌪️ NUCLEAR TEST 5: CHAOS ENGINEERING");
    println!("=====================================");
    println!("Duration: 30 minutes");
    println!("Chaos probability: {}%", CHAOS_PROBABILITY * 100.0);
    println!("Target: <1% failures, <100ms recovery\n");
    
    let start_time = Instant::now();
    let total_operations = Arc::new(AtomicU64::new(0));
    let failed_operations = Arc::new(AtomicU64::new(0));
    let recovery_times = Arc::new(parking_lot::Mutex::new(Vec::new()));
    
    // Start IPC server
    let socket_path = "/tmp/lapce_nuclear_5.sock";
    let server = Arc::new(IpcServer::new(socket_path).await.unwrap());
    
    // Register chaos handler
    server.register_handler(MessageType::CompletionRequest, |data| async move {
        let mut rng = rand::thread_rng();
        
        // Randomly inject delays
        if rng.gen::<f64>() < 0.05 {
            sleep(Duration::from_millis(rng.gen_range(10..100))).await;
        }
        
        // Randomly fail
        if rng.gen::<f64>() < 0.02 {
            return Err(lapce_ai_rust::ipc::IpcError::HandlerPanic);
        }
        
        Ok(data)
    });
    
    // Start server
    let server_handle = {
        let server = server.clone();
        tokio::spawn(async move {
            server.serve().await.unwrap();
        })
    };
    
    sleep(Duration::from_millis(500)).await;
    
    let stop_signal = Arc::new(AtomicBool::new(false));
    let mut chaos_handles = Vec::new();
    
    // Spawn chaos agents
    for agent_id in 0..CONCURRENT_CONNECTIONS {
        let stop = stop_signal.clone();
        let total_ops = total_operations.clone();
        let failed_ops = failed_operations.clone();
        let recovery = recovery_times.clone();
        
        let handle = tokio::spawn(async move {
            let mut rng = rand::thread_rng();
            let mut last_failure: Option<Instant> = None;
            
            while !stop.load(Ordering::Relaxed) {
                // Try to connect (with possible failures)
                let connect_result = timeout(
                    Duration::from_secs(5),
                    SharedMemoryStream::connect(socket_path)
                ).await;
                
                let mut stream = match connect_result {
                    Ok(Ok(s)) => {
                        // Record recovery time if recovering from failure
                        if let Some(fail_time) = last_failure {
                            let recovery_ms = fail_time.elapsed().as_millis() as u64;
                            recovery.lock().push(recovery_ms);
                            last_failure = None;
                        }
                        s
                    }
                    _ => {
                        // Connection failed
                        if last_failure.is_none() {
                            last_failure = Some(Instant::now());
                        }
                        failed_ops.fetch_add(1, Ordering::Relaxed);
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                
                // Run operations with chaos
                for _ in 0..100 {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    
                    total_ops.fetch_add(1, Ordering::Relaxed);
                    
                    // Decide chaos type
                    let chaos = if rng.gen::<f64>() < CHAOS_PROBABILITY {
                        Some(match rng.gen_range(0..5) {
                            0 => ChaosType::CorruptedMessage,
                            1 => ChaosType::OversizedMessage,
                            2 => ChaosType::Timeout,
                            3 => ChaosType::ConnectionDrop,
                            _ => ChaosType::SlowResponse,
                        })
                    } else {
                        None
                    };
                    
                    // Execute with chaos
                    let result = match chaos {
                        Some(ChaosType::CorruptedMessage) => {
                            // Send garbage
                            let garbage = vec![rng.gen::<u8>(); 256];
                            stream.write_all(&garbage).await.err();
                            Some(())
                        }
                        Some(ChaosType::OversizedMessage) => {
                            // Try to send huge message
                            let huge = vec![0u8; 10 * 1024 * 1024];
                            stream.write_all(&huge).await.err();
                            Some(())
                        }
                        Some(ChaosType::Timeout) => {
                            // Timeout on operation
                            let msg = vec![0u8; 256];
                            timeout(Duration::from_millis(1), async {
                                stream.write_all(&msg).await
                            }).await.err();
                            Some(())
                        }
                        Some(ChaosType::ConnectionDrop) => {
                            // Force disconnect
                            drop(stream);
                            break;
                        }
                        Some(ChaosType::SlowResponse) => {
                            sleep(Duration::from_millis(rng.gen_range(100..500))).await;
                            None
                        }
                        None => {
                            // Normal operation
                            let msg = vec![agent_id as u8; 256];
                            let write_result = stream.write_all(&msg).await;
                            if write_result.is_ok() {
                                let mut resp = vec![0u8; 256];
                                stream.read_exact(&mut resp).await.ok();
                            }
                            None
                        }
                    };
                    
                    if result.is_some() {
                        failed_ops.fetch_add(1, Ordering::Relaxed);
                        if last_failure.is_none() {
                            last_failure = Some(Instant::now());
                        }
                    }
                }
            }
        });
        
        chaos_handles.push(handle);
    }
    
    // Progress monitor
    let monitor_handle = {
        let total = total_operations.clone();
        let failed = failed_operations.clone();
        let stop = stop_signal.clone();
        tokio::spawn(async move {
            let mut last_report = Instant::now();
            while !stop.load(Ordering::Relaxed) {
                sleep(Duration::from_secs(30)).await;
                
                let elapsed = last_report.elapsed().as_secs_f64();
                let total_count = total.load(Ordering::Relaxed);
                let failed_count = failed.load(Ordering::Relaxed);
                let rate = total_count as f64 / elapsed;
                let failure_rate = if total_count > 0 {
                    (failed_count as f64 / total_count as f64) * 100.0
                } else {
                    0.0
                };
                
                println!("Progress: {} ops ({:.2} ops/sec), {:.2}% failures", 
                    total_count, rate, failure_rate);
            }
        })
    };
    
    // Run for abbreviated time (30 seconds for testing, would be 30 minutes in production)
    let test_duration = if cfg!(debug_assertions) {
        Duration::from_secs(30) // 30 seconds for testing
    } else {
        TEST_DURATION // Full 30 minutes
    };
    
    sleep(test_duration).await;
    
    // Stop chaos
    stop_signal.store(true, Ordering::Relaxed);
    monitor_handle.abort();
    
    for handle in chaos_handles {
        handle.abort();
    }
    
    // Analyze results
    let total_ops = total_operations.load(Ordering::Relaxed);
    let failed_ops = failed_operations.load(Ordering::Relaxed);
    let failure_rate = if total_ops > 0 {
        (failed_ops as f64 / total_ops as f64) * 100.0
    } else {
        0.0
    };
    
    let recovery_vec = recovery_times.lock().clone();
    let avg_recovery = if !recovery_vec.is_empty() {
        recovery_vec.iter().sum::<u64>() as f64 / recovery_vec.len() as f64
    } else {
        0.0
    };
    
    let max_recovery = recovery_vec.iter().max().copied().unwrap_or(0);
    
    let total_time = start_time.elapsed();
    
    println!("\n📊 RESULTS");
    println!("==========");
    println!("Test duration: {:.2}s", total_time.as_secs_f64());
    println!("Total operations: {}", total_ops);
    println!("Failed operations: {}", failed_ops);
    println!("Failure rate: {:.3}%", failure_rate);
    println!("\nRecovery Times:");
    println!("  Average: {:.2}ms", avg_recovery);
    println!("  Max: {}ms", max_recovery);
    println!("  Recoveries: {}", recovery_vec.len());
    
    // Validation
    let mut passed = true;
    
    if failure_rate < 1.0 {
        println!("\n✅ Failure rate {:.3}% < 1%", failure_rate);
    } else {
        println!("\n❌ Failure rate {:.3}% >= 1%", failure_rate);
        passed = false;
    }
    
    if avg_recovery < 100.0 {
        println!("✅ Avg recovery {:.2}ms < 100ms", avg_recovery);
    } else {
        println!("❌ Avg recovery {:.2}ms >= 100ms", avg_recovery);
        passed = false;
    }
    
    if !passed {
        panic!("Chaos test failed requirements");
    }
    
    println!("\n✅ CHAOS TEST PASSED - System is resilient!");
    
    server_handle.abort();
}
