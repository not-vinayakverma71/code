/// Enforce 0600 permissions on shared memory segments and lock files
/// Security: Restrict access to owner-only (user isolation)

#![cfg(unix)]

use anyhow::{Result, Context};
use std::os::unix::fs::{PermissionsExt, MetadataExt};
use std::path::Path;
use tracing::{warn, debug};

/// Required permissions: owner read/write only (0600)
const REQUIRED_MODE: u32 = 0o600;

/// Enforce 0600 permissions on a file or directory
pub fn enforce_0600<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for {:?}", path))?;
    
    let current_mode = metadata.permissions().mode() & 0o777;
    
    if current_mode != REQUIRED_MODE {
        debug!("Fixing permissions on {:?}: {:o} -> {:o}", path, current_mode, REQUIRED_MODE);
        let mut perms = metadata.permissions();
        perms.set_mode(REQUIRED_MODE);
        std::fs::set_permissions(path, perms)
            .with_context(|| format!("Failed to set 0600 permissions on {:?}", path))?;
    }
    
    Ok(())
}

/// Verify permissions are 0600 (read-only check)
pub fn verify_0600<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for {:?}", path))?;
    
    let current_mode = metadata.permissions().mode() & 0o777;
    
    if current_mode != REQUIRED_MODE {
        anyhow::bail!(
            "Insecure permissions on {:?}: {:o} (expected 0600)",
            path,
            current_mode
        );
    }
    
    Ok(())
}

/// Create file descriptor with 0600 permissions
pub fn create_fd_0600(fd: std::os::unix::io::RawFd) -> Result<()> {
    unsafe {
        // Cast to libc::mode_t for platform compatibility (u16 on macOS, u32 on Linux)
        let result = libc::fchmod(fd, REQUIRED_MODE as libc::mode_t);
        if result != 0 {
            let err = std::io::Error::last_os_error();
            anyhow::bail!("fchmod failed: {}", err);
        }
    }
    Ok(())
}

/// Get current user ID for namespace isolation
pub fn get_current_uid() -> u32 {
    unsafe { libc::getuid() }
}

/// Get current session ID for additional isolation
pub fn get_current_session() -> Option<u32> {
    unsafe {
        let sid = libc::getsid(0);
        if sid > 0 {
            Some(sid as u32)
        } else {
            None
        }
    }
}

/// Create per-user namespaced base path
/// Format: /lapce_ipc_{uid}_{session}
pub fn create_user_namespaced_base() -> String {
    let uid = get_current_uid();
    
    if let Some(session) = get_current_session() {
        format!("/lapce_ipc_{}_{}", uid, session)
    } else {
        format!("/lapce_ipc_{}", uid)
    }
}

/// Validate path belongs to current user
pub fn validate_path_ownership<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for {:?}", path))?;
    
    let file_uid = metadata.uid();
    let current_uid = get_current_uid();
    
    if file_uid != current_uid {
        anyhow::bail!(
            "Path {:?} owned by uid {} (current user: {})",
            path,
            file_uid,
            current_uid
        );
    }
    
    Ok(())
}

/// Set up secure directory for lock files with 0700 permissions
pub fn create_secure_lock_dir<P: AsRef<Path>>(dir: P) -> Result<()> {
    let dir = dir.as_ref();
    
    if !dir.exists() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create lock directory {:?}", dir))?;
    }
    
    // Set 0700 on directory (owner-only access)
    let metadata = std::fs::metadata(dir)?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o700);
    std::fs::set_permissions(dir, perms)
        .with_context(|| format!("Failed to set 0700 on directory {:?}", dir))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_enforce_0600() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file");
        
        // Create file with wrong permissions
        let file = File::create(&file_path).unwrap();
        let mut perms = file.metadata().unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&file_path, perms).unwrap();
        
        // Verify it's not 0600
        let mode = std::fs::metadata(&file_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);
        
        // Enforce 0600
        enforce_0600(&file_path).unwrap();
        
        // Verify it's now 0600
        let mode = std::fs::metadata(&file_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_verify_0600() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file");
        
        // Create file with 0600
        let file = File::create(&file_path).unwrap();
        let mut perms = file.metadata().unwrap().permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&file_path, perms).unwrap();
        
        // Should pass
        assert!(verify_0600(&file_path).is_ok());
        
        // Change to 0644
        let mut perms = std::fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&file_path, perms).unwrap();
        
        // Should fail
        assert!(verify_0600(&file_path).is_err());
    }

    #[test]
    fn test_user_namespaced_base() {
        let base = create_user_namespaced_base();
        let uid = get_current_uid();
        
        assert!(base.contains(&uid.to_string()));
        assert!(base.starts_with("/lapce_ipc_"));
    }

    #[test]
    fn test_secure_lock_dir() {
        let dir = tempdir().unwrap();
        let lock_dir = dir.path().join("locks");
        
        create_secure_lock_dir(&lock_dir).unwrap();
        
        assert!(lock_dir.exists());
        let mode = std::fs::metadata(&lock_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
    }
}
