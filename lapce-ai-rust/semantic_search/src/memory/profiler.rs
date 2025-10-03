// Memory Profiling and Tracking System
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::fmt;

/// Global memory statistics
pub struct MemoryStats {
    pub current_usage: AtomicUsize,
    pub peak_usage: AtomicUsize,
    pub total_allocated: AtomicUsize,
    pub total_freed: AtomicUsize,
    pub allocation_count: AtomicUsize,
    pub deallocation_count: AtomicUsize,
}

impl MemoryStats {
    pub const fn new() -> Self {
        Self {
            current_usage: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
            total_allocated: AtomicUsize::new(0),
            total_freed: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
            deallocation_count: AtomicUsize::new(0),
        }
    }

    pub fn record_allocation(&self, size: usize) {
        let current = self.current_usage.fetch_add(size, Ordering::SeqCst) + size;
        self.total_allocated.fetch_add(size, Ordering::Relaxed);
        self.allocation_count.fetch_add(1, Ordering::Relaxed);
        
        // Update peak if needed
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                current,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
    }

    pub fn record_deallocation(&self, size: usize) {
        self.current_usage.fetch_sub(size, Ordering::SeqCst);
        self.total_freed.fetch_add(size, Ordering::Relaxed);
        self.deallocation_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_current_mb(&self) -> f64 {
        self.current_usage.load(Ordering::Relaxed) as f64 / 1_048_576.0
    }

    pub fn get_peak_mb(&self) -> f64 {
        self.peak_usage.load(Ordering::Relaxed) as f64 / 1_048_576.0
    }
}

/// Custom allocator that tracks memory usage
pub struct TrackedAllocator;

static MEMORY_STATS: MemoryStats = MemoryStats::new();

unsafe impl GlobalAlloc for TrackedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            MEMORY_STATS.record_allocation(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        MEMORY_STATS.record_deallocation(layout.size());
        System.dealloc(ptr, layout);
    }
}

/// Allocation tracking by source
#[derive(Debug, Clone)]
pub struct AllocationSource {
    pub file: &'static str,
    pub line: u32,
    pub function: &'static str,
    pub size: usize,
    pub timestamp: Instant,
}

/// Memory profiler that tracks allocations by source
pub struct MemoryProfiler {
    allocations: Arc<RwLock<HashMap<usize, AllocationSource>>>,
    hot_paths: Arc<RwLock<HashMap<String, HotPath>>>,
    leak_candidates: Arc<RwLock<Vec<LeakCandidate>>>,
    start_time: Instant,
}

#[derive(Debug, Clone)]
pub struct HotPath {
    pub location: String,
    pub allocation_count: usize,
    pub total_size: usize,
    pub avg_size: usize,
    pub peak_size: usize,
}

#[derive(Debug, Clone)]
pub struct LeakCandidate {
    pub location: String,
    pub size: usize,
    pub age: Duration,
    pub allocation_time: Instant,
}

impl MemoryProfiler {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            hot_paths: Arc::new(RwLock::new(HashMap::new())),
            leak_candidates: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    pub fn track_allocation(&self, ptr: usize, source: AllocationSource) {
        let location = format!("{}:{} in {}", source.file, source.line, source.function);
        
        // Update hot paths
        {
            let mut hot_paths = self.hot_paths.write().unwrap();
            let hot_path = hot_paths.entry(location.clone()).or_insert(HotPath {
                location: location.clone(),
                allocation_count: 0,
                total_size: 0,
                avg_size: 0,
                peak_size: 0,
            });
            
            hot_path.allocation_count += 1;
            hot_path.total_size += source.size;
            hot_path.avg_size = hot_path.total_size / hot_path.allocation_count;
            hot_path.peak_size = hot_path.peak_size.max(source.size);
        }
        
        // Track allocation
        self.allocations.write().unwrap().insert(ptr, source);
    }

    pub fn track_deallocation(&self, ptr: usize) {
        self.allocations.write().unwrap().remove(&ptr);
    }

    pub fn detect_leaks(&self) -> Vec<LeakCandidate> {
        let mut candidates = Vec::new();
        let now = Instant::now();
        let threshold = Duration::from_secs(60); // Consider leak if alive > 60s
        
        for (_, source) in self.allocations.read().unwrap().iter() {
            let age = now - source.timestamp;
            if age > threshold && source.size > 1024 { // Only track allocations > 1KB
                candidates.push(LeakCandidate {
                    location: format!("{}:{} in {}", source.file, source.line, source.function),
                    size: source.size,
                    age,
                    allocation_time: source.timestamp,
                });
            }
        }
        
        candidates.sort_by_key(|c| std::cmp::Reverse(c.size));
        candidates
    }

    pub fn get_hot_paths(&self, top_n: usize) -> Vec<HotPath> {
        let mut paths: Vec<_> = self.hot_paths.read().unwrap().values().cloned().collect();
        paths.sort_by_key(|p| std::cmp::Reverse(p.total_size));
        paths.truncate(top_n);
        paths
    }

