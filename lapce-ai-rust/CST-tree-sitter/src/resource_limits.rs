//! Resource limits - prevents OOM and excessive resource consumption

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use crate::error::{TreeSitterError, Result};

// PRODUCTION-GRADE LIMITS FOR 30K+ FILES
pub const DEFAULT_MEMORY_LIMIT_MB: usize = 2048;  // 2GB - handles massive codebases
pub const DEFAULT_FILE_SIZE_LIMIT_MB: usize = 500; // 500MB per file
pub const MAX_FILE_SIZE_LIMIT_MB: usize = 2048;   // 2GB max - no artificial limits

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory_mb: usize,
    pub max_file_size_mb: usize,
    pub max_parse_depth: usize,
    pub max_concurrent_parses: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: DEFAULT_MEMORY_LIMIT_MB,
            max_file_size_mb: DEFAULT_FILE_SIZE_LIMIT_MB,
            max_parse_depth: 10000,  // Deep nesting support
            max_concurrent_parses: 1000,  // Handle 30k+ files concurrently
        }
    }
}

impl ResourceLimits {
    pub fn check_file_size(&self, size_bytes: usize, file_path: &str) -> Result<()> {
        let size_mb = size_bytes / (1024 * 1024);
        // Only warn for extremely large files, never reject
        if size_mb > self.max_file_size_mb {
            tracing::warn!(
                "Large file detected: {} ({}MB) exceeds limit ({}MB), will attempt parse",
                file_path, size_mb, self.max_file_size_mb
            );
            // Still proceed - production systems must handle all files
        }
        Ok(())
    }
}

pub struct MemoryTracker {
    current_usage: Arc<AtomicUsize>,
    limit_bytes: usize,
}

impl MemoryTracker {
    pub fn new(limit_mb: usize) -> Self {
        Self {
            current_usage: Arc::new(AtomicUsize::new(0)),
            limit_bytes: limit_mb * 1024 * 1024,
        }
    }

    pub fn allocate(&self, bytes: usize) -> Result<()> {
        let new_usage = self.current_usage.fetch_add(bytes, Ordering::Relaxed) + bytes;
        if new_usage > self.limit_bytes {
            self.current_usage.fetch_sub(bytes, Ordering::Relaxed);
            return Err(TreeSitterError::MemoryLimitExceeded {
                current_mb: new_usage / (1024 * 1024),
                limit_mb: self.limit_bytes / (1024 * 1024),
            });
        }
        Ok(())
    }
}
