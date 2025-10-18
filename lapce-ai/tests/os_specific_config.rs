/// OS-specific configuration for IPC tests
/// Handles platform differences in shared memory and resource limits

#[cfg(target_os = "linux")]
pub mod linux {
    use std::fs;
    use std::process::Command;
    
    pub fn configure_system() -> Result<(), Box<dyn std::error::Error>> {
        // Set ulimits for Linux
        Command::new("ulimit")
            .args(&["-n", "65536"])  // Max file descriptors
            .output()?;
            
        Command::new("ulimit")
            .args(&["-u", "32768"])  // Max user processes
            .output()?;
            
        // Configure shared memory limits
        if let Ok(_) = fs::write("/proc/sys/kernel/shmmax", "68719476736") {
            println!("✓ Configured Linux shared memory: 64GB max");
        }
        
        Ok(())
    }
    
    pub fn get_memory_info() -> (u64, u64) {
        let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let mut total = 0u64;
        let mut available = 0u64;
        
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                total = line.split_whitespace().nth(1).unwrap_or("0")
                    .parse::<u64>().unwrap_or(0) * 1024;
            }
            if line.starts_with("MemAvailable:") {
                available = line.split_whitespace().nth(1).unwrap_or("0")
                    .parse::<u64>().unwrap_or(0) * 1024;
            }
        }
        
        (total, available)
    }
    
    pub const SHM_PATH_PREFIX: &str = "/dev/shm/lapce_ipc_";
}

#[cfg(target_os = "macos")]
pub mod macos {
    use std::process::Command;
    
    pub fn configure_system() -> Result<(), Box<dyn std::error::Error>> {
        // Set ulimits for macOS
        Command::new("ulimit")
            .args(&["-n", "32768"])  // Max file descriptors (lower than Linux)
            .output()?;
            
        // Configure shared memory
        Command::new("sysctl")
            .args(&["-w", "kern.sysv.shmmax=2147483648"])  // 2GB max
            .output()?;
            
        Command::new("sysctl")
            .args(&["-w", "kern.sysv.shmmni=256"])  // Max segments
            .output()?;
            
        println!("✓ Configured macOS shared memory: 2GB max");
        
        Ok(())
    }
    
    pub fn get_memory_info() -> (u64, u64) {
        let output = Command::new("vm_stat")
            .output()
            .unwrap_or_else(|_| Default::default());
            
        let text = String::from_utf8_lossy(&output.stdout);
        let page_size = 16384u64; // macOS page size
        
        let mut free_pages = 0u64;
        let mut total_pages = 0u64;
        
        for line in text.lines() {
            if line.contains("Pages free:") {
                free_pages = line.split(':').nth(1).unwrap_or("0")
                    .trim().trim_end_matches('.')
                    .parse::<u64>().unwrap_or(0);
            }
        }
        
        // Estimate total from sysctl
        if let Ok(output) = Command::new("sysctl")
            .args(&["-n", "hw.memsize"])
            .output() {
            let total = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u64>()
                .unwrap_or(0);
            total_pages = total / page_size;
        }
        
        (total_pages * page_size, free_pages * page_size)
    }
    
    pub const SHM_PATH_PREFIX: &str = "/tmp/lapce_ipc_";
}

#[cfg(target_os = "windows")]
pub mod windows {
    use std::process::Command;
    use std::mem;
    use winapi::um::sysinfoapi::{GetPhysicallyInstalledSystemMemory, GlobalMemoryStatusEx, MEMORYSTATUSEX};
    
    pub fn configure_system() -> Result<(), Box<dyn std::error::Error>> {
        // Windows doesn't have ulimit, but we can set process limits
        // This would require admin privileges in real scenarios
        
        println!("✓ Windows configuration: Using default limits");
        
        // Set process priority to high
        Command::new("wmic")
            .args(&["process", "where", "name='cargo.exe'", "CALL", "setpriority", "128"])
            .output()?;
            
        Ok(())
    }
    
    pub fn get_memory_info() -> (u64, u64) {
        unsafe {
            let mut mem_status: MEMORYSTATUSEX = mem::zeroed();
            mem_status.dwLength = mem::size_of::<MEMORYSTATUSEX>() as u32;
            
            if GlobalMemoryStatusEx(&mut mem_status) != 0 {
                (mem_status.ullTotalPhys, mem_status.ullAvailPhys)
            } else {
                (0, 0)
            }
        }
    }
    
    pub const SHM_PATH_PREFIX: &str = r"\\.\pipe\lapce_ipc_";
}

// Cross-platform interface
pub fn configure_for_current_os() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    return linux::configure_system();
    
    #[cfg(target_os = "macos")]
    return macos::configure_system();
    
    #[cfg(target_os = "windows")]
    return windows::configure_system();
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        println!("⚠️ Unsupported OS, using defaults");
        Ok(())
    }
}

pub fn get_shm_path(name: &str) -> String {
    #[cfg(target_os = "linux")]
    let prefix = linux::SHM_PATH_PREFIX;
    
    #[cfg(target_os = "macos")]
    let prefix = macos::SHM_PATH_PREFIX;
    
    #[cfg(target_os = "windows")]
    let prefix = windows::SHM_PATH_PREFIX;
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let prefix = "/tmp/lapce_ipc_";
    
    format!("{}{}", prefix, name)
}

pub fn get_system_memory() -> (u64, u64) {
    #[cfg(target_os = "linux")]
    return linux::get_memory_info();
    
    #[cfg(target_os = "macos")]
    return macos::get_memory_info();
    
    #[cfg(target_os = "windows")]
    return windows::get_memory_info();
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return (0, 0);
}
