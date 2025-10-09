# Step 1: IPC Server Implementation Documentation

## Overview
Production-grade IPC system with shared memory transport achieving ≥1M msg/s throughput and ≤10µs p99 latency.

## Architecture AI Message Protocol
## High-Performance SHared Memory Communication with AI Orchestration

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED: 1:1 TYPESCRIPT TO RUST PORT ONLY
**THIS IS NOT A REWRITE - IT'S A TRANSLATION**

**READ EVERY FILE IN**: `/home/verma/lapce/Codex`
- Years of battle-tested AI logic - DO NOT CHANGE
- ONLY translate TypeScript → Rust
- Keep EXACT same algorithms, flow, decisions
- Same function names (just snake_case)
- Same parameters, same returns, same errors
- The  Document is updated to use Shared Memory that bypasses Kernel , don`t use unix socket it has fundamental hardware limit to meet success criteria.
## ✅ Success Criteria
- [ ] **Memory Usage**: < 3MB total footprint
- [ ] **Latency**: < 10μs per message round-trip
- [ ] **Throughput**: > 1M messages/second
- [ ] **Connections**: Support 1000+ concurrent connections
- [ ] **Zero Allocations**: No heap allocations in hot path
- [ ] **Error Recovery**: Automatic reconnection within 100ms
- [ ] **Test Coverage**: > 90% code coverage
- [ ] **Benchmark**: Outperform Node.js IPC by 10x

## Overview
The IPC (Inter-Process Communication) server is the foundation of our Rust port, handling all communication between the IDE and AI services. This implementation achieves 95% memory reduction compared to Node.js.

Shared Memory

## AI Message Protocol (MUST MATCH EXACTLY)

### Message Types from Codex
```typescript
// From codex-reference - MUST port exactly
interface AIRequest {
    messages: Message[];
    model: string;
    temperature?: number;
    maxTokens?: number;
    tools?: Tool[];
    systemPrompt?: string;
    stream?: boolean;
}

interface Message {
    role: "system" | "user" | "assistant";
    content: string;
    toolCalls?: ToolCall[];
}

interface ToolCall {
    name: string;
    parameters: any;
    id: string;
}
```

### Rust Implementation MUST Match
```rust
#[derive(Serialize, Deserialize, Archive)]
pub struct AIRequest {
    pub messages: Vec<Message>,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<Tool>>,
    pub system_prompt: Option<String>,
    pub stream: Option<bool>,
}

// EXACT same structure as TypeScript
#[derive(Serialize, Deserialize, Archive)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}
```

## Architecture Design

### Core Components
```rust
use tokio::net::Shared Memory Listener;
use tokio::sync::mpsc;
use dashmap::DashMap;
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use parking_lot::RwLock;

pub struct IpcServer {
    //Shared Memory  domain socket listener
    listener: Shared Memory Listener,
    
    // Handler registry - lock-free concurrent hashmap
    handlers: Arc<DashMap<MessageType, Handler>>,
    
    // Connection pool for reusing connections
    connections: Arc<ConnectionPool>,
    
    // Metrics collector
    metrics: Arc<Metrics>,
    
