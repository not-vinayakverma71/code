/// Comprehensive Unit Tests - HOUR 10
#[cfg(test)]
mod tests {
    use crate::*;
    use crate::ipc_server::*;
    use crate::ipc_messages::*;
    use crate::zero_copy_ipc::*;
    use crate::shared_memory_ipc::*;
    use crate::auto_reconnect::*;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::net::UnixStream;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use bytes::Bytes;
    
    // ========== IPC Messages Tests ==========
    
    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::Echo as u32, 0);
        assert_eq!(MessageType::Complete as u32, 1);
        assert_eq!(MessageType::Stream as u32, 2);
        assert_eq!(MessageType::Cancel as u32, 3);
        assert_eq!(MessageType::Heartbeat as u32, 4);
        assert_eq!(MessageType::Shutdown as u32, 5);
    }
    
    #[test]
    fn test_message_type_from_bytes() {
        let bytes = [0, 0, 0, 0];
        assert_eq!(MessageType::from_bytes(&bytes).unwrap(), MessageType::Echo);
        
        let bytes = [1, 0, 0, 0];
        assert_eq!(MessageType::from_bytes(&bytes).unwrap(), MessageType::Complete);
        
        let bytes = [255, 255, 255, 255];
        assert!(MessageType::from_bytes(&bytes).is_err());
    }
    
    #[test]
    fn test_message_type_to_bytes() {
        assert_eq!(MessageType::Echo.to_bytes(), [0, 0, 0, 0]);
        assert_eq!(MessageType::Complete.to_bytes(), [1, 0, 0, 0]);
        assert_eq!(MessageType::Shutdown.to_bytes(), [5, 0, 0, 0]);
    }
    
    // ========== Zero-Copy IPC Tests ==========
    
    #[test]
    fn test_zero_copy_buffer_creation() {
        let buffer = ZeroCopyRingBuffer::new(10);
        assert_eq!(buffer.capacity, 1024);
        assert_eq!(buffer.mask, 1023);
    }
    
    #[test]
    fn test_zero_copy_write_read() {
        let mut buffer = ZeroCopyRingBuffer::new(10);
        let data = b"test data";
        let mut out = Vec::new();
        
        assert!(buffer.write(data));
        assert!(buffer.read(&mut out));
        assert_eq!(out, data);
    }
    
    #[test]
    fn test_zero_copy_empty_read() {
        let mut buffer = ZeroCopyRingBuffer::new(10);
        let mut out = Vec::new();
        
        assert!(!buffer.read(&mut out));
        assert!(out.is_empty());
    }
    
    #[test]
    fn test_zero_copy_full_buffer() {
        let mut buffer = ZeroCopyRingBuffer::new(6); // 64 bytes
        let data = vec![0u8; 30];
        
        // Should succeed first time
        assert!(buffer.write(&data));
        
        // Should fail when full
        assert!(!buffer.write(&data));
        
        // Read to make space
        let mut out = Vec::new();
        assert!(buffer.read(&mut out));
        
        // Should succeed again
        assert!(buffer.write(&data));
    }
    
    #[test]
    fn test_zero_copy_server_channels() {
        let mut server = ZeroCopyIpcServer::new();
        
        let ch1 = server.create_channel(10);
        let ch2 = server.create_channel(10);
        
        assert_eq!(ch1, 0);
        assert_eq!(ch2, 1);
    }
    
    #[test]
    fn test_zero_copy_server_send_recv() {
        let mut server = ZeroCopyIpcServer::new();
        let channel = server.create_channel(10);
        
        let data = b"hello";
        assert!(server.send(channel, data));
        
        let mut out = Vec::new();
        assert!(server.recv(channel, &mut out));
        assert_eq!(out, data);
    }
    
    #[test]
    fn test_zero_copy_invalid_channel() {
        let server = ZeroCopyIpcServer::new();
        assert!(!server.send(999, b"test"));
        
        let mut out = Vec::new();
        assert!(!server.recv(999, &mut out));
    }
    
    #[test]
    fn test_zero_copy_message_ordering() {
        let mut server = ZeroCopyIpcServer::new();
        let channel = server.create_channel(15);
        
        for i in 0..100u8 {
            assert!(server.send(channel, &[i]));
        }
        
        let mut out = Vec::new();
        for i in 0..100u8 {
            assert!(server.recv(channel, &mut out));
            assert_eq!(out[0], i);
        }
    }
    
    // ========== Shared Memory IPC Tests ==========
    
    #[test]
    fn test_shared_memory_buffer() {
        let buffer = SharedMemoryBuffer::new(100);
        
        let data = vec![1, 2, 3, 4, 5];
        assert!(buffer.send(data.clone()));
        
        let received = buffer.recv().unwrap();
        assert_eq!(received, data);
    }
    
    #[test]
    fn test_shared_memory_capacity() {
        let buffer = SharedMemoryBuffer::new(5);
        
        // Fill buffer
        for _ in 0..5 {
            assert!(buffer.send(vec![0]));
        }
        
        // Should be full
        assert!(!buffer.send(vec![0]));
        assert_eq!(buffer.dropped_count(), 1);
        
        // Make space
        buffer.recv();
        
        // Should work again
        assert!(buffer.send(vec![0]));
    }
    
    #[test]
    fn test_shared_memory_server() {
        let server = SharedMemoryIpcServer::new();
        let channel = server.create_channel(100);
        
        server.send(channel, b"test");
        assert_eq!(server.message_count(), 1);
        
        let data = server.recv(channel).unwrap();
        assert_eq!(data, b"test");
    }
    
    #[test]
    fn test_shared_memory_multiple_channels() {
        let server = SharedMemoryIpcServer::new();
        
        let ch1 = server.create_channel(100);
        let ch2 = server.create_channel(100);
        
        server.send(ch1, b"channel1");
        server.send(ch2, b"channel2");
        
        assert_eq!(server.recv(ch1).unwrap(), b"channel1");
        assert_eq!(server.recv(ch2).unwrap(), b"channel2");
    }
    
    // ========== IPC Server Tests ==========
    
    #[tokio::test]
    async fn test_ipc_server_creation() {
        let server = IpcServer::new("/tmp/test_creation.sock").await.unwrap();
        assert!(std::path::Path::new("/tmp/test_creation.sock").exists());
        drop(server);
        assert!(!std::path::Path::new("/tmp/test_creation.sock").exists());
    }
    
    #[tokio::test]
    async fn test_ipc_server_handler() {
        let server = IpcServer::new("/tmp/test_handler.sock").await.unwrap();
        
        server.register_handler(MessageType::Echo, |data| async move {
            Ok(data)
        });
        
        server.register_handler(MessageType::Complete, |data| async move {
            let mut result = data.to_vec();
            result.reverse();
            Ok(Bytes::from(result))
        });
    }
    
    #[tokio::test]
    async fn test_buffer_pool() {
        let pool = BufferPool::new();
        
        let buf1 = pool.acquire(100);
        assert!(buf1.capacity() >= 4096);
        
        let buf2 = pool.acquire(10000);
        assert!(buf2.capacity() >= 65536);
        
        let buf3 = pool.acquire(100000);
        assert!(buf3.capacity() >= 1048576);
        
        pool.release(buf1);
        pool.release(buf2);
        pool.release(buf3);
    }
    
    #[tokio::test]
    async fn test_metrics_recording() {
        let metrics = Metrics::new();
        
        metrics.record(MessageType::Echo, Duration::from_micros(500));
        metrics.record(MessageType::Complete, Duration::from_micros(5000));
        metrics.record(MessageType::Stream, Duration::from_micros(50000));
        metrics.record(MessageType::Custom, Duration::from_micros(500000));
        
        let prometheus = metrics.export_prometheus();
        assert!(prometheus.contains("ipc_requests_total 4"));
    }
    
    // ========== Auto-Reconnect Tests ==========
    
    #[tokio::test]
    async fn test_reconnecting_client_creation() {
        let client = ReconnectingClient::new("/tmp/test.sock".to_string());
        assert!(!client.is_connected());
        assert_eq!(client.reconnect_count(), 0);
    }
    
    #[tokio::test]
    async fn test_reconnecting_client_connect_failure() {
        let client = ReconnectingClient::new("/tmp/nonexistent_test.sock".to_string());
        
        let result = tokio::time::timeout(
            Duration::from_millis(150),
            client.connect()
        ).await;
        
        assert!(result.is_err() || result.unwrap().is_err());
        assert!(!client.is_connected());
    }
    
    // ========== Performance Tests ==========
    
    #[test]
    fn test_throughput_shared_memory() {
        let server = SharedMemoryIpcServer::new();
        let channel = server.create_channel(10000);
        let data = vec![0u8; 100];
        
        let start = Instant::now();
        for _ in 0..100_000 {
            server.send(channel, &data);
        }
        let elapsed = start.elapsed();
        
        let throughput = 100_000.0 / elapsed.as_secs_f64();
        assert!(throughput > 1_000_000.0, "Throughput {} should exceed 1M msg/sec", throughput);
    }
    
    #[test]
    fn test_throughput_zero_copy() {
        let mut server = ZeroCopyIpcServer::new();
        let channel = server.create_channel(20);
        let data = vec![0u8; 100];
        
        let start = Instant::now();
        for _ in 0..1_000_000 {
            server.send(channel, &data);
        }
        let elapsed = start.elapsed();
        
        let throughput = 1_000_000.0 / elapsed.as_secs_f64();
        assert!(throughput > 10_000_000.0, "Throughput {} should exceed 10M msg/sec", throughput);
    }
    
    #[test]
    fn test_latency_zero_copy() {
        let mut server = ZeroCopyIpcServer::new();
        let channel = server.create_channel(16);
        let data = vec![0u8; 100];
        let mut out = Vec::new();
        
        let start = Instant::now();
        server.send(channel, &data);
        server.recv(channel, &mut out);
        let elapsed = start.elapsed();
        
        assert!(elapsed.as_micros() < 10, "Latency {:?} should be <10μs", elapsed);
    }
    
    // ========== Edge Cases ==========
    
    #[test]
    fn test_empty_message() {
        let mut server = ZeroCopyIpcServer::new();
        let channel = server.create_channel(10);
        
        assert!(server.send(channel, b""));
        
        let mut out = Vec::new();
        assert!(server.recv(channel, &mut out));
        assert!(out.is_empty());
    }
    
    #[test]
    fn test_large_message() {
        let mut server = ZeroCopyIpcServer::new();
        let channel = server.create_channel(20);
        
        let data = vec![0u8; 10000];
        assert!(server.send(channel, &data));
        
        let mut out = Vec::new();
        assert!(server.recv(channel, &mut out));
        assert_eq!(out.len(), 10000);
    }
    
    #[test]
    fn test_concurrent_access() {
        use std::thread;
        use std::sync::Arc;
        
        let server = Arc::new(SharedMemoryIpcServer::new());
        let channel = server.create_channel(10000);
        
        let server1 = server.clone();
        let handle1 = thread::spawn(move || {
            for i in 0..1000 {
                server1.send(channel, &[i as u8]);
            }
        });
        
        let server2 = server.clone();
        let handle2 = thread::spawn(move || {
            let mut count = 0;
            while count < 1000 {
                if server2.recv(channel).is_some() {
                    count += 1;
                }
            }
            count
        });
        
        handle1.join().unwrap();
        let received = handle2.join().unwrap();
        assert_eq!(received, 1000);
    }
    
    #[test]
    fn test_message_count() {
        let server = SharedMemoryIpcServer::new();
        let channel = server.create_channel(100);
        
        assert_eq!(server.message_count(), 0);
        
        for i in 1..=10 {
            server.send(channel, b"test");
            assert_eq!(server.message_count(), i);
        }
    }
}
