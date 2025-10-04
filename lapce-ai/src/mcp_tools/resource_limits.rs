/// Resource Limits for Tool Execution
use std::time::Duration;
use nix::sys::resource::{setrlimit, Resource};
use anyhow::Result;

pub struct ResourceLimiter {
    cpu_limit: Option<Duration>,
    memory_limit: Option<usize>,
    file_limit: Option<usize>,
}

impl ResourceLimiter {
    pub fn new() -> Self {
        Self {
            cpu_limit: Some(Duration::from_secs(30)),
            memory_limit: Some(512 * 1024 * 1024), // 512MB
            file_limit: Some(100),
        }
    }
    
    pub fn apply_limits(&self) -> Result<()> {
        // CPU time limit
        if let Some(cpu_secs) = self.cpu_limit {
            setrlimit(Resource::RLIMIT_CPU, cpu_secs.as_secs(), Some(cpu_secs.as_secs()))?;
        }
        
        // Memory limit
        if let Some(mem_bytes) = self.memory_limit {
            setrlimit(Resource::RLIMIT_AS, mem_bytes as u64, Some(mem_bytes as u64))?;
        }
        
        // File descriptor limit
        if let Some(fd_limit) = self.file_limit {
            setrlimit(Resource::RLIMIT_NOFILE, fd_limit as u64, Some(fd_limit as u64))?;
        }
        
        Ok(())
    }
}