    // Shutdown signal
    shutdown: tokio::sync::broadcast::Sender<()>,
}
```

## Implementation Details

### 1. Socket Setup and Binding
```rust
impl IpcServer {
    pub async fn new(socket_path: &str) -> Result<Self> {
        // Remove existing socket file if it exists
        if std::path::Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path)?;
        }
        
        // Create Shared Memory domain socket
        let listener = Shared Memory Listener::bind(socket_path)?;
        
        // Set socket permissions (owner read/write only)
        std::fs::set_permissions(
            socket_path,
            std::fs::Permissions::from_mode(0o600)
        )?;
        
        // Pre-allocate handler map with expected capacity
        let handlers = Arc::new(DashMap::with_capacity(32));
        
        // Initialize connection pool
        let connections = Arc::new(ConnectionPool::new(
            100,  // max connections
            Duration::from_secs(300),  // idle timeout
        ));
        
        Ok(Self {
            listener,
            handlers,
            connections,
            metrics: Arc::new(Metrics::new()),
            shutdown: tokio::sync::broadcast::channel(1).0,
        })
    }
}
```

### 2. Connection Handling with Backpressure
```rust
pub async fn serve(self: Arc<Self>) -> Result<()> {
    // Use semaphore for connection limiting
    let semaphore = Arc::new(tokio::sync::Semaphore::new(100));
    
    loop {
        tokio::select! {
            // Accept new connections
            result = self.listener.accept() => {
                let (stream, _) = result?;
                let permit = semaphore.clone().acquire_owned().await?;
                
                // Spawn handler task
                let server = self.clone();
                tokio::spawn(async move {
                    let _permit = permit; // Hold permit until done
                    if let Err(e) = server.handle_connection(stream).await {
                        tracing::error!("Connection error: {}", e);
                    }
                });
            }
            
            // Shutdown signal
            _ = self.shutdown.subscribe().recv() => {
                tracing::info!("IPC server shutting down");
                break;
            }
        }
    }
    
    Ok(())
}
```

### 3. Zero-Copy Message Processing
```rust
async fn handle_connection(&self, mut stream: Shared MemoryStream) -> Result<()> {
    // Reuse buffer for entire connection lifetime
    let mut buffer = BytesMut::with_capacity(8192);
    
    // Connection-specific metrics
    let mut conn_metrics = ConnectionMetrics::default();
    
    loop {
        // Read message length (4 bytes)
        stream.read_exact(&mut buffer[..4]).await?;
        let msg_len = u32::from_le_bytes([
            buffer[0], buffer[1], buffer[2], buffer[3]
        ]) as usize;
        
        // Validate message size
        if msg_len > MAX_MESSAGE_SIZE {
            return Err(Error::MessageTooLarge(msg_len));
        }
        
        // Ensure buffer capacity without reallocation
        if buffer.capacity() < msg_len {
            buffer.reserve(msg_len - buffer.len());
        }
        
        // Read message body
        unsafe {
            buffer.set_len(msg_len);
        }
        stream.read_exact(&mut buffer[..msg_len]).await?;
        
        // Process without copying
        let response = self.process_message(&buffer[..msg_len], &mut conn_metrics).await?;
        
        // Write response directly
        stream.write_all(&response).await?;
        
        // Clear buffer for reuse
        buffer.clear();
    }
}
```

### 4. Handler Registration and Dispatch
```rust
pub fn register_handler<F, Fut>(&self, msg_type: MessageType, handler: F)
where
    F: Fn(Bytes) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Bytes>> + Send,
{
    self.handlers.insert(msg_type, Box::new(move |data| {
        Box::pin(handler(data))
    }));
}

async fn process_message(&self, data: &[u8], metrics: &mut ConnectionMetrics) -> Result<Bytes> {
    let start = Instant::now();
    
    // Parse message type without allocation
    let msg_type = MessageType::from_bytes(&data[..4])?;
    
    // Get handler
    let handler = self.handlers
        .get(&msg_type)
        .ok_or(Error::UnknownMessageType(msg_type))?;
    
    // Execute handler with zero-copy data
    let payload = Bytes::copy_from_slice(&data[4..]);
    let response = handler.value()(payload).await?;
    
    // Update metrics
    metrics.requests += 1;
    metrics.total_time += start.elapsed();
    self.metrics.record(msg_type, start.elapsed());
    
    Ok(response)
}
```

## Memory Optimizations

### 1. Buffer Pool Management
```rust
pub struct BufferPool {
    small: Vec<BytesMut>,  // 4KB buffers
    medium: Vec<BytesMut>, // 64KB buffers
    large: Vec<BytesMut>,  // 1MB buffers
}

impl BufferPool {
    pub fn acquire(&mut self, size: usize) -> BytesMut {
        let buffer = if size <= 4096 {
            self.small.pop().unwrap_or_else(|| BytesMut::with_capacity(4096))
        } else if size <= 65536 {
            self.medium.pop().unwrap_or_else(|| BytesMut::with_capacity(65536))
        } else {
            self.large.pop().unwrap_or_else(|| BytesMut::with_capacity(1048576))
        };
        
        buffer
    }
    
    pub fn release(&mut self, mut buffer: BytesMut) {
        buffer.clear();
        
        match buffer.capacity() {
            0..=4096 if self.small.len() < 100 => self.small.push(buffer),
            4097..=65536 if self.medium.len() < 50 => self.medium.push(buffer),
            65537..=1048576 if self.large.len() < 10 => self.large.push(buffer),
            _ => {} // Let it drop
        }
    }
}
```

### 2. Connection Pooling
```rust
pub struct ConnectionPool {
    idle: Arc<RwLock<Vec<Connection>>>,
    active: Arc<DashMap<ConnectionId, Connection>>,
    max_idle: usize,
    idle_timeout: Duration,
}

