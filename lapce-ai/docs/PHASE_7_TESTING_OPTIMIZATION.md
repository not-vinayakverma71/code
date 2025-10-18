# Phase 7: Testing, Benchmarking & Optimization (1.5 weeks)
## Production-Grade Quality Assurance with Zero Overhead

## 🎯 STRICT SUCCESS CRITERIA - MUST ACHIEVE ALL
- [ ] **Translation Accuracy**: 100% identical outputs vs Codex for same inputs
- [ ] **Memory Target**: < 350MB total system (vs current 3.5GB)
- [ ] **Performance**: 10x faster than current system minimum
- [ ] **Test Coverage**: > 95% code coverage with automated tests
- [ ] **AI Behavior**: CHARACTER-FOR-CHARACTER match in all responses
- [ ] **Load Testing**: Handle 1K concurrent operations
- [ ] **Memory Leaks**: Zero leaks after 24h stress test
- [ ] **Benchmark**: All success criteria automated and passing

⚠️ **GATE**: Phase 8 starts ONLY when system performs 10x better with identical AI behavior.

## ⚠️MUST  TEST THE TRANSLATION - NOT NEW CODE
**VERIFY 1:1 TYPESCRIPT → RUST PORT**

**TEST REQUIREMENTS**:
- Compare Rust output with TypeScript output CHARACTER-BY-CHARACTER
- Same inputs → Same outputs ALWAYS
- Use test cases from `/home/verma/lapce/lapce-ai-rust/codex-reference/`
- This is a TRANSLATION verification, not new feature testing
- Years of battle-tested logic - just verify the port is accurate

### Week 1: Comprehensive Testing Framework
**Goal:** 95% code coverage with minimal test overhead
**Memory Target:** < 2MB for test infrastructure

### Unit Testing Infrastructure
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    // Property-based testing for parsers
    proptest! {
        #[test]
        fn test_parser_never_panics(input in "\\PC*") {
            let parser = CodeIntelligence::new().unwrap();
            // Should never panic, even with random input
            let _ = parser.parse_content(&input);
        }
        
        #[test]
        fn test_streaming_maintains_order(
            chunks in prop::collection::vec(prop::string::string_regex("[^\\n]*\\n").unwrap(), 0..100)
        ) {
            let mut pipeline = StreamPipeline::new();
            let mut accumulated = String::new();
            
            for chunk in chunks {
                pipeline.process_chunk(chunk.as_bytes());
                accumulated.push_str(&chunk);
            }
            
            assert_eq!(pipeline.get_full_content(), accumulated);
        }
    }
    
    // Fuzzing for security-critical components
    #[test]
    fn fuzz_ipc_protocol() {
        use arbitrary::{Arbitrary, Unstructured};
        
        let data = include_bytes!("../fuzz/corpus/ipc_messages.bin");
        let mut u = Unstructured::new(data);
        
        while !u.is_empty() {
            if let Ok(msg) = IpcMessage::arbitrary(&mut u) {
                // Should handle any message without crashing
                let _ = process_message(msg);
            }
        }
    }
}
```

### Integration Testing
```rust
pub struct TestEnvironment {
    temp_dir: TempDir,
    ai_server: LapceAI,
    mock_providers: Vec<MockProvider>,
}

impl TestEnvironment {
    pub async fn setup() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        
        // Initialize with test configuration
        let config = Config {
            workspace: temp_dir.path().to_owned(),
            memory_limit: 10 * 1024 * 1024, // 10MB for tests
            providers: vec![],
        };
        
        let ai_server = LapceAI::initialize(config).await?;
        
        Ok(Self {
            temp_dir,
            ai_server,
            mock_providers: vec![],
        })
    }
    
    pub async fn test_end_to_end_completion(&mut self) -> Result<()> {
        // Create test file
        let test_file = self.temp_dir.path().join("test.rs");
        tokio::fs::write(&test_file, "fn main() {\n    println").await?;
        
        // Request completion
        let response = self.ai_server.complete(CompletionRequest {
            file: test_file.clone(),
            position: Position { line: 1, column: 12 },
            ..Default::default()
        }).await?;
        
        // Verify response
        assert!(response.completions.iter().any(|c| c.text.contains("println!")));
        
        Ok(())
    }
}

