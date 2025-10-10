/// Per-boot random SHM namespace suffix for security
/// Prevents cross-boot shared memory conflicts and improves isolation

use std::sync::OnceLock;
use std::fs;
use anyhow::{Result, Context};
use tracing::{info, warn};

static BOOT_SUFFIX: OnceLock<String> = OnceLock::new();

/// Get or generate the per-boot random SHM namespace suffix
pub fn get_boot_suffix() -> &'static str {
    BOOT_SUFFIX.get_or_init(|| {
        generate_boot_suffix().unwrap_or_else(|e| {
            warn!("Failed to generate secure boot suffix: {}, using fallback", e);
            generate_fallback_suffix()
        })
    })
}

/// Generate secure per-boot suffix based on system state
fn generate_boot_suffix() -> Result<String> {
    // Try to get boot ID from /proc/sys/kernel/random/boot_id
    if let Ok(boot_id) = fs::read_to_string("/proc/sys/kernel/random/boot_id") {
        let clean_id = boot_id.trim().replace('-', "");
        if clean_id.len() >= 8 {
            let suffix = &clean_id[..8]; // First 8 chars
            info!("Using kernel boot_id suffix: {}", suffix);
            return Ok(suffix.to_string());
        }
    }
    
    // Fallback: Try uptime + process info
    if let Ok(uptime) = fs::read_to_string("/proc/uptime") {
        let uptime_parts: Vec<&str> = uptime.split_whitespace().collect();
        if let Some(uptime_str) = uptime_parts.get(0) {
            if let Ok(uptime_f64) = uptime_str.parse::<f64>() {
                let pid = std::process::id();
                let boot_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() - uptime_f64 as u64;
                
                let suffix = format!("{:08x}", boot_time ^ (pid as u64));
                info!("Using uptime-based suffix: {}", suffix);
                return Ok(suffix);
            }
        }
    }
    
    anyhow::bail!("Could not determine boot information")
}

/// Generate fallback suffix when secure methods fail
fn generate_fallback_suffix() -> String {
    let pid = std::process::id();
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let suffix = format!("{:08x}", time ^ (pid as u64) ^ rand::random::<u64>());
    warn!("Using fallback random suffix: {}", suffix);
    suffix
}

/// Create namespaced SHM path with boot suffix
pub fn create_namespaced_path(base_path: &str) -> String {
    let suffix = get_boot_suffix();
    let base = if base_path.starts_with('/') {
        base_path
    } else {
        &format!("/{}", base_path)
    };
    
    // macOS has a 31-character limit (PSHMNAMLEN) for shm names
    // After replacing '/' with '_', we need: base + '-' + 8-char-suffix = max 31 chars
    // So base can be at most 31 - 1 - 8 = 22 chars (including leading /)
    #[cfg(target_os = "macos")]
    let base = if base.len() > 22 {
        // Truncate but keep leading /
        &base[..22]
    } else {
        base
    };
    
    format!("{}-{}", base, suffix)
}

/// Extract base path from namespaced path
pub fn extract_base_path(namespaced_path: &str) -> String {
    if let Some(dash_pos) = namespaced_path.rfind('-') {
        if dash_pos >= 8 { // Account for suffix length
            return namespaced_path[..dash_pos].to_string();
        }
    }
    namespaced_path.to_string()
}

/// Check if a path belongs to the current boot
pub fn is_current_boot_path(path: &str) -> bool {
    let suffix = get_boot_suffix();
    path.ends_with(&format!("-{}", suffix))
}

/// Clean up old SHM segments from previous boots
pub fn cleanup_stale_shm_segments(base_paths: &[&str]) -> Result<()> {
    let current_suffix = get_boot_suffix();
    let mut cleaned_count = 0;
    
    // On Linux, SHM segments appear as files in /dev/shm
    if let Ok(entries) = fs::read_dir("/dev/shm") {
        for entry in entries {
            let entry = entry.context("Failed to read /dev/shm entry")?;
            let path = entry.path();
            
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                // Check if this looks like one of our SHM segments
                for base_path in base_paths {
                    let base_name = base_path.trim_start_matches('/');
                    if filename.starts_with(base_name) && filename.contains('-') {
                        // Extract suffix
                        if let Some(dash_pos) = filename.rfind('-') {
                            let suffix = &filename[dash_pos + 1..];
                            
                            // If it's not our current boot suffix, clean it up
                            if suffix != current_suffix && suffix.len() == 8 {
                                if let Err(e) = fs::remove_file(&path) {
                                    warn!("Failed to clean stale SHM segment {}: {}", filename, e);
                                } else {
                                    info!("Cleaned stale SHM segment: {}", filename);
                                    cleaned_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    if cleaned_count > 0 {
        info!("Cleaned {} stale SHM segments from previous boots", cleaned_count);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_boot_suffix_generation() {
        let suffix1 = get_boot_suffix();
        let suffix2 = get_boot_suffix();
        
        // Should be consistent within same process
        assert_eq!(suffix1, suffix2);
        
        // Should be 8 characters (hex)
        assert_eq!(suffix1.len(), 8);
        
        // Should be valid hex
        assert!(u32::from_str_radix(suffix1, 16).is_ok());
    }
    
    #[test]
    fn test_namespaced_path_creation() {
        let path = create_namespaced_path("/test_path");
        assert!(path.starts_with("/test_path-"));
        assert_eq!(path.len(), "/test_path-".len() + 8);
        
        let path2 = create_namespaced_path("test_path");
        assert!(path2.starts_with("/test_path-"));
    }
    
    #[test]
    fn test_base_path_extraction() {
        let namespaced = "/test_path-12345678";
        let base = extract_base_path(namespaced);
        assert_eq!(base, "/test_path");
    }
    
    #[test]
    fn test_current_boot_path_check() {
        let suffix = get_boot_suffix();
        let current_path = format!("/test-{}", suffix);
        let old_path = "/test-abcd1234";
        
        assert!(is_current_boot_path(&current_path));
        assert!(!is_current_boot_path(old_path));
    }
}
