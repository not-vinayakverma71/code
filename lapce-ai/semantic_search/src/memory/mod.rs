// Memory optimization and profiling module
pub mod shared_pool;
pub mod profiler;
pub use shared_pool::{SharedMemoryPool, SharedSegment, PoolStats};