#[tokio::test]
async fn test_memory_limits() {
    let env = TestEnvironment::setup().await.unwrap();
    
    // Track initial memory
    let initial_memory = get_process_memory();
    
    // Perform heavy operations
    for _ in 0..100 {
        let _ = env.ai_server.parse_file(&PathBuf::from("large_file.rs")).await;
    }
    
    // Check memory didn't exceed limits
    let final_memory = get_process_memory();
    assert!(final_memory - initial_memory < 5 * 1024 * 1024); // Less than 5MB growth
}
```

### Performance Benchmarking
```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_ipc_throughput(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let server = runtime.block_on(IpcServer::new("/tmp/bench.sock")).unwrap();
    
    let mut group = c.benchmark_group("ipc_throughput");
    
    for size in [1, 10, 100, 1000, 10000].iter() {
        let payload = vec![0u8; *size];
        
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                b.to_async(&runtime).iter(|| async {
                    server.process_message(&payload).await
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_parsing_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");
    let parser = CodeIntelligence::new().unwrap();
    
    // Test files of different sizes
    let files = vec![
        ("small", include_str!("../fixtures/small.rs")),
        ("medium", include_str!("../fixtures/medium.rs")),
        ("large", include_str!("../fixtures/large.rs")),
    ];
    
    for (name, content) in files {
        group.bench_function(name, |b| {
            b.iter(|| {
                parser.parse_content(black_box(content))
            });
        });
    }
    
    group.finish();
}

criterion_group!(benches, benchmark_ipc_throughput, benchmark_parsing_performance);
criterion_main!(benches);
```

### Week 1.5: Memory & Performance Optimization
**Goal:** Profile and optimize hot paths
**Memory Target:** Final optimization pass

### Memory Profiling
```rust
pub struct MemoryProfiler {
    snapshots: Arc<RwLock<Vec<MemorySnapshot>>>,
    allocator_stats: Arc<AllocatorStats>,
}

impl MemoryProfiler {
    pub async fn profile_operation<F, Fut, T>(&self, name: &str, f: F) -> (T, MemoryProfile)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        // Capture initial state
        let before = self.capture_snapshot().await;
        
        // Run operation
        let start = Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        
        // Capture final state
        let after = self.capture_snapshot().await;
        
        let profile = MemoryProfile {
            operation: name.to_string(),
            duration,
            memory_allocated: after.allocated - before.allocated,
            memory_deallocated: after.deallocated - before.deallocated,
            peak_memory: after.peak_memory,
            allocations: after.allocation_count - before.allocation_count,
        };
        
        (result, profile)
    }
    
    async fn capture_snapshot(&self) -> MemorySnapshot {
        MemorySnapshot {
            allocated: self.allocator_stats.allocated.load(Ordering::Relaxed),
            deallocated: self.allocator_stats.deallocated.load(Ordering::Relaxed),
            peak_memory: self.allocator_stats.peak.load(Ordering::Relaxed),
            allocation_count: self.allocator_stats.count.load(Ordering::Relaxed),
            timestamp: Instant::now(),
        }
    }
}

// Custom allocator for tracking
#[global_allocator]
static ALLOCATOR: TrackedAllocator = TrackedAllocator;

struct TrackedAllocator;

unsafe impl GlobalAlloc for TrackedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let ptr = System.alloc(layout);
        
        if !ptr.is_null() {
            STATS.allocated.fetch_add(size, Ordering::Relaxed);
            STATS.count.fetch_add(1, Ordering::Relaxed);
            
            // Update peak
            let current = STATS.allocated.load(Ordering::Relaxed);
            let mut peak = STATS.peak.load(Ordering::Relaxed);
            while current > peak {
                match STATS.peak.compare_exchange_weak(
                    peak, current, Ordering::Relaxed, Ordering::Relaxed
                ) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }
        }
        
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        STATS.deallocated.fetch_add(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout);
    }
}
```

### Performance Optimization Techniques
```rust
pub struct PerformanceOptimizer {
    hot_path_cache: Arc<HotPathCache>,
    jit_compiler: Arc<JitCompiler>,
    simd_optimizer: Arc<SimdOptimizer>,
}

impl PerformanceOptimizer {
    pub async fn optimize_hot_paths(&self) -> Result<()> {
        // 1. Identify hot paths through profiling
        let hot_paths = self.identify_hot_paths().await?;
        
        // 2. Apply optimizations
        for path in hot_paths {
            match path.optimization_type {
                OptType::Cache => {
                    self.hot_path_cache.cache_path(&path).await?;
                }
                OptType::Simd => {
                    self.simd_optimizer.vectorize(&path).await?;
                }
                OptType::Inline => {
                    self.apply_inline_optimization(&path)?;
                }
            }
        }
        
        Ok(())
    }
    
