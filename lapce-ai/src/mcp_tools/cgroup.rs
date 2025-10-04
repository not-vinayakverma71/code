use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Write};
use anyhow::{Context, Result};

pub struct CGroupManager {
    cgroup_path: PathBuf,
    cgroup_name: String,
}

impl CGroupManager {
    pub fn new(name: String) -> Result<Self> {
        let cgroup_path = PathBuf::from("/sys/fs/cgroup");
        Ok(Self {
            cgroup_path,
            cgroup_name: name,
        })
    }
    
    pub fn create_cgroup(&self, subsystem: &str) -> Result<PathBuf> {
        let path = self.cgroup_path
            .join(subsystem)
            .join(&self.cgroup_name);
        
        fs::create_dir_all(&path)
            .context("Failed to create cgroup directory")?;
        
        Ok(path)
    }
    
    pub fn set_memory_limit(&self, limit_bytes: u64) -> Result<()> {
        let cgroup_path = self.create_cgroup("memory")?;
        
        // Set memory limit
        let limit_path = cgroup_path.join("memory.limit_in_bytes");
        fs::write(&limit_path, limit_bytes.to_string())
            .context("Failed to set memory limit")?;
        
        // Set swap limit (same as memory to prevent swap)
        let swap_path = cgroup_path.join("memory.memsw.limit_in_bytes");
        fs::write(&swap_path, limit_bytes.to_string())
            .context("Failed to set swap limit")?;
        
        Ok(())
    }
    
    pub fn set_cpu_quota(&self, quota_us: u32, period_us: u32) -> Result<()> {
        let cgroup_path = self.create_cgroup("cpu")?;
        
        // Set CPU period
        let period_path = cgroup_path.join("cpu.cfs_period_us");
        fs::write(&period_path, period_us.to_string())
            .context("Failed to set CPU period")?;
        
        // Set CPU quota
        let quota_path = cgroup_path.join("cpu.cfs_quota_us");
        fs::write(&quota_path, quota_us.to_string())
            .context("Failed to set CPU quota")?;
        
        Ok(())
    }
    
    pub fn add_process(&self, subsystem: &str, pid: u32) -> Result<()> {
        let cgroup_path = self.cgroup_path
            .join(subsystem)
            .join(&self.cgroup_name);
        
        let tasks_path = cgroup_path.join("cgroup.procs");
        fs::write(&tasks_path, pid.to_string())
            .context("Failed to add process to cgroup")?;
        
        Ok(())
    }
    
    pub fn cleanup(&self) -> Result<()> {
        // Remove cgroup directories
        for subsystem in &["memory", "cpu", "cpuacct"] {
            let path = self.cgroup_path
                .join(subsystem)
                .join(&self.cgroup_name);
            
            if path.exists() {
                fs::remove_dir(&path).ok();
            }
        }
        
        Ok(())
    }
}

impl Drop for CGroupManager {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

// CGroup v2 support
pub struct CGroupV2Manager {
    cgroup_path: PathBuf,
}

impl CGroupV2Manager {
    pub fn new(name: String) -> Result<Self> {
        let cgroup_path = PathBuf::from("/sys/fs/cgroup").join(name);
        fs::create_dir_all(&cgroup_path)?;
        
        Ok(Self { cgroup_path })
    }
    
    pub fn set_memory_max(&self, max_bytes: u64) -> Result<()> {
        let memory_max = self.cgroup_path.join("memory.max");
        fs::write(memory_max, max_bytes.to_string())?;
        Ok(())
    }
    
    pub fn set_cpu_max(&self, quota_us: u32, period_us: u32) -> Result<()> {
        let cpu_max = self.cgroup_path.join("cpu.max");
        fs::write(cpu_max, format!("{} {}", quota_us, period_us))?;
        Ok(())
    }
    
    pub fn add_process(&self, pid: u32) -> Result<()> {
        let procs = self.cgroup_path.join("cgroup.procs");
        fs::write(procs, pid.to_string())?;
        Ok(())
    }
}
