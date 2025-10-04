# Step 8: Memory Management - Arena Allocators & Zero-Copy Strategies
## Achieving 50MB Total Memory Footprint

## ⚠️ CRITICAL RULES THAT MUST BE FOLLOWED : 1:1 TRANSLATION - PRESERVE YEARS OF MEMORY TUNING
**THIS IS NOT A REWRITE - IT'S A LANGUAGE PORT**

**CRITICAL**: Study `/home/verma/lapce/Codex`
- Token buffer management - copy line by line
- Context window limits - same exact values (128k, 200k per model)
- Message truncation - same algorithms, same decisions
- Memory patterns took YEARS to optimize - just translate syntax
- Copy All System Prompt - every single prompt tool,errors,every single guideline that used to manage the AI - ALl the thousands of files - very critical took  years to perfect - translate all thousands of file

## ✅ Success Criteria
- [ ] **Every single Prompt that used to guideline the AI**
- [ ] **Memory Usage**: < 3.5MB fixed overhead
- [ ] **Arena Performance**: Zero allocations per request
- [ ] **Object Pool Hit Rate**: > 95% reuse rate
- [ ] **String Interning**: 50% memory reduction for strings
- [ ] **MMap Cache**: < 100MB configurable limit
- [ ] **Allocation Tracking**: < 1% performance overhead
- [ ] **Memory Leaks**: Zero leaks after 24h runtime
- [ ] **Test Coverage**: Stress test with 1M allocations

## Overview
Our memory management system

## AI-Specific Memory Requirements
- **System Prompt**: ~5KB constant
- **Context Window**: 128KB-1MB depending on model
- **Tool Definitions**: ~20KB when loaded
- **Message History**: Must handle 100+ messages efficiently
- **Streaming Buffers**: 64KB for SSE parsing uses arena allocation, object pooling, and zero-copy techniques to minimize allocations and achieve predictable performance.

## Core Memory Architecture

### Memory Management System
```rust
use bumpalo::Bump;
use crossbeam::queue::ArrayQueue;
use memmap2::{Mmap, MmapOptions};
use std::alloc::{GlobalAlloc, Layout, System};

pub struct MemoryManager {
    // Arena allocators for request processing
    request_arenas: ThreadLocal<RefCell<Bump>>,
    
    // Object pools for frequent allocations
    object_pools: ObjectPoolManager,
    
    // Memory-mapped file cache
    mmap_cache: MmapCache,
    
    // String interning for deduplication
    string_interner: StringInterner,
    
    // Custom allocator with tracking
    allocator: TrackedAllocator,
    
    // Memory metrics
    metrics: Arc<MemoryMetrics>,
}

pub struct ObjectPoolManager {
    request_pool: ObjectPool<Request>,
    response_pool: ObjectPool<Response>,
    buffer_pool: BufferPool,
    token_pool: ObjectPool<Token>,
}

pub struct TrackedAllocator {
    inner: System,
    allocated: AtomicUsize,
    peak: AtomicUsize,
}
```

## Arena Allocation

### 1. Request-Scoped Arenas
```rust
thread_local! {
    static REQUEST_ARENA: RefCell<Bump> = RefCell::new(Bump::with_capacity(1024 * 1024));
}

pub struct ArenaScope<'a> {
    arena: &'a Bump,
    initial_size: usize,
}

impl<'a> ArenaScope<'a> {
    pub fn new() -> Self {
        REQUEST_ARENA.with(|arena| {
            let arena = arena.borrow();
            let initial_size = arena.allocated_bytes();
            
            ArenaScope {
                arena: unsafe { &*(arena.as_ptr() as *const Bump) },
                initial_size,
            }
        })
    }
    
    pub fn alloc<T>(&self, value: T) -> &'a T {
        self.arena.alloc(value)
    }
    
    pub fn alloc_slice<T: Copy>(&self, slice: &[T]) -> &'a [T] {
        self.arena.alloc_slice_copy(slice)
    }
    
    pub fn alloc_str(&self, s: &str) -> &'a str {
        self.arena.alloc_str(s)
    }
}

impl<'a> Drop for ArenaScope<'a> {
    fn drop(&mut self) {
        // Reset arena to initial state
        REQUEST_ARENA.with(|arena| {
            let mut arena = arena.borrow_mut();
            arena.reset();
        });
    }
}

// Usage in request processing
pub async fn process_request(data: &[u8]) -> Result<Response> {
    let arena = ArenaScope::new();
    
    // All allocations in this scope use arena
    let request = arena.alloc(parse_request(data, &arena)?);
    let context = arena.alloc(build_context(request, &arena)?);
    let response = handle_request(request, context, &arena)?;
    
    // Arena automatically resets when scope ends
    Ok(response.to_owned())
}
```