    // SIMD optimization for text processing
    pub fn find_newlines_simd(text: &[u8]) -> Vec<usize> {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::*;
            
            unsafe {
                let mut positions = Vec::new();
                let newline = _mm256_set1_epi8(b'\n' as i8);
                
                let chunks = text.chunks_exact(32);
                let remainder = chunks.remainder();
                
                for (i, chunk) in chunks.enumerate() {
                    let data = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);
                    let cmp = _mm256_cmpeq_epi8(data, newline);
                    let mask = _mm256_movemask_epi8(cmp);
                    
                    if mask != 0 {
                        for j in 0..32 {
                            if mask & (1 << j) != 0 {
                                positions.push(i * 32 + j);
                            }
                        }
                    }
                }
                
                // Handle remainder
                for (i, &byte) in remainder.iter().enumerate() {
                    if byte == b'\n' {
                        positions.push(text.len() - remainder.len() + i);
                    }
                }
                
                positions
            }
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        {
            text.iter().enumerate()
                .filter(|(_, &b)| b == b'\n')
                .map(|(i, _)| i)
                .collect()
        }
    }
}
```

### Stress Testing
```rust
#[tokio::test]
async fn stress_test_concurrent_operations() {
    let server = LapceAI::initialize(Config::default()).await.unwrap();
    let num_tasks = 1000;
    let num_operations = 100;
    
    let tasks: Vec<_> = (0..num_tasks).map(|i| {
        let server = server.clone();
        
        tokio::spawn(async move {
            for j in 0..num_operations {
                let request = CompletionRequest {
                    messages: vec![Message {
                        role: "user",
                        content: format!("Test request {} - {}", i, j),
                    }],
                    ..Default::default()
                };
                
                let result = server.complete(request).await;
                assert!(result.is_ok());
            }
        })
    }).collect();
    
    // All tasks should complete without errors
    for task in tasks {
        task.await.unwrap();
    }
    
    // Memory should stay within bounds
    let memory = get_process_memory();
    assert!(memory < 50 * 1024 * 1024); // Less than 50MB
}

#[tokio::test]
async fn test_memory_leak_detection() {
    let server = LapceAI::initialize(Config::default()).await.unwrap();
    
    // Baseline memory
    let baseline = get_process_memory();
    
    // Perform many operations
    for _ in 0..10000 {
        let _ = server.parse_file(&PathBuf::from("test.rs")).await;
    }
    
    // Force cleanup
    drop(server);
    
    // Allow GC time
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Check for leaks
    let final_memory = get_process_memory();
    assert!(final_memory - baseline < 1024 * 1024); // Less than 1MB growth
}
```

## Load Testing
```rust
pub struct LoadTester {
    scenarios: Vec<LoadScenario>,
    metrics_collector: Arc<MetricsCollector>,
}

impl LoadTester {
    pub async fn run_load_test(&self) -> LoadTestResults {
        let mut results = LoadTestResults::default();
        
        for scenario in &self.scenarios {
            let scenario_result = self.run_scenario(scenario).await;
            results.scenarios.push(scenario_result);
        }
        
        results
    }
    
    async fn run_scenario(&self, scenario: &LoadScenario) -> ScenarioResult {
        let start = Instant::now();
        let mut errors = 0;
        let mut latencies = Vec::new();
        
        // Generate load
        let semaphore = Arc::new(Semaphore::new(scenario.concurrent_users));
        let tasks: Vec<_> = (0..scenario.total_requests).map(|_| {
            let sem = semaphore.clone();
            
            tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let req_start = Instant::now();
                
                match execute_request().await {
                    Ok(_) => {
                        latencies.push(req_start.elapsed());
                    }
                    Err(_) => {
                        errors += 1;
                    }
                }
            })
        }).collect();
        
        futures::future::join_all(tasks).await;
        
        ScenarioResult {
            duration: start.elapsed(),
            requests_per_second: scenario.total_requests as f64 / start.elapsed().as_secs_f64(),
            p50_latency: percentile(&latencies, 50.0),
            p99_latency: percentile(&latencies, 99.0),
            error_rate: errors as f64 / scenario.total_requests as f64,
        }
    }
}
```

## Dependencies
```toml
[dependencies]
# Testing
proptest = "1.5"
arbitrary = { version = "1.3", features = ["derive"] }

# Benchmarking
criterion = "0.5"

# Fuzzing
afl = "0.15"

# Profiling
pprof = { version = "0.13", features = ["flamegraph"] }

[dev-dependencies]
tempfile = "3.10"
tokio-test = "0.4"
```

## Expected Results - Phase 7
- **Test Coverage**: > 95%
- **Benchmark Suite**: 50+ benchmarks
- **Memory Leak Detection**: Zero leaks
- **Performance Regression**: < 5% tolerance
- **Load Testing**: Handle 10K concurrent operations
- **Optimization Gains**: 20-30% performance improvement
