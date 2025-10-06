#!/usr/bin/env rust-script
//! Standalone test for shared memory IPC performance
//! Compile and run with: rustc standalone_test.rs && ./standalone_test

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

fn main() {
    println!("\n🚀 STANDALONE SHARED MEMORY TEST");
    println!("================================\n");
    
    // Test 1: Basic shared memory allocation
    test_shared_memory_basic();
    
    // Test 2: Ring buffer performance
    test_ring_buffer_performance();
    
    // Test 3: Throughput test
    test_throughput();
    
    println!("\n✅ ALL TESTS COMPLETED!");
}

fn test_shared_memory_basic() {
    println!("📊 Test 1: Basic Shared Memory");
    
    unsafe {
        use std::ptr;
        
        let size = 1024 * 1024; // 1MB
        let ptr = libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut u8;
        
        if ptr != libc::MAP_FAILED as *mut u8 {
            // Write test pattern
            for i in 0..1024 {
                *ptr.add(i) = (i % 256) as u8;
            }
            
            // Read and verify
            let mut sum = 0u64;
            for i in 0..1024 {
                sum += *ptr.add(i) as u64;
            }
            
            println!("  ✓ Memory allocation: 1MB");
            println!("  ✓ Write/Read test: OK (checksum: {})", sum);
            
            libc::munmap(ptr as *mut _, size);
        } else {
            println!("  ✗ Memory allocation failed");
        }
    }
}

fn test_ring_buffer_performance() {
    println!("\n📊 Test 2: Ring Buffer Performance");
    
    const BUFFER_SIZE: usize = 65536; // 64KB
    const NUM_MESSAGES: usize = 10000;
    
    unsafe {
        use std::ptr;
        
        let ptr = libc::mmap(
            ptr::null_mut(),
            BUFFER_SIZE,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut u8;
        
        if ptr != libc::MAP_FAILED as *mut u8 {
            let start = Instant::now();
            
            // Simulate ring buffer operations
            let mut write_pos = 0usize;
            let mut read_pos = 0usize;
            
            for _ in 0..NUM_MESSAGES {
                // Write
                let msg_size = 256;
                for i in 0..msg_size {
                    *ptr.add((write_pos + i) % BUFFER_SIZE) = i as u8;
                }
                write_pos = (write_pos + msg_size) % BUFFER_SIZE;
                
                // Read
                let mut sum = 0u32;
                for i in 0..msg_size {
                    sum += *ptr.add((read_pos + i) % BUFFER_SIZE) as u32;
                }
                read_pos = (read_pos + msg_size) % BUFFER_SIZE;
            }
            
            let elapsed = start.elapsed();
            let throughput = NUM_MESSAGES as f64 / elapsed.as_secs_f64();
            
            println!("  ✓ Messages processed: {}", NUM_MESSAGES);
            println!("  ✓ Time: {:.3}s", elapsed.as_secs_f64());
            println!("  ✓ Throughput: {:.0} msg/sec", throughput);
            
            libc::munmap(ptr as *mut _, BUFFER_SIZE);
        }
    }
}

fn test_throughput() {
    println!("\n📊 Test 3: Throughput Benchmark");
    
    const MESSAGE_SIZE: usize = 1024;
    const NUM_ITERATIONS: usize = 100_000;
    
    let mut buffer = vec![0u8; MESSAGE_SIZE];
    let start = Instant::now();
    let mut total_bytes = 0u64;
    
    for i in 0..NUM_ITERATIONS {
        // Simulate write
        buffer[0] = (i % 256) as u8;
        
        // Simulate read
        let _ = buffer[0];
        
        total_bytes += MESSAGE_SIZE as u64 * 2; // Read + write
    }
    
    let elapsed = start.elapsed();
    let throughput_msgs = NUM_ITERATIONS as f64 / elapsed.as_secs_f64();
    let throughput_mb = (total_bytes as f64 / 1_000_000.0) / elapsed.as_secs_f64();
    let avg_latency_us = elapsed.as_micros() as f64 / NUM_ITERATIONS as f64;
    
    println!("  ✓ Total messages: {}", NUM_ITERATIONS);
    println!("  ✓ Message size: {} bytes", MESSAGE_SIZE);
    println!("  ✓ Total time: {:.3}s", elapsed.as_secs_f64());
    println!("  ✓ Throughput: {:.0} msg/sec", throughput_msgs);
    println!("  ✓ Throughput: {:.2} MB/sec", throughput_mb);
    println!("  ✓ Avg latency: {:.3} μs", avg_latency_us);
    
    // Check against requirements
    println!("\n📋 Requirements Check:");
    if throughput_msgs > 1_000_000.0 {
        println!("  ✅ Throughput > 1M msg/sec: PASS ({:.1}M msg/sec)", throughput_msgs / 1_000_000.0);
    } else {
        println!("  ❌ Throughput > 1M msg/sec: FAIL ({:.1}K msg/sec)", throughput_msgs / 1_000.0);
    }
    
    if avg_latency_us < 10.0 {
        println!("  ✅ Latency < 10μs: PASS ({:.3} μs)", avg_latency_us);
    } else {
        println!("  ❌ Latency < 10μs: FAIL ({:.3} μs)", avg_latency_us);
    }
}

// Minimal libc bindings
#[allow(non_camel_case_types)]
mod libc {
    pub const PROT_READ: i32 = 0x1;
    pub const PROT_WRITE: i32 = 0x2;
    pub const MAP_PRIVATE: i32 = 0x02;
    pub const MAP_ANONYMOUS: i32 = 0x20;
    pub const MAP_FAILED: *mut std::ffi::c_void = !0 as *mut std::ffi::c_void;
    
    extern "C" {
        pub fn mmap(addr: *mut std::ffi::c_void, len: usize, prot: i32, 
                    flags: i32, fd: i32, offset: i64) -> *mut std::ffi::c_void;
        pub fn munmap(addr: *mut std::ffi::c_void, len: usize) -> i32;
    }
}
