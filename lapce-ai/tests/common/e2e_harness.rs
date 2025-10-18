/// E2E Test Harness - Launches real IPC server process with LSP gateway
/// NO MOCKS - Separate processes for true cross-process IPC validation

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use anyhow::{Context, Result};

/// E2E test harness that manages a real IPC server process
pub struct E2eHarness {
    server_process: Option<Child>,
    server_pid: Option<u32>,
    ipc_socket_path: PathBuf,
    shm_prefix: String,
    temp_dir: tempfile::TempDir,
}

impl E2eHarness {
    /// Start a new IPC server process with LSP gateway enabled
    pub async fn start() -> Result<Self> {
        let temp_dir = tempfile::TempDir::new()
            .context("Failed to create temp directory")?;
        
        let ipc_socket_path = temp_dir.path().join("lapce-ipc.sock");
        let shm_prefix = format!("lapce-e2e-test-{}", std::process::id());
        
        // Build path to IPC server binary
        let server_binary = Self::find_server_binary()
            .context("Failed to find IPC server binary")?;
        
        println!("Starting IPC server: {}", server_binary.display());
        println!("Socket path: {}", ipc_socket_path.display());
        println!("SHM prefix: {}", shm_prefix);
        
        // Launch server process
        let mut cmd = Command::new(&server_binary);
        cmd.arg("--enable-lsp-gateway")
            .arg("--socket")
            .arg(&ipc_socket_path)
            .arg("--shm-prefix")
            .arg(&shm_prefix)
            .env("RUST_LOG", "lapce_ai=debug")
            .env("RUST_BACKTRACE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        let mut server_process = cmd.spawn()
            .context("Failed to spawn IPC server process")?;
        
        let server_pid = server_process.id();
        println!("IPC server started with PID: {}", server_pid);
        
        // Wait for server to be ready (check socket exists)
        let mut attempts = 0;
        while !ipc_socket_path.exists() && attempts < 50 {
            sleep(Duration::from_millis(100)).await;
            attempts += 1;
            
            // Check if process died
            if let Ok(Some(status)) = server_process.try_wait() {
                anyhow::bail!("IPC server died during startup with status: {}", status);
            }
        }
        
        if !ipc_socket_path.exists() {
            anyhow::bail!("IPC server failed to create socket after 5 seconds");
        }
        
        println!("IPC server ready");
        
        Ok(Self {
            server_process: Some(server_process),
            server_pid: Some(server_pid),
            ipc_socket_path,
            shm_prefix,
            temp_dir,
        })
    }
    
    /// Find the IPC server binary (either in target/debug or target/release)
    fn find_server_binary() -> Result<PathBuf> {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .context("Failed to get workspace root")?
            .to_path_buf();
        
        // Try debug first, then release
        let candidates = vec![
            workspace_root.join("target/debug/lapce_ipc_server"),
            workspace_root.join("target/release/lapce_ipc_server"),
            workspace_root.join("lapce-ai/target/debug/lapce_ipc_server"),
            workspace_root.join("lapce-ai/target/release/lapce_ipc_server"),
        ];
        
        for candidate in candidates {
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        
        anyhow::bail!("IPC server binary not found. Run: cargo build --bin lapce_ipc_server --features lsp_gateway")
    }
    
    /// Get the socket path for connecting to the IPC server
    pub fn socket_path(&self) -> &PathBuf {
        &self.ipc_socket_path
    }
    
    /// Get the shared memory prefix
    pub fn shm_prefix(&self) -> &str {
        &self.shm_prefix
    }
    
    /// Get the server PID
    pub fn server_pid(&self) -> Option<u32> {
        self.server_pid
    }
    
    /// Check if the server process is still running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut process) = self.server_process {
            match process.try_wait() {
                Ok(Some(_)) => false, // Exited
                Ok(None) => true,     // Still running
                Err(_) => false,      // Error checking
            }
        } else {
            false
        }
    }
    
    /// Gracefully shutdown the server
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(mut process) = self.server_process.take() {
            println!("Shutting down IPC server (PID: {})", process.id());
            
            // Try graceful SIGTERM first
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                
                let pid = Pid::from_raw(process.id() as i32);
                let _ = kill(pid, Signal::SIGTERM);
                
                // Wait up to 5 seconds for graceful shutdown
                for _ in 0..50 {
                    if let Ok(Some(_)) = process.try_wait() {
                        println!("IPC server shut down gracefully");
                        return Ok(());
                    }
                    sleep(Duration::from_millis(100)).await;
                }
                
                // Force kill if still running
                println!("IPC server didn't respond to SIGTERM, sending SIGKILL");
                let _ = kill(pid, Signal::SIGKILL);
            }
            
            #[cfg(not(unix))]
            {
                // Windows: just kill
                let _ = process.kill();
            }
            
            let _ = process.wait();
        }
        
        Ok(())
    }
    
    /// Force kill the server (for chaos tests)
    pub fn force_kill(&mut self) -> Result<()> {
        if let Some(mut process) = self.server_process.as_mut() {
            println!("Force killing IPC server (PID: {})", process.id());
            
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                
                let pid = Pid::from_raw(process.id() as i32);
                kill(pid, Signal::SIGKILL)
                    .context("Failed to send SIGKILL")?;
            }
            
            #[cfg(not(unix))]
            {
                process.kill()
                    .context("Failed to kill process")?;
            }
            
            let _ = process.wait();
            self.server_process = None;
        }
        
        Ok(())
    }
    
    /// Restart the server (for recovery tests)
    pub async fn restart(&mut self) -> Result<()> {
        self.shutdown().await?;
        
        // Wait a bit for cleanup
        sleep(Duration::from_millis(500)).await;
        
        // Start new instance
        let new_harness = Self::start().await?;
        
        // Replace our state
        self.server_process = new_harness.server_process;
        self.server_pid = new_harness.server_pid;
        
        Ok(())
    }
    
    /// Read server stdout (for debugging)
    pub fn read_stdout(&mut self) -> Option<String> {
        // TODO: Implement stdout capture if needed for debugging
        None
    }
    
    /// Read server stderr (for debugging)
    pub fn read_stderr(&mut self) -> Option<String> {
        // TODO: Implement stderr capture if needed for debugging
        None
    }
}

impl Drop for E2eHarness {
    fn drop(&mut self) {
        // Cleanup: kill server if still running
        if let Some(mut process) = self.server_process.take() {
            println!("Cleaning up IPC server on drop");
            let _ = process.kill();
            let _ = process.wait();
        }
        
        // Cleanup shared memory objects
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            // Remove any shared memory files
            let shm_dir = PathBuf::from("/dev/shm");
            if let Ok(entries) = fs::read_dir(&shm_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(&self.shm_prefix) {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_harness_start_stop() {
        let mut harness = E2eHarness::start().await
            .expect("Failed to start harness");
        
        assert!(harness.is_running(), "Server should be running");
        assert!(harness.socket_path().exists(), "Socket should exist");
        
        harness.shutdown().await
            .expect("Failed to shutdown harness");
        
        assert!(!harness.is_running(), "Server should be stopped");
    }
    
    #[tokio::test]
    async fn test_harness_restart() {
        let mut harness = E2eHarness::start().await
            .expect("Failed to start harness");
        
        let original_pid = harness.server_pid();
        
        harness.restart().await
            .expect("Failed to restart harness");
        
        assert!(harness.is_running(), "Server should be running after restart");
        assert_ne!(harness.server_pid(), original_pid, "PID should be different after restart");
    }
}
