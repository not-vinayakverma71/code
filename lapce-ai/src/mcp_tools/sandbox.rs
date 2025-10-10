use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;
#[cfg(unix)]
use nix::unistd::{Uid, Gid};
use parking_lot::Mutex;
use crate::mcp_tools::system::CGroupManager;

// Process Sandboxing from lines 411-471

pub struct SandboxConfig {
    pub working_dir: PathBuf,
    pub env_vars: HashMap<String, String>,
    pub timeout: Duration,
    pub memory_limit: usize,
    pub cpu_limit: Duration,
}

pub struct ProcessOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct ProcessSandbox {
    chroot_path: PathBuf,
    #[cfg(unix)]
    uid: Option<Uid>,
    #[cfg(unix)]
    gid: Option<Gid>,
    enable_namespaces: bool,
    enable_seccomp: bool,
    resource_limits: ResourceLimits,
    pub cgroup_manager: Arc<CGroupManager>,
    pub namespace_manager: Arc<NamespaceManager>,
    pub temp_dirs: Arc<Mutex<Vec<PathBuf>>>,
    pub allowed_paths: Vec<PathBuf>,
    pub denied_commands: Vec<String>,
}

#[derive(Clone)]
pub struct ResourceLimits {
    pub cpu_time: Option<u64>,      // seconds
    pub memory: Option<u64>,        // bytes
    pub file_size: Option<u64>,     // bytes
    pub process_count: Option<u64>, // number of processes
    pub open_files: Option<u64>,    // number of file descriptors
    pub stack_size: Option<u64>,    // bytes
}

pub struct NamespaceManager {
    namespaces: HashMap<String, u32>,
}

impl NamespaceManager {
    pub fn new() -> Self {
        Self {
            namespaces: HashMap::new(),
        }
    }
}

impl ProcessSandbox {
    pub fn new() -> Self {
        Self {
            chroot_path: PathBuf::from("/tmp/sandbox"),
            uid: None,
            gid: None,
            enable_namespaces: false,
            enable_seccomp: false,
            resource_limits: ResourceLimits {
                cpu_time: Some(30),
                memory: Some(100 * 1024 * 1024),
                file_size: Some(10 * 1024 * 1024),
                process_count: Some(10),
                open_files: Some(100),
                stack_size: Some(8 * 1024 * 1024),
            },
            cgroup_manager: Arc::new(CGroupManager::new()),
            namespace_manager: Arc::new(NamespaceManager::new()),
            temp_dirs: Arc::new(Mutex::new(Vec::new())),
            allowed_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/home"),
            ],
            denied_commands: vec![
                "sudo".to_string(),
                "su".to_string(),
                "chmod".to_string(),
                "chown".to_string(),
            ],
        }
    }
    
    pub async fn execute_sandboxed(
        &self,
        command: &str,
        config: SandboxConfig,
    ) -> Result<ProcessOutput> {
        // REAL IMPLEMENTATION with resource limits
        use tokio::process::Command;
        use std::process::Stdio;
        
        // Security check - block dangerous commands
        for denied in &self.denied_commands {
            if command.contains(denied) {
                return Ok(ProcessOutput {
                    stdout: String::new(),
                    stderr: format!("Command '{}' is not allowed in sandbox", denied),
                    exit_code: 1,
                });
            }
        }
        
        // Check path access
        if !self.allowed_paths.iter().any(|p| config.working_dir.starts_with(p)) {
            return Ok(ProcessOutput {
                stdout: String::new(),
                stderr: format!("Working directory {:?} not in allowed paths", config.working_dir),
                exit_code: 1,
            });
        }
        
        // Build command with resource limits
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.current_dir(&config.working_dir);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        // Set environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }
        
        // Apply ulimit-style resource limits using shell
        let limited_command = format!(
            "ulimit -v {} && ulimit -t {} && {}",
            config.memory_limit / 1024, // Convert to KB for ulimit
            config.cpu_limit.as_secs(),
            command
        );
        
        let mut limited_cmd = Command::new("sh");
        limited_cmd.arg("-c").arg(&limited_command);
        limited_cmd.current_dir(&config.working_dir);
        limited_cmd.stdin(Stdio::null());
        limited_cmd.stdout(Stdio::piped());
        limited_cmd.stderr(Stdio::piped());
        
        // Set environment variables
        for (key, value) in &config.env_vars {
            limited_cmd.env(key, value);
        }
        
        // Execute with timeout
        match tokio::time::timeout(config.timeout, limited_cmd.output()).await {
            Ok(Ok(output)) => {
                Ok(ProcessOutput {
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    exit_code: output.status.code().unwrap_or(-1),
                })
            },
            Ok(Err(e)) => {
                Ok(ProcessOutput {
                    stdout: String::new(),
                    stderr: format!("Failed to execute command: {}", e),
                    exit_code: -1,
                })
            },
            Err(_) => {
                Ok(ProcessOutput {
                    stdout: String::new(),
                    stderr: format!("Command timed out after {:?}", config.timeout),
                    exit_code: -1,
                })
            }
        }
    }
    
    pub async fn create_temp_sandbox(&self) -> Result<PathBuf> {
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path().to_path_buf();
        
        // Store temp dir to clean up later
        self.temp_dirs.lock().push(path.clone());
        
        // Forget the tempdir to prevent automatic cleanup
        std::mem::forget(temp_dir);
        
        Ok(path)
    }
    
    pub async fn cleanup_temp_sandboxes(&self) -> Result<()> {
        let mut temp_dirs = self.temp_dirs.lock();
        for dir in temp_dirs.drain(..) {
            if dir.exists() {
                tokio::fs::remove_dir_all(dir).await?;
            }
        }
        Ok(())
    }
}
