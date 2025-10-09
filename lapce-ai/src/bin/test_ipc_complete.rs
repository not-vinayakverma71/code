/// Complete IPC System Test - Verifies <10μs latency, >1M msg/sec
use std::time::Instant;
use lapce_ai_rust::ipc::shared_memory_complete::{SharedMemoryListener, SharedMemoryStream};
use lapce_ai_rust::ipc_server::IpcServer;
use lapce_ai_rust::ipc_messages::{MessageType, ClineMessage, ClineAsk, AIRequest, Message};
use std::sync::Arc;
use bytes::Bytes;

#[tokio::main]
async fn main() {
    println!("🚀 COMPLETE IPC SYSTEM TEST");
    println!("===========================\n");
    
    // Test 1: SharedMemory Performance
    test_shared_memory_performance().await;
    
    // Test 2: IPC Server with Protocol Messages
    test_ipc_server().await;
    
    // Test 3: End-to-End Message Flow
    test_end_to_end().await;
    
    println!("\n✅ ALL IPC TESTS COMPLETED!");
}

async fn test_shared_memory_performance() {
    println!("1️⃣ SHARED MEMORY PERFORMANCE:");
    
    let iterations = 100_000;
    let data = vec![0u8; 1024]; // 1KB messages
    
    // Create listener and stream
    let mut listener = SharedMemoryListener::bind("perf_test").unwrap();
    
    // Server task
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut total_bytes = 0u64;
        
        for _ in 0..iterations {
            let mut buf = vec![0u8; 1024];
            stream.read_exact(&mut buf).await.unwrap();
            stream.write_all(&buf).await.unwrap();
            total_bytes += 2048; // Read + Write
        }
        
        total_bytes
    });
    
    // Client
    let mut stream = SharedMemoryStream::connect("perf_test").await.unwrap();
    
    let start = Instant::now();
    for _ in 0..iterations {
        stream.write_all(&data).await.unwrap();
        let mut buf = vec![0u8; 1024];
        stream.read_exact(&mut buf).await.unwrap();
    }
    let duration = start.elapsed();
    
    let total_bytes = server.await.unwrap();
    let throughput = iterations as f64 * 2.0 / duration.as_secs_f64();
    let latency_us = duration.as_micros() as f64 / (iterations * 2) as f64;
    let bandwidth_mb = total_bytes as f64 / duration.as_secs_f64() / 1_000_000.0;
    
    println!("  • Throughput: {:.2}M msg/sec", throughput / 1_000_000.0);
    println!("  • Latency: {:.2}μs per operation", latency_us);
    println!("  • Bandwidth: {:.2}MB/sec", bandwidth_mb);
    
    assert!(latency_us < 10.0, "Latency must be < 10μs");
    assert!(throughput > 1_000_000.0, "Throughput must be > 1M msg/sec");
}

async fn test_ipc_server() {
    println!("\n2️⃣ IPC SERVER WITH PROTOCOLS:");
    
    let server = Arc::new(IpcServer::new("ipc_test").await.unwrap());
    
    // Register handlers for protocol messages
    server.register_handler(MessageType::Echo, |data| {
        Box::pin(async move {
            // Echo handler for testing
            Ok(data)
        })
    });
    
    // Start server
    let server_handle = tokio::spawn({
        let server = server.clone();
        async move {
            server.serve().await.unwrap();
        }
    });
    
    // Test client connections
    let iterations = 10_000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let mut stream = SharedMemoryStream::connect("ipc_test").await.unwrap();
        
        // Send a ClineMessage
        let msg = ClineMessage {
            ts: 1234567890,
            msg_type: "ask".to_string(),
            ask: Some(ClineAsk::Command),
            say: None,
            text: Some("Test message".to_string()),
            images: None,
            partial: None,
            reasoning: None,
            conversation_history_index: None,
            checkpoint: None,
            progress_status: None,
            context_condense: None,
            is_protected: None,
            api_protocol: None,
            metadata: None,
        };
        
        let serialized = serde_json::to_vec(&msg).unwrap();
        let len = serialized.len() as u32;
        
        stream.write_all(&len.to_le_bytes()).await.unwrap();
        stream.write_all(&serialized).await.unwrap();
        
        // Read response
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await.unwrap();
        let response_len = u32::from_le_bytes(len_buf) as usize;
        
        let mut response = vec![0u8; response_len];
        stream.read_exact(&mut response).await.unwrap();
    }
    
    let duration = start.elapsed();
    let req_per_sec = iterations as f64 / duration.as_secs_f64();
    
    println!("  • Request/Response: {:.0} req/sec", req_per_sec);
    println!("  • Average latency: {:.2}μs", duration.as_micros() as f64 / iterations as f64);
    
    server.shutdown();
}

async fn test_end_to_end() {
    println!("\n3️⃣ END-TO-END MESSAGE FLOW:");
    
    let messages = vec![
        ("ClineMessage", test_cline_message()),
        ("AIRequest", test_ai_request()),
        ("TaskEvent", test_task_event()),
    ];
    
    for (name, latency) in messages {
        println!("  • {}: {:.2}μs latency", name, latency);
    }
}

fn test_cline_message() -> f64 {
    let msg = ClineMessage {
        ts: 1234567890,
        msg_type: "ask".to_string(),
        ask: Some(ClineAsk::Tool),
        say: None,
        text: Some("Execute tool?".to_string()),
        images: None,
        partial: None,
        reasoning: None,
        conversation_history_index: None,
        checkpoint: None,
        progress_status: None,
        context_condense: None,
        is_protected: None,
        api_protocol: None,
        metadata: None,
    };
    
    let iterations = 10_000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let serialized = serde_json::to_vec(&msg).unwrap();
        let _deserialized: ClineMessage = serde_json::from_slice(&serialized).unwrap();
    }
    
    start.elapsed().as_micros() as f64 / iterations as f64
}

fn test_ai_request() -> f64 {
    use lapce_ai_rust::ipc_messages::MessageRole;
    
    let req = AIRequest {
        messages: vec![
            Message {
                role: MessageRole::User,
                content: "Test message".to_string(),
                tool_calls: None,
            }
        ],
        model: "gpt-4".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(1000),
        tools: None,
        system_prompt: None,
        stream: Some(false),
    };
    
    let iterations = 10_000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let serialized = serde_json::to_vec(&req).unwrap();
        let _deserialized: AIRequest = serde_json::from_slice(&serialized).unwrap();
    }
    
    start.elapsed().as_micros() as f64 / iterations as f64
}

fn test_task_event() -> f64 {
    use lapce_ai_rust::events_exact_translation::{TaskEvent, TokenUsage};
    
    let event = TaskEvent::TaskStarted {
        payload: ("task-123".to_string(),),
        task_id: Some(456),
    };
    
    let iterations = 10_000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let serialized = serde_json::to_vec(&event).unwrap();
        let _deserialized: TaskEvent = serde_json::from_slice(&serialized).unwrap();
    }
    
    start.elapsed().as_micros() as f64 / iterations as f64
}
