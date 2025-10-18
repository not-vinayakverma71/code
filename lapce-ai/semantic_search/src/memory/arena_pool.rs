// Phase 5: Request-side arena pool for zero-allocation query processing
// Prevents memory spikes under extreme load by reusing allocations

use std::sync::Arc;
use std::cell::RefCell;
use crossbeam::queue::SegQueue;
use std::collections::VecDeque;

/// Thread-local arena for embedding vectors
/// Reuses allocations to prevent memory spikes under load
pub struct EmbeddingArena {
    // Pool of reusable Vec<f32> buffers
    buffer_pool: RefCell<VecDeque<Vec<f32>>>,
    // Pool of Arc references (cheap to clone)
    arc_pool: RefCell<Vec<Arc<[f32]>>>,
    // Maximum buffers to keep in pool
    max_pool_size: usize,
    // Statistics
    allocations_saved: RefCell<usize>,
    allocations_made: RefCell<usize>,
}

impl EmbeddingArena {
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            buffer_pool: RefCell::new(VecDeque::with_capacity(max_pool_size)),
            arc_pool: RefCell::new(Vec::with_capacity(max_pool_size)),
            max_pool_size,
            allocations_saved: RefCell::new(0),
            allocations_made: RefCell::new(0),
        }
    }
    
    /// Get a buffer from pool or allocate new one
    pub fn get_buffer(&self, size: usize) -> Vec<f32> {
        let mut pool = self.buffer_pool.borrow_mut();
        
        // Try to reuse existing buffer
        if let Some(mut buffer) = pool.pop_front() {
            buffer.clear();
            buffer.reserve(size);
            *self.allocations_saved.borrow_mut() += 1;
            buffer
        } else {
            *self.allocations_made.borrow_mut() += 1;
            Vec::with_capacity(size)
        }
    }
    
    /// Return buffer to pool for reuse
    pub fn return_buffer(&self, mut buffer: Vec<f32>) {
        let mut pool = self.buffer_pool.borrow_mut();
        
        if pool.len() < self.max_pool_size {
            buffer.clear();
            pool.push_back(buffer);
        }
        // Otherwise let it deallocate
    }
    
    /// Borrow Arc without cloning the underlying data
    pub fn borrow_arc(&self, arc: &Arc<[f32]>) -> ArcHandle {
        ArcHandle {
            arc: Arc::clone(arc), // Cheap Arc clone, not data clone
            arena: self as *const EmbeddingArena,
        }
    }
    
    /// Get arena statistics
    pub fn get_stats(&self) -> ArenaStats {
        ArenaStats {
            buffers_pooled: self.buffer_pool.borrow().len(),
            allocations_saved: *self.allocations_saved.borrow(),
            allocations_made: *self.allocations_made.borrow(),
            reuse_ratio: if *self.allocations_made.borrow() > 0 {
                *self.allocations_saved.borrow() as f64 / 
                (*self.allocations_saved.borrow() + *self.allocations_made.borrow()) as f64
            } else {
                0.0
            },
        }
    }
}

/// Handle to borrowed Arc that doesn't clone data
pub struct ArcHandle {
    arc: Arc<[f32]>,
    arena: *const EmbeddingArena,
}

impl ArcHandle {
    /// Get slice reference without cloning
    pub fn as_slice(&self) -> &[f32] {
        &self.arc
    }
    
    /// Convert to owned only when absolutely necessary
    pub fn to_owned(&self) -> Vec<f32> {
        unsafe {
            if let Some(arena) = self.arena.as_ref() {
                let mut buffer = arena.get_buffer(self.arc.len());
                buffer.extend_from_slice(&self.arc);
                buffer
            } else {
                self.arc.to_vec()
            }
        }
    }
}

impl Drop for ArcHandle {
    fn drop(&mut self) {
        // Arc automatically decrements refcount
        // No explicit action needed
    }
}

/// Global arena pool shared across threads
pub struct GlobalArenaPool {
    // Lock-free queue for cross-thread buffer sharing
    shared_buffers: Arc<SegQueue<Vec<f32>>>,
    // Maximum global pool size
    max_global_size: usize,
}

impl GlobalArenaPool {
    pub fn new(max_global_size: usize) -> Self {
        Self {
            shared_buffers: Arc::new(SegQueue::new()),
            max_global_size,
        }
    }
    
    /// Donate unused buffer to global pool
    pub fn donate_buffer(&self, buffer: Vec<f32>) {
        // Only keep if under limit (approximate check)
        if self.shared_buffers.len() < self.max_global_size {
            self.shared_buffers.push(buffer);
        }
    }
    
    /// Try to get buffer from global pool
    pub fn try_get_buffer(&self) -> Option<Vec<f32>> {
        self.shared_buffers.pop()
    }
}

/// Thread-local storage for embedding arena
thread_local! {
    static ARENA: RefCell<EmbeddingArena> = RefCell::new(EmbeddingArena::new(100));
}

/// Get thread-local arena
pub fn with_arena<F, R>(f: F) -> R 
where 
    F: FnOnce(&EmbeddingArena) -> R
{
    ARENA.with(|arena| {
        f(&arena.borrow())
    })
}

/// Arena statistics
#[derive(Debug, Clone)]
pub struct ArenaStats {
    pub buffers_pooled: usize,
    pub allocations_saved: usize,
    pub allocations_made: usize,
    pub reuse_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_arena_reuse() {
        let arena = EmbeddingArena::new(10);
        
        // First allocation
        let buf1 = arena.get_buffer(1536);
        assert_eq!(arena.get_stats().allocations_made, 1);
        
        // Return to pool
        arena.return_buffer(buf1);
        
        // Second allocation should reuse
        let buf2 = arena.get_buffer(1536);
        assert_eq!(arena.get_stats().allocations_saved, 1);
        
        arena.return_buffer(buf2);
    }
    
    #[test]
    fn test_arc_handle() {
        let arena = EmbeddingArena::new(10);
        let data = vec![1.0, 2.0, 3.0];
        let arc: Arc<[f32]> = Arc::from(data.into_boxed_slice());
        
        let handle = arena.borrow_arc(&arc);
        assert_eq!(handle.as_slice(), &[1.0, 2.0, 3.0]);
        
        // No data copy happens here
        let slice = handle.as_slice();
        assert_eq!(slice.len(), 3);
    }
}