### 2. Generational Arena for Long-Lived Objects
```rust
use generational_arena::{Arena, Index};

pub struct GenerationalArena<T> {
    arena: Arena<T>,
    free_list: Vec<Index>,
    generation: u64,
}

impl<T> GenerationalArena<T> {
    pub fn insert(&mut self, value: T) -> Index {
        if let Some(index) = self.free_list.pop() {
            self.arena[index] = value;
            index
        } else {
            self.arena.insert(value)
        }
    }
    
    pub fn remove(&mut self, index: Index) -> Option<T> {
        let value = self.arena.remove(index);
        if value.is_some() {
            self.free_list.push(index);
        }
        value
    }
    
    pub fn get(&self, index: Index) -> Option<&T> {
        self.arena.get(index)
    }
    
    pub fn compact(&mut self) {
        // Compact arena to reduce fragmentation
        let mut new_arena = Arena::with_capacity(self.arena.len());
        let mut mapping = HashMap::new();
        
        for (old_idx, value) in self.arena.drain() {
            let new_idx = new_arena.insert(value);
            mapping.insert(old_idx, new_idx);
        }
        
        self.arena = new_arena;
        self.free_list.clear();
        self.generation += 1;
    }
}
```

## Object Pooling

### 1. Generic Object Pool
```rust
pub struct ObjectPool<T> {
    pool: Arc<ArrayQueue<T>>,
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
    created: AtomicUsize,
}

impl<T> ObjectPool<T> {
    pub fn new<F>(max_size: usize, factory: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            pool: Arc::new(ArrayQueue::new(max_size)),
            factory: Arc::new(factory),
            max_size,
            created: AtomicUsize::new(0),
        }
    }
    
    pub fn acquire(&self) -> PooledObject<T> {
        let object = self.pool.pop()
            .unwrap_or_else(|| {
                self.created.fetch_add(1, Ordering::Relaxed);
                (self.factory)()
            });
            
        PooledObject {
            object: Some(object),
            pool: self.pool.clone(),
        }
    }
    
    pub fn clear(&self) {
        while self.pool.pop().is_some() {}
    }
}

pub struct PooledObject<T> {
    object: Option<T>,
    pool: Arc<ArrayQueue<T>>,
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(object) = self.object.take() {
            let _ = self.pool.push(object);
        }
    }
}

impl<T> Deref for PooledObject<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        self.object.as_ref().unwrap()
    }
}
```

