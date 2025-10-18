# Phase 1: Core High-Impact Components (4 weeks)
## Targeting 70% Memory Reduction

## 🎯 STRICT SUCCESS CRITERIA - MUST ACHIEVE ALL
- [ ] **Memory Baseline**: < 50MB total runtime (vs current 500MB+)
- [ ] **IPC Latency**: < 10μs per message (10x faster than Node.js)
- [ ] **Binary Protocol**: > 500K messages/second throughput
- [ ] **Zero Allocations**: No heap allocations in message hot path
- [ ] **Connection Pool**: 95% hit rate with < 1ms acquisition
- [ ] **Crash Recovery**: < 100ms reconnection after failure
- [ ] **Test Coverage**: > 95% with fuzzing on protocol
- [ ] **Benchmark**: Outperform current system by 5x minimum

**GATE**: Phase 2 starts ONLY when all criteria pass automated tests.

## ⚠️CRITICAL RULES THAT MUST BE FOLLOWED : THIS IS A 1:1 TRANSLATION - NOT A REWRITE
**YEARS OF BATTLE-TESTED AI LOGIC - PRESERVE EVERYTHING**

**BEFORE ANY IMPLEMENTATION**:
1. Study EVERY file in `/home/verma/lapce/Codex`
2. This is a TRANSLATION job: TypeScript → Rust
3. DO NOT change algorithms, logic, flow, or decisions
4. DO NOT "optimize" or "improve" - just translate
5. Keep same function names (snake_case), same parameters, same returns

**WHAT CHANGES**: Only the programming language
**WHAT STAYS**: Everything else - 100% identical AI behavior

**CRITICAL BEHAVIORS TO PRESERVE**:
- System prompt assembly order and format
- Tool XML schemas (character-for-character)
- Message role formats (system/user/assistant)
- Streaming chunk formats per provider
- Error message strings (exact match)

### Week 1-2: IPC Server & Binary Protocol #Used SharedMemory bcz Unix has fundamental hardware limit

**Current Issue:** JSON serialization uses 30-40MB RAM, 15% CPU during streaming
**Rust Solution:** Zero-copy binary protocol with `rkyv` + `tokio`

```rust
// Core IPC server with minimal allocations
pub struct IpcServer {
    socket: UnixListener,
    codec: BinaryCodec, // Custom zero-copy codec
    handlers: Arc<DashMap<Method, Handler>>,
}

impl IpcServer {
    pub async fn handle_connection(&self, stream: UnixStream) {
        let framed = Framed::new(stream, self.codec.clone());
        // Process without allocating - direct dispatch
    }
}
```

**Memory Savings:** 35MB → 3MB
**Implementation:**
- `tokio::net::UnixListener` for IPC
- `rkyv` for zero-copy serialization (10x faster than bincode)
- `bytes::Bytes` for buffer management
- Lock-free `dashmap` for handler registry

### Week 2-3: Multi-Provider AI Client Core
**Current Issue:** Each provider client holds 15-20MB in buffers and connection state
**Rust Solution:** Unified connection pool with streaming

```rust
pub struct AiProviderPool {
    connections: bb8::Pool<HttpsConnection>,
    request_queue: crossbeam::channel::Sender<Request>,
    stream_processor: StreamProcessor,
}

// Single allocation for all providers
static PROVIDER_POOL: OnceCell<AiProviderPool> = OnceCell::new();
```

**Memory Savings:** 60MB → 8MB
**Implementation:**
- `reqwest` with connection pooling
- `bb8` for async connection management
- `crossbeam-channel` for lock-free queuing
- Streaming with `futures::Stream`

### Week 3-4: Tree-sitter Native Integration
**Current Issue:** 38 WASM modules using 50MB+ RAM
**Rust Solution:** Native tree-sitter-rust bindings

```rust
use tree_sitter::{Parser, Language};
use tree_sitter_rust::language as rust_language;

pub struct NativeParser {
    parsers: HashMap<FileType, Parser>,
    // Share parsers across files
    shared_tree_cache: Arc<RwLock<LruCache<PathBuf, Tree>>>,
}
```

**Memory Savings:** 50MB → 5MB
**Implementation:**
- Direct FFI bindings to tree-sitter C library
- Shared parser instances
- LRU cache for parsed trees
- Incremental parsing support

## Critical Performance Optimizations

### 1. Memory-Mapped File I/O
```rust
use memmap2::MmapOptions;

pub struct FileReader {
    mmap_cache: DashMap<PathBuf, Mmap>,
}

// Zero-copy file reading
impl FileReader {
    pub fn read_file(&self, path: &Path) -> Result<&[u8]> {
        self.mmap_cache.entry(path.to_owned())
            .or_try_insert_with(|| {
                let file = File::open(path)?;
                unsafe { MmapOptions::new().map(&file) }
            })
            .map(|mmap| &mmap[..])
    }
}
```

### 2. Arena Allocation for Requests
```rust
use bumpalo::Bump;

thread_local! {
    static REQUEST_ARENA: RefCell<Bump> = RefCell::new(Bump::new());
}

// Reset arena after each request
pub fn process_request<'a>(data: &'a [u8]) -> Response<'a> {
    REQUEST_ARENA.with(|arena| {
        let arena = arena.borrow();
        // All allocations in this request use arena
        let request: &Request = arena.alloc(parse_request(data));
        handle_request(request, &arena)
    })
}
```

### 3. String Interning for Repeated Tokens
```rust
use lasso::{Rodeo, Spur};

pub struct TokenInterner {
    rodeo: RwLock<Rodeo>,
}

impl TokenInterner {
    pub fn intern(&self, s: &str) -> Spur {
        let rodeo = self.rodeo.read().unwrap();
        if let Some(key) = rodeo.get(s) {
            return key;
        }
        drop(rodeo);
        
        self.rodeo.write().unwrap().get_or_intern(s)
    }
}
```

## Dependencies to Add
```toml
[dependencies]
# Core async runtime
tokio = { version = "1.40", features = ["full"] }
tokio-util = "0.7"

# Zero-copy serialization
rkyv = "0.7"
bytes = "1.7"

# HTTP & networking
reqwest = { version = "0.12", features = ["stream", "rustls-tls"] }
bb8 = "0.8"

# Data structures
dashmap = "6.0"
crossbeam-channel = "0.5"
lasso = "0.7"  # String interning

# Memory optimization
memmap2 = "0.9"
bumpalo = "3.16"

# Tree-sitter
tree-sitter = "0.23"
tree-sitter-rust = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-python = "0.23"
```

## Expected Results - Phase 1
- **Memory:** 135MB → 40MB (70% reduction)
- **Cold Start:** 500ms → 50ms (90% reduction)
- **Request Latency:** 50% reduction
- **CPU Usage:** 60% reduction during idle

## Migration Strategy
1. Run Rust IPC server alongside Node.js
2. Gradually route requests to Rust server
3. Monitor performance metrics
4. Switch providers one by one
5. Deprecate Node.js components
