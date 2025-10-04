/// Memory Profiling & Optimization - Day 42 AM
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK_MEMORY: AtomicUsize = AtomicUsize::new(0);

pub struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let ptr = System.alloc(layout);
        
        if !ptr.is_null() {
            let prev = ALLOCATED.fetch_add(size, Ordering::SeqCst);
            let current = prev + size;
            let peak = PEAK_MEMORY.load(Ordering::SeqCst);
            if current > peak {
                PEAK_MEMORY.store(current, Ordering::SeqCst);
            }
        }
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
    }
}

pub struct MemoryProfiler {
    snapshots: Arc<RwLock<Vec<MemorySnapshot>>>,
    allocation_sites: Arc<RwLock<HashMap<String, AllocationSite>>>,
}

#[derive(Clone, Debug)]
pub struct MemorySnapshot {
    pub timestamp: std::time::Instant,
    pub allocated_bytes: usize,
    pub deallocated_bytes: usize,
    pub live_bytes: usize,
    pub peak_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct AllocationSite {
    pub location: String,
    pub count: usize,
    pub total_size: usize,
    pub avg_size: usize,
}

impl MemoryProfiler {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(Vec::new())),
            allocation_sites: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn take_snapshot(&self) {
        let allocated = ALLOCATED.load(Ordering::SeqCst);
        let deallocated = DEALLOCATED.load(Ordering::SeqCst);
        let peak = PEAK_MEMORY.load(Ordering::SeqCst);
        
        let snapshot = MemorySnapshot {
            timestamp: std::time::Instant::now(),
            allocated_bytes: allocated,
            deallocated_bytes: deallocated,
            live_bytes: allocated.saturating_sub(deallocated),
            peak_bytes: peak,
        };
        
        self.snapshots.write().await.push(snapshot);
    }
    
    pub async fn analyze_heap(&self) -> HeapAnalysis {
        let snapshots = self.snapshots.read().await;
        
        if snapshots.is_empty() {
            return HeapAnalysis::default();
        }
        
        let latest = snapshots.last().unwrap();
        let growth_rate = if snapshots.len() >= 2 {
            let prev = &snapshots[snapshots.len() - 2];
            let time_diff = latest.timestamp.duration_since(prev.timestamp).as_secs_f64();
            if time_diff > 0.0 {
                (latest.live_bytes as f64 - prev.live_bytes as f64) / time_diff
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        HeapAnalysis {
            current_usage: latest.live_bytes,
            peak_usage: latest.peak_bytes,
            fragmentation_ratio: calculate_fragmentation(latest.allocated_bytes, latest.live_bytes),
            growth_rate_bytes_per_sec: growth_rate,
            potential_leaks: detect_leaks(&snapshots),
        }
    }
    
    pub fn track_allocation(&self, location: String, size: usize) {
        let rt = tokio::runtime::Handle::current();
        let sites = self.allocation_sites.clone();
        
        rt.spawn(async move {
            let mut sites = sites.write().await;
            let site = sites.entry(location).or_insert(AllocationSite {
                location: String::new(),
                count: 0,
                total_size: 0,
                avg_size: 0,
            });
            
            site.count += 1;
            site.total_size += size;
            site.avg_size = site.total_size / site.count;
        });
    }
    
    pub async fn hot_allocations(&self, limit: usize) -> Vec<AllocationSite> {
        let sites = self.allocation_sites.read().await;
        let mut sorted: Vec<_> = sites.values().cloned().collect();
        sorted.sort_by_key(|s| std::cmp::Reverse(s.total_size));
        sorted.truncate(limit);
        sorted
    }
}

#[derive(Debug, Default)]
pub struct HeapAnalysis {
    pub current_usage: usize,
    pub peak_usage: usize,
    pub fragmentation_ratio: f64,
    pub growth_rate_bytes_per_sec: f64,
    pub potential_leaks: Vec<PotentialLeak>,
}

#[derive(Debug, Clone)]
pub struct PotentialLeak {
    pub location: String,
    pub size_bytes: usize,
    pub growth_pattern: String,
}

fn calculate_fragmentation(allocated: usize, live: usize) -> f64 {
    if allocated == 0 {
        return 0.0;
    }
    1.0 - (live as f64 / allocated as f64)
}

fn detect_leaks(snapshots: &[MemorySnapshot]) -> Vec<PotentialLeak> {
    let mut leaks = Vec::new();
    
    if snapshots.len() < 10 {
        return leaks;
    }
    
    // Simple leak detection: consistent growth over time
    let mut consistent_growth = true;
    for window in snapshots.windows(2) {
        if window[1].live_bytes <= window[0].live_bytes {
            consistent_growth = false;
            break;
        }
    }
    
    if consistent_growth {
        let growth = snapshots.last().unwrap().live_bytes - snapshots.first().unwrap().live_bytes;
        leaks.push(PotentialLeak {
            location: "Unknown".to_string(),
            size_bytes: growth,
            growth_pattern: "Linear".to_string(),
        });
    }
    
    leaks
}

pub async fn optimize_memory_usage() -> MemoryOptimizationReport {
    // Force garbage collection-like cleanup
    let _cleanup = Vec::<u8>::with_capacity(0);
    
    // Analyze current usage
    let allocated = ALLOCATED.load(Ordering::SeqCst);
    let deallocated = DEALLOCATED.load(Ordering::SeqCst);
    let live = allocated.saturating_sub(deallocated);
    
    MemoryOptimizationReport {
        before_bytes: live,
        after_bytes: live, // Would be less after real optimization
        saved_bytes: 0,
        optimization_suggestions: vec![
            "Use Arc for shared immutable data".to_string(),
            "Replace Vec with SmallVec for small collections".to_string(),
            "Use string interning for repeated strings".to_string(),
            "Enable jemalloc for better memory management".to_string(),
        ],
    }
}

#[derive(Debug)]
pub struct MemoryOptimizationReport {
    pub before_bytes: usize,
    pub after_bytes: usize,
    pub saved_bytes: usize,
    pub optimization_suggestions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_memory_profiler() {
        let profiler = MemoryProfiler::new();
        
        // Take initial snapshot
        profiler.take_snapshot().await;
        
        // Allocate some memory
        let _data = vec![0u8; 1024 * 1024];
        
        // Take another snapshot
        profiler.take_snapshot().await;
        
        // Analyze
        let analysis = profiler.analyze_heap().await;
        assert!(analysis.current_usage > 0);
    }
}