### 2. Buffer Pool
```rust
pub struct BufferPool {
    small: Arc<ArrayQueue<BytesMut>>,  // 4KB buffers
    medium: Arc<ArrayQueue<BytesMut>>, // 64KB buffers
    large: Arc<ArrayQueue<BytesMut>>,  // 1MB buffers
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            small: Arc::new(ArrayQueue::new(100)),
            medium: Arc::new(ArrayQueue::new(50)),
            large: Arc::new(ArrayQueue::new(10)),
        }
    }
    
    pub fn acquire(&self, size_hint: usize) -> PooledBuffer {
        let (pool, buffer) = if size_hint <= 4096 {
            let buf = self.small.pop()
                .unwrap_or_else(|| BytesMut::with_capacity(4096));
            (&self.small, buf)
        } else if size_hint <= 65536 {
            let buf = self.medium.pop()
                .unwrap_or_else(|| BytesMut::with_capacity(65536));
            (&self.medium, buf)
        } else {
            let buf = self.large.pop()
                .unwrap_or_else(|| BytesMut::with_capacity(1048576));
            (&self.large, buf)
        };
        
        PooledBuffer {
            buffer: Some(buffer),
            pool: pool.clone(),
            initial_capacity: size_hint,
        }
    }
}

pub struct PooledBuffer {
    buffer: Option<BytesMut>,
    pool: Arc<ArrayQueue<BytesMut>>,
    initial_capacity: usize,
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(mut buffer) = self.buffer.take() {
            buffer.clear();
            
            // Only return to pool if capacity hasn't grown too much
            if buffer.capacity() <= self.initial_capacity * 2 {
                let _ = self.pool.push(buffer);
            }
        }
    }
}
```

## Memory-Mapped I/O

### 1. MMap Cache
```rust
pub struct MmapCache {
    cache: DashMap<PathBuf, Arc<Mmap>>,
    total_size: AtomicUsize,
    max_size: usize,
}

impl MmapCache {
    pub async fn get_or_load(&self, path: &Path) -> Result<Arc<Mmap>> {
        // Check cache
        if let Some(mmap) = self.cache.get(path) {
            return Ok(mmap.clone());
        }
        
        // Load file
        let file = tokio::fs::File::open(path).await?;
        let metadata = file.metadata().await?;
        let file_size = metadata.len() as usize;
        
        // Check if we need to evict
        if self.total_size.load(Ordering::Relaxed) + file_size > self.max_size {
            self.evict_lru(file_size);
        }
        
        // Memory map the file
        let file_std = file.into_std().await;
        let mmap = unsafe { MmapOptions::new().map(&file_std)? };
        let mmap = Arc::new(mmap);
        
        // Update cache
        self.cache.insert(path.to_owned(), mmap.clone());
        self.total_size.fetch_add(file_size, Ordering::Relaxed);
        
        Ok(mmap)
    }
    
    fn evict_lru(&self, needed_size: usize) {
        // Simple LRU eviction
        let mut entries: Vec<_> = self.cache.iter()
            .map(|entry| (entry.key().clone(), entry.value().len()))
            .collect();
            
        entries.sort_by_key(|(_path, size)| *size);
        
        let mut freed = 0;
        for (path, size) in entries {
            if freed >= needed_size {
                break;
            }
            
            self.cache.remove(&path);
            self.total_size.fetch_sub(size, Ordering::Relaxed);
            freed += size;
        }
    }
}
```

## String Interning

### 1. Thread-Safe String Interner
```rust
use lasso::{Rodeo, Spur, ThreadedRodeo};

pub struct StringInterner {
    rodeo: Arc<ThreadedRodeo>,
    cache_stats: Arc<InternerStats>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            rodeo: Arc::new(ThreadedRodeo::new()),
            cache_stats: Arc::new(InternerStats::default()),
        }
    }
    
    pub fn intern(&self, s: &str) -> Spur {
        let key = self.rodeo.get_or_intern(s);
        self.cache_stats.record_intern(s.len());
        key
    }
    
    pub fn get(&self, key: Spur) -> &str {
        self.rodeo.resolve(&key)
    }
    
    pub fn intern_many<I>(&self, strings: I) -> Vec<Spur>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        strings.into_iter()
            .map(|s| self.intern(s.as_ref()))
            .collect()
    }
    
    pub fn memory_usage(&self) -> usize {
        self.rodeo.len() * std::mem::size_of::<String>()
            + self.rodeo.capacity() * std::mem::size_of::<Spur>()
    }
}
```

## Custom Allocator