impl ConnectionPool {
    pub async fn acquire(&self) -> Connection {
        // Try to get idle connection
        if let Some(conn) = self.idle.write().pop() {
            if !conn.is_expired() {
                return conn;
            }
        }
        
        // Create new connection
        Connection::new()
    }
    
    pub fn release(&self, conn: Connection) {
        let mut idle = self.idle.write();
        if idle.len() < self.max_idle {
            idle.push(conn);
        }
    }
}
```

## Performance Metrics

### Benchmarking Setup
```rust
#[bench]
fn bench_message_processing(b: &mut Bencher) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let server = runtime.block_on(IpcServer::new("/tmp/bench.sock")).unwrap();
    
    // Register test handler
    server.register_handler(MessageType::Echo, |data| async move {
        Ok(data)
    });
    
    let test_message = vec![0u8; 1024];
    
    b.iter(|| {
        runtime.block_on(async {
            server.process_message(&test_message, &mut ConnectionMetrics::default()).await
        })
    });
}
```

### Expected Performance
- **Latency**: < 10μs per message
- **Throughput**: 1M+ messages/second
- **Memory**: 2-3MB total footprint
- **CPU**: < 1% idle, < 10% under load

## Error Handling

### Graceful Error Recovery
```rust
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Message too large: {0} bytes")]
    MessageTooLarge(usize),
    
    #[error("Unknown message type: {0:?}")]
    UnknownMessageType(MessageType),
    
    #[error("Handler panic")]
    HandlerPanic,
    
    #[error("Connection timeout")]
    Timeout,
}

impl IpcServer {
    async fn handle_error(&self, error: IpcError, conn_id: ConnectionId) {
        match error {
            IpcError::Io(e) if e.kind() == ErrorKind::UnexpectedEof => {
                // Client disconnected cleanly
                self.connections.remove(conn_id);
            }
            IpcError::MessageTooLarge(_) => {
                // Log and close connection
                tracing::warn!("Message too large from {:?}", conn_id);
                self.connections.close(conn_id);
            }
            IpcError::HandlerPanic => {
                // Restart handler, continue connection
                tracing::error!("Handler panic, recovering");
                self.recover_handler().await;
            }
            _ => {
                tracing::error!("IPC error: {}", error);
            }
        }
    }
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_server_creation() {
        let server = IpcServer::new("/tmp/test.sock").await.unwrap();
        assert!(Path::new("/tmp/test.sock").exists());
    }
    
    #[tokio::test]
    async fn test_handler_registration() {
        let server = IpcServer::new("/tmp/test2.sock").await.unwrap();
        
        server.register_handler(MessageType::Echo, |data| async move {
            Ok(data)
        });
        
        assert!(server.handlers.contains_key(&MessageType::Echo));
    }
    
    #[tokio::test]
    async fn test_concurrent_connections() {
        let server = Arc::new(IpcServer::new("/tmp/test3.sock").await.unwrap());
        
        // Spawn server
        let server_handle = tokio::spawn(server.clone().serve());
        
        // Create 100 concurrent clients
        let mut handles = vec![];
        for i in 0..100 {
            let handle = tokio::spawn(async move {
                let mut stream = Shared Memory Stream::connect("/tmp/test3.sock").await.unwrap();
                // Send/receive messages
            });
            handles.push(handle);
        }
        
        // Wait for all clients
        for handle in handles {
            handle.await.unwrap();
        }
    }
}
```

## Production Deployment

### Configuration
```toml
[ipc]
socket_path = "/tmp/lapce-ai.sock"
max_connections = 1000
idle_timeout_secs = 300
max_message_size = 10485760  # 10MB
buffer_pool_size = 100

[metrics]
enable = true
export_interval_secs = 60
```

### Monitoring
```rust
impl Metrics {
    pub fn export_prometheus(&self) -> String {
        format!(
            "# HELP ipc_requests_total Total IPC requests\n\
             # TYPE ipc_requests_total counter\n\
             ipc_requests_total {}\n\
             # HELP ipc_latency_seconds IPC request latency\n\
             # TYPE ipc_latency_seconds histogram\n\
             ipc_latency_seconds_bucket{{le=\"0.001\"}} {}\n\
             ipc_latency_seconds_bucket{{le=\"0.01\"}} {}\n\
             ipc_latency_seconds_bucket{{le=\"0.1\"}} {}\n",
            self.total_requests.load(Ordering::Relaxed),
            self.latency_buckets[0].load(Ordering::Relaxed),
            self.latency_buckets[1].load(Ordering::Relaxed),
            self.latency_buckets[2].load(Ordering::Relaxed),
        )
    }
}
```

## Integration Points

### With Binary Protocol (Step 2)
```rust
pub trait Codec {
    fn encode(&self, msg: &Message) -> Bytes;
    fn decode(&self, data: &[u8]) -> Result<Message>;
}