    pub fn get_memory_report(&self) -> MemoryReport {
        MemoryReport {
            current_usage_mb: MEMORY_STATS.get_current_mb(),
            peak_usage_mb: MEMORY_STATS.get_peak_mb(),
            total_allocated_mb: MEMORY_STATS.total_allocated.load(Ordering::Relaxed) as f64 / 1_048_576.0,
            total_freed_mb: MEMORY_STATS.total_freed.load(Ordering::Relaxed) as f64 / 1_048_576.0,
            allocation_count: MEMORY_STATS.allocation_count.load(Ordering::Relaxed),
            deallocation_count: MEMORY_STATS.deallocation_count.load(Ordering::Relaxed),
            active_allocations: self.allocations.read().unwrap().len(),
            runtime: self.start_time.elapsed(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryReport {
    pub current_usage_mb: f64,
    pub peak_usage_mb: f64,
    pub total_allocated_mb: f64,
    pub total_freed_mb: f64,
    pub allocation_count: usize,
    pub deallocation_count: usize,
    pub active_allocations: usize,
    pub runtime: Duration,
}

impl fmt::Display for MemoryReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Memory Report (Runtime: {:?})\n\
             ├─ Current Usage: {:.2} MB\n\
             ├─ Peak Usage: {:.2} MB\n\
             ├─ Total Allocated: {:.2} MB\n\
             ├─ Total Freed: {:.2} MB\n\
             ├─ Allocations: {} (active: {})\n\
             └─ Deallocations: {}",
            self.runtime,
            self.current_usage_mb,
            self.peak_usage_mb,
            self.total_allocated_mb,
            self.total_freed_mb,
            self.allocation_count,
            self.active_allocations,
            self.deallocation_count
        )
    }
}

/// Macro for tracking allocations with source location
#[macro_export]
macro_rules! track_alloc {
    ($profiler:expr, $ptr:expr, $size:expr) => {
        $profiler.track_allocation(
            $ptr as usize,
            $crate::memory::profiler::AllocationSource {
                file: file!(),
                line: line!(),
                function: module_path!(),
                size: $size,
                timestamp: std::time::Instant::now(),
            }
        )
    };
}

/// Memory dashboard for monitoring
pub struct MemoryDashboard {
    profiler: Arc<MemoryProfiler>,
    update_interval: Duration,
    last_update: Instant,
}

impl MemoryDashboard {
    pub fn new(profiler: Arc<MemoryProfiler>) -> Self {
        Self {
            profiler,
            update_interval: Duration::from_secs(5),
            last_update: Instant::now(),
        }
    }

    pub fn should_update(&self) -> bool {
        self.last_update.elapsed() > self.update_interval
    }

    pub fn update(&mut self) -> DashboardData {
        self.last_update = Instant::now();
        
        let report = self.profiler.get_memory_report();
        let hot_paths = self.profiler.get_hot_paths(5);
        let leak_candidates = self.profiler.detect_leaks();
        
        DashboardData {
            report,
            hot_paths,
            leak_candidates: leak_candidates.into_iter().take(5).collect(),
            steady_state_achieved: report.current_usage_mb < 3.0,
        }
    }

    pub fn print_dashboard(&mut self) {
        if !self.should_update() {
            return;
        }
        
        let data = self.update();
        
        println!("\n╔══════════════════════════════════════════════════════════════════╗");
        println!("║                      MEMORY DASHBOARD                             ║");
        println!("╚══════════════════════════════════════════════════════════════════╝");
        
        println!("\n{}", data.report);
        
        if data.steady_state_achieved {
            println!("\n✅ STEADY STATE ACHIEVED: < 3MB");
        } else {
            println!("\n⚠️  Memory usage above 3MB target");
        }
        
        if !data.hot_paths.is_empty() {
            println!("\n🔥 Hot Allocation Paths:");
            for (i, path) in data.hot_paths.iter().enumerate() {
                println!("  {}. {} - {} allocs, {:.2} MB total",
                    i + 1,
                    path.location,
                    path.allocation_count,
                    path.total_size as f64 / 1_048_576.0
                );
            }
        }
        
        if !data.leak_candidates.is_empty() {
            println!("\n⚠️  Potential Memory Leaks:");
            for candidate in &data.leak_candidates {
                println!("  • {} - {:.2} KB, age: {:?}",
                    candidate.location,
                    candidate.size as f64 / 1024.0,
                    candidate.age
                );
            }
        }
        
        println!("\n════════════════════════════════════════════════════════════════════");
    }
}

#[derive(Debug)]
pub struct DashboardData {
    pub report: MemoryReport,
    pub hot_paths: Vec<HotPath>,
    pub leak_candidates: Vec<LeakCandidate>,
    pub steady_state_achieved: bool,
}

/// Get global memory stats
pub fn get_memory_stats() -> &'static MemoryStats {
    &MEMORY_STATS
}

/// Check if steady state target is met
pub fn is_steady_state() -> bool {
    MEMORY_STATS.get_current_mb() < 3.0
}
