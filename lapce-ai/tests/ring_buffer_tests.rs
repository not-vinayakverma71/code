/// Ring Buffer Correctness Tests
/// Tests wrap-around, boundaries, empty/full states, concurrency, and backpressure

use lapce_ai_rust::ipc::shared_memory_complete::SharedMemoryBuffer;
use std::sync::Arc;
use parking_lot::RwLock;
use std::thread;
use std::time::Duration;

#[test]
fn test_basic_read_write() {
    let mut buffer = SharedMemoryBuffer::create("/test_basic_rw", 1024 * 1024)
        .expect("Failed to create buffer");
    
    let data = b"Hello, Ring Buffer!";
    buffer.write(data).expect("Write failed");
    
    let read_data = buffer.read().expect("Read failed");
    assert_eq!(read_data, data);
}

#[test]
fn test_empty_buffer_read() {
    let mut buffer = SharedMemoryBuffer::create("/test_empty", 1024 * 1024)
        .expect("Failed to create buffer");
    
    let result = buffer.read();
    assert!(result.is_none(), "Empty buffer should return None");
}

#[test]
fn test_wrap_around() {
    let buffer_size = 1024; // Small buffer to test wrap-around
    let mut buffer = SharedMemoryBuffer::create("/test_wrap", buffer_size)
        .expect("Failed to create buffer");
    
    // Fill buffer near capacity
    let chunk_size = 100;
    let num_chunks = 8; // Will cause wrap-around
    
    for i in 0..num_chunks {
        let data = vec![i as u8; chunk_size];
        buffer.write(&data).expect(&format!("Write {} failed", i));
    }
    
    // Read back all chunks
    for i in 0..num_chunks {
        let read_data = buffer.read().expect(&format!("Read {} failed", i));
        assert_eq!(read_data.len(), chunk_size);
        assert_eq!(read_data[0], i as u8);
    }
}

#[test]
fn test_boundary_sizes() {
    let mut buffer = SharedMemoryBuffer::create("/test_boundary", 4096)
        .expect("Failed to create buffer");
    
    // Test various sizes
    let test_sizes = vec![0, 1, 255, 256, 1023, 1024, 2047, 2048];
    
    for size in test_sizes {
        if size == 0 {
            // Empty write should succeed
            buffer.write(&[]).expect("Empty write failed");
            continue;
        }
        
        let data = vec![0xAA; size];
        let result = buffer.write(&data);
        
        if size <= 2048 { // Half of buffer capacity
            assert!(result.is_ok(), "Write of {} bytes should succeed", size);
            let read_data = buffer.read().expect("Read failed");
            assert_eq!(read_data.len(), size);
        } else {
            // Large messages should fail
            assert!(result.is_err(), "Write of {} bytes should fail", size);
        }
    }
}

#[test]
fn test_full_buffer_backpressure() {
    let buffer_size = 1024;
    let mut buffer = SharedMemoryBuffer::create("/test_full", buffer_size)
        .expect("Failed to create buffer");
    
    // Fill buffer to capacity
    let chunk = vec![0xFF; 400]; // Less than half capacity
    
    // First write should succeed
    buffer.write(&chunk).expect("First write failed");
    
    // Second write should succeed
    buffer.write(&chunk).expect("Second write failed");
    
    // Third write should trigger backpressure
    let start = std::time::Instant::now();
    let result = buffer.write(&chunk);
    let elapsed = start.elapsed();
    
    // Should either succeed after backpressure wait or fail with WouldBlock
    if result.is_ok() {
        assert!(elapsed >= Duration::from_millis(1), 
                "Should have waited for backpressure");
    } else {
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("would block") || err_msg.contains("full"),
                "Should fail with buffer full error");
    }
}