impl IpcServer {
    pub fn with_codec<C: Codec>(mut self, codec: C) -> Self {
        self.codec = Box::new(codec);
        self
    }
}
```

### With Provider Pool (Step 3)
```rust
impl IpcServer {
    pub fn register_provider_handlers(&self, provider_pool: Arc<ProviderPool>) {
        self.register_handler(MessageType::Complete, move |data| {
            let pool = provider_pool.clone();
            async move {
                let request: CompletionRequest = deserialize(&data)?;
                let response = pool.complete(request).await?;
                Ok(serialize(&response))
            }
        });
    }
}
```

## Memory Profile
- **Static allocation**: 500KB
- **Per connection**: 8KB
- **Buffer pools**: 1MB pre-allocated
- **Total at 100 connections**: ~2.3MB



**Here's the FINAL Ultimate Stress Test - shorter but more intense** 🔥 [1]

## **The Final Nuclear Stress Test Protocol**

### **Level 1: Connection Bomb (5 minutes)**
```rust
#[tokio::test]
async fn connection_bomb_test() {
    let server = Arc::new(IpcServer::new("/tmp/stress.sock").await.unwrap());
    
    // 1000 connections simultaneously for 5 minutes
    let handles = (0..1000).map(|_| {
        tokio::spawn(async {
            let mut stream = connect("/tmp/stress.sock").await.unwrap();
            
            // Each sends 5000 messages (1000 msg/sec for 5 min)
            for _ in 0..5000 {
                let msg = create_message(1024); // 1KB each
                stream.write_all(&msg).await.unwrap();
                let response = read_response(&mut stream).await.unwrap();
                tokio::time::sleep(Duration::from_micros(1)).await;
            }
        })
    }).collect::<Vec<_>>();
    
    futures::future::join_all(handles).await;
    
    // MUST handle 1M+ messages/second [file:271]
    assert!(total_throughput >= 1_000_000);
}
```

### **Level 2: Memory Exhaustion + Buffer Pool Destruction**
```rust
#[tokio::test]
async fn memory_destruction_test() {
    let server = Arc::new(IpcServer::new("/tmp/memory.sock").await.unwrap());
    
    // Try to exhaust ALL buffer sizes simultaneously
    let small_buffer_spam = (0..500).map(|_| {
        tokio::spawn(async {
            // Exhaust all small buffers (4KB each)
            for _ in 0..1000 {
                let msg = vec![0u8; 4096];
                send_message(&msg).await.unwrap();
            }
        })
    });
    
    let large_buffer_spam = (0..100).map(|_| {
        tokio::spawn(async {
            // Exhaust all large buffers (1MB each)  
            for _ in 0..500 {
                let msg = vec![0u8; 1048576];
                send_message(&msg).await.unwrap();
            }
        })
    });
    
    let all_handles = small_buffer_spam.chain(large_buffer_spam).collect::<Vec<_>>();
    futures::future::join_all(all_handles).await;
    
    // SYSTEM MUST STAY UNDER 3MB [file:271]
    assert!(get_memory_usage() < 3 * 1024 * 1024);
}
```

### **Level 3: Latency Torture Under Maximum Load (10 minutes)**
```rust
#[tokio::test]
async fn latency_torture_test() {
    let server = Arc::new(IpcServer::new("/tmp/latency.sock").await.unwrap());
    
    // Background: 999 connections hammering server at max capacity
    let background_load = (0..999).map(|_| {
        tokio::spawn(async {
            for _ in 0..60000 { // 10 minutes of max load
                let msg = create_message(4096);
                send_message(&msg).await.unwrap();
                // No sleep = maximum possible load
            }
        })
    }).collect::<Vec<_>>();
    
    // Test connection: Measure latency during chaos
    let mut max_latency = Duration::ZERO;
    let mut latency_violations = 0;
    
    for i in 0..10000 {
        let start = Instant::now();
        
        let msg = create_message(1024);
        send_message(&msg).await.unwrap();
        let _response = read_response().await.unwrap();
        
        let latency = start.elapsed();
        max_latency = max_latency.max(latency);
        
        // Count violations but don't fail immediately
        if latency >= Duration::from_micros(10) {
            latency_violations += 1;
            println!("Latency violation #{}: {}μs at message {}", 
                    latency_violations, latency.as_micros(), i);
        }
    }
    
    // Cancel background load
    for handle in background_load {
        handle.abort();
    }
    
    // HARD REQUIREMENT: <1% latency violations [file:271]
    assert!(latency_violations < 100, 
           "Too many latency violations: {}/10000", latency_violations);
    assert!(max_latency < Duration::from_micros(50),
           "Maximum latency too high: {}μs", max_latency.as_micros());
}
```

### **Level 4: Rapid Memory Leak Detection (2 hours compressed)**
```rust
#[tokio::test]
async fn memory_leak_detection() {
    let server = Arc::new(IpcServer::new("/tmp/leak.sock").await.unwrap());
    
    let start_memory = get_memory_usage();
    let mut memory_samples = vec![];
    
    // Simulate 2 hours of intensive usage in accelerated time
    for cycle in 0..120 { // 120 cycles = 2 hours worth
        let connections = rand::thread_rng().gen_range(100..500);
        
        let handles = (0..connections).map(|_| {
            tokio::spawn(async {
                // Intensive usage pattern
                for _ in 0..100 {
                    send_autocomplete_request().await;
                    send_ai_chat_request().await;
                    send_file_analysis_request().await;
                }
            })
        }).collect::<Vec<_>>();
        
        futures::future::join_all(handles).await;
        
        let current_memory = get_memory_usage();
        memory_samples.push(current_memory);
        
        // CRITICAL: No memory growth trend [file:271]
        assert!(current_memory < start_memory + 512 * 1024, 
               "Memory leak detected: {}KB growth in cycle {}", 
               (current_memory - start_memory) / 1024, cycle);
        
        if cycle % 10 == 0 {
            println!("Cycle {}: Memory = {}MB", cycle, current_memory / 1024 / 1024);
        }
    }
    
    // Final memory must be close to start
    let final_memory = get_memory_usage();
    assert!(final_memory < start_memory + 256 * 1024,
           "Accumulated memory leak: {}KB", (final_memory - start_memory) / 1024);
}
```

### **Level 5: Chaos Engineering - The Final Boss (30 minutes)**
```rust
#[tokio::test]
async fn chaos_final_boss() {
    let server = Arc::new(IpcServer::new("/tmp/chaos.sock").await.unwrap());
    
    let chaos_handle = tokio::spawn(async {
        for _ in 0..1800 { // 30 minutes of chaos
            match rand::thread_rng().gen_range(0..6) {
                0 => kill_random_connections(10).await,
                1 => send_corrupted_messages(50).await,
                2 => simulate_network_timeouts(20).await,
                3 => send_oversized_messages(30).await,
                4 => simulate_memory_pressure().await,
                5 => flood_with_tiny_messages(1000).await,
                _ => {}
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    
    let mut recovery_failures = 0;
    
    // Normal operations during chaos
    for i in 0..18000 { // 30 minutes worth
        let start = Instant::now();
        let result = send_normal_message().await;
        
        match result {
            Ok(_) => {
                // Success - verify latency still good
                let latency = start.elapsed();
                assert!(latency < Duration::from_micros(50), 
                       "Latency degraded during chaos: {}μs", latency.as_micros());
            },
            Err(_) => {
                // Failure - test recovery within 100ms [file:271]
                tokio::time::sleep(Duration::from_millis(100)).await;
                let recovery = send_normal_message().await;
                
                if recovery.is_err() {
                    recovery_failures += 1;
                    println!("Recovery failure #{} at message {}", recovery_failures, i);
                }
            }
        }
        
        if i % 1000 == 0 {
            println!("Chaos test: {}/18000 messages processed", i);
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    chaos_handle.abort();
    
    // MUST have <1% recovery failures [file:271]
    assert!(recovery_failures < 180, 
           "Too many recovery failures: {}/18000", recovery_failures);
}
```

## **Final Success Criteria**

**ALL tests must pass:**
- ✅ **Connection Bomb**: Handle 1000 concurrent connections [1]
- ✅ **Memory Destruction**: Stay under 3MB always [1]
- ✅ **Latency Torture**: <10μs in 99%+ of messages [1]
- ✅ **Memory Leak**: No growth over time [1]
- ✅ **Chaos Recovery**: <1% failure rate, 100ms recovery [1]

**Total test time: ~3 hours instead of 24+ hours** 

**If this passes, your IPC system is bulletproof** 🚀