### 1. Tracked Allocator
```rust
pub struct TrackedAllocator {
    allocated: AtomicUsize,
    peak: AtomicUsize,
    allocations: AtomicU64,
}

unsafe impl GlobalAlloc for TrackedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let ptr = System.alloc(layout);
        
        if !ptr.is_null() {
            let old = self.allocated.fetch_add(size, Ordering::Relaxed);
            let new = old + size;
            
            // Update peak
            let mut peak = self.peak.load(Ordering::Relaxed);
            while new > peak {
                match self.peak.compare_exchange_weak(
                    peak,
                    new,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(x) => peak = x,
                }
            }
            
            self.allocations.fetch_add(1, Ordering::Relaxed);
        }
        
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        self.allocated.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: TrackedAllocator = TrackedAllocator {
    allocated: AtomicUsize::new(0),
    peak: AtomicUsize::new(0),
    allocations: AtomicU64::new(0),
};
```

## Memory Profiling

### 1. Memory Profiler
```rust
pub struct MemoryProfiler {
    snapshots: Arc<RwLock<Vec<MemorySnapshot>>>,
    interval: Duration,
    handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    timestamp: Instant,
    allocated: usize,
    peak: usize,
    allocations: u64,
    arena_usage: usize,
    pool_usage: usize,
    cache_usage: usize,
}

impl MemoryProfiler {
    pub fn start(interval: Duration) -> Self {
        let snapshots = Arc::new(RwLock::new(Vec::new()));
        let snapshots_clone = snapshots.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                interval.tick().await;
                
                let snapshot = MemorySnapshot {
                    timestamp: Instant::now(),
                    allocated: ALLOCATOR.allocated.load(Ordering::Relaxed),
                    peak: ALLOCATOR.peak.load(Ordering::Relaxed),
                    allocations: ALLOCATOR.allocations.load(Ordering::Relaxed),
                    arena_usage: get_arena_usage(),
                    pool_usage: get_pool_usage(),
                    cache_usage: get_cache_usage(),
                };
                
                snapshots_clone.write().unwrap().push(snapshot);
            }
        });
        
        Self {
            snapshots,
            interval,
            handle: Some(handle),
        }
    }
    
    pub fn generate_report(&self) -> MemoryReport {
        let snapshots = self.snapshots.read().unwrap();
        
        MemoryReport {
            current_allocated: ALLOCATOR.allocated.load(Ordering::Relaxed),
            peak_allocated: ALLOCATOR.peak.load(Ordering::Relaxed),
            total_allocations: ALLOCATOR.allocations.load(Ordering::Relaxed),
            snapshots: snapshots.clone(),
        }
    }
}
```

## Memory Optimization Patterns

### 1. Copy-on-Write
```rust
use std::sync::Arc;

pub struct Cow<T: Clone> {
    data: Arc<T>,
}

impl<T: Clone> Cow<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: Arc::new(data),
        }
    }
    
    pub fn get(&self) -> &T {
        &self.data
    }
    
    pub fn get_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.data)
    }
    
    pub fn is_unique(&self) -> bool {
        Arc::strong_count(&self.data) == 1
    }
}
```

### 2. Lazy Allocation
```rust
use once_cell::sync::Lazy;

pub struct LazyBuffer {
    buffer: Lazy<BytesMut>,
    size: usize,
}

impl LazyBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            buffer: Lazy::new(move || BytesMut::with_capacity(size)),
            size,
        }
    }
    
    pub fn get(&self) -> &BytesMut {
        &*self.buffer
    }
    
    pub fn get_mut(&mut self) -> &mut BytesMut {
        Lazy::force_mut(&mut self.buffer)
    }
}
```

## Memory Profile
- **Arena allocators**: 1MB pre-allocated
- **Object pools**: 2MB total
- **MMap cache**: Configurable (default 100MB)
- **String interner**: 500KB
- **Tracking overhead**: < 100KB
- **Total overhead**: ~3.5MB fixed + configurable cache