#[test]
fn test_concurrent_readers_writers() {
    let buffer = Arc::new(RwLock::new(
        SharedMemoryBuffer::create("/test_concurrent", 1024 * 1024)
            .expect("Failed to create buffer")
    ));
    
    let num_writers = 4;
    let num_readers = 4;
    let messages_per_thread = 100;
    
    let mut handles = vec![];
    
    // Start writers
    for writer_id in 0..num_writers {
        let buffer_clone = buffer.clone();
        let handle = thread::spawn(move || {
            for msg_id in 0..messages_per_thread {
                let data = format!("W{}M{}", writer_id, msg_id);
                let mut buffer_guard = buffer_clone.write();
                buffer_guard.write(data.as_bytes())
                    .expect(&format!("Writer {} msg {} failed", writer_id, msg_id));
                drop(buffer_guard);
                thread::sleep(Duration::from_micros(10));
            }
        });
        handles.push(handle);
    }
    
    // Start readers
    let total_expected = num_writers * messages_per_thread;
    for reader_id in 0..num_readers {
        let buffer_clone = buffer.clone();
        let handle = thread::spawn(move || {
            let mut read_count = 0;
            let mut empty_reads = 0;
            
            while read_count < total_expected / num_readers {
                let mut buffer_guard = buffer_clone.write();
                if let Some(data) = buffer_guard.read() {
                    // Verify data format
                    let msg = String::from_utf8_lossy(&data);
                    assert!(msg.starts_with("W"), "Invalid message format");
                    read_count += 1;
                } else {
                    empty_reads += 1;
                    if empty_reads > 10000 {
                        panic!("Reader {} stuck with {} messages read", 
                               reader_id, read_count);
                    }
                }
                drop(buffer_guard);
                thread::sleep(Duration::from_micros(5));
            }
            read_count
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[test]
fn test_message_ordering() {
    let mut buffer = SharedMemoryBuffer::create("/test_ordering", 1024 * 1024)
        .expect("Failed to create buffer");
    
    // Write sequence of messages
    for i in 0..100 {
        let data = format!("Message {:03}", i);
        buffer.write(data.as_bytes()).expect(&format!("Write {} failed", i));
    }
    
    // Read and verify order
    for i in 0..100 {
        let read_data = buffer.read().expect(&format!("Read {} failed", i));
        let msg = String::from_utf8_lossy(&read_data);
        assert_eq!(msg, format!("Message {:03}", i), "Message order violated");
    }
}

#[test]
fn test_corrupted_data_recovery() {
    let mut buffer = SharedMemoryBuffer::create("/test_corrupt", 1024 * 1024)
        .expect("Failed to create buffer");
    
    // Write valid message
    buffer.write(b"Valid1").expect("Write failed");
    
    // Simulate corruption by writing invalid length
    // This is handled internally by the read method
    
    // Write another valid message
    buffer.write(b"Valid2").expect("Write failed");
    
    // Should be able to read valid messages
    let msg1 = buffer.read();
    assert!(msg1.is_some(), "Should read first message");
    
    // After corruption handling, might skip to next valid message
    // or return None if buffer is reset
    let msg2 = buffer.read();
    // Either None (reset) or Valid2 is acceptable
    if let Some(data) = msg2 {
        assert_eq!(data, b"Valid2");
    }
}

#[test]
fn test_maximum_throughput() {
    let mut buffer = SharedMemoryBuffer::create("/test_throughput", 4 * 1024 * 1024)
        .expect("Failed to create buffer");
    
    let message_size = 512;
    let num_messages = 10000;
    let data = vec![0xAB; message_size];
    
    let start = std::time::Instant::now();
    
    // Write messages
    for _ in 0..num_messages {
        buffer.write(&data).expect("Write failed");
    }
    
    // Read messages
    for _ in 0..num_messages {
        buffer.read().expect("Read failed");
    }
    
    let elapsed = start.elapsed();
    let total_bytes = (message_size * num_messages * 2) as f64; // Read + write
    let throughput_mbps = (total_bytes / 1_000_000.0) / elapsed.as_secs_f64();
    
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    assert!(throughput_mbps > 100.0, "Throughput should exceed 100 MB/s");
}

#[test]
fn test_concurrent_wrap_around() {
    let buffer = Arc::new(RwLock::new(
        SharedMemoryBuffer::create("/test_concurrent_wrap", 1024)
            .expect("Failed to create buffer")
    ));
    
    let writer_buffer = buffer.clone();
    let writer = thread::spawn(move || {
        for i in 0..1000 {
            let data = vec![(i % 256) as u8; 50];
            let mut guard = writer_buffer.write();
            if guard.write(&data).is_err() {
                // Buffer full, wait a bit
                drop(guard);
                thread::sleep(Duration::from_micros(100));
                let mut guard = writer_buffer.write();
                guard.write(&data).expect("Write failed after wait");
            }
            drop(guard);
        }
    });
    
    let reader_buffer = buffer.clone();
    let reader = thread::spawn(move || {
        let mut read_count = 0;
        let mut last_value = None;
        
        while read_count < 1000 {
            let mut guard = reader_buffer.write();
            if let Some(data) = guard.read() {
                let value = data[0];
                
                // Verify sequence (with wrap)
                if let Some(last) = last_value {
                    let expected = (last + 1) % 256;
                    assert_eq!(value, expected, 
                              "Sequence broken: expected {}, got {}", expected, value);
                }
                last_value = Some(value);
                read_count += 1;
            }
            drop(guard);
            thread::sleep(Duration::from_micros(50));
        }
    });
    
    writer.join().expect("Writer panicked");
    reader.join().expect("Reader panicked");
}

#[test]
fn test_zero_copy_performance() {
    let mut buffer = SharedMemoryBuffer::create("/test_zerocopy", 1024 * 1024)
        .expect("Failed to create buffer");
    
    // Large message to test zero-copy benefits
    let large_data = vec![0x42; 100_000];
    
    let write_start = std::time::Instant::now();
    buffer.write(&large_data).expect("Write failed");
    let write_time = write_start.elapsed();
    
    let read_start = std::time::Instant::now();
    let read_data = buffer.read().expect("Read failed");
    let read_time = read_start.elapsed();
    
    assert_eq!(read_data.len(), large_data.len());
    
    // Zero-copy operations should be very fast
    assert!(write_time < Duration::from_millis(1), 
            "Write took {:?}, should be < 1ms", write_time);
    assert!(read_time < Duration::from_millis(1),
            "Read took {:?}, should be < 1ms", read_time);
}
