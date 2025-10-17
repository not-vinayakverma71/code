//! System Information Section
//!
//! 1:1 Translation from Codex `src/core/prompts/sections/system-info.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/sections/system-info.ts (lines 1-20)

use std::path::Path;

/// Get operating system name
fn get_os_name() -> String {
    // Match os-name() behavior from Node.js
    let os = std::env::consts::OS;
    let version = match os {
        "linux" => {
            // Try to get distribution name
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if let Some(name) = line.strip_prefix("PRETTY_NAME=") {
                        return name.trim_matches('"').to_string();
                    }
                }
            }
            "Linux".to_string()
        }
        "macos" => {
            // Try to get macOS version
            if let Ok(output) = std::process::Command::new("sw_vers")
                .arg("-productVersion")
                .output()
            {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    return format!("macOS {}", version.trim());
                }
            }
            "macOS".to_string()
        }
        "windows" => "Windows".to_string(),
        _ => os.to_string(),
    };
    
    version
}

/// Get default shell
fn get_shell() -> String {
    std::env::var("SHELL")
        .unwrap_or_else(|_| {
            // Fallback defaults
            if cfg!(target_os = "windows") {
                "cmd.exe".to_string()
            } else {
                "/bin/sh".to_string()
            }
        })
}

/// Get home directory
fn get_home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~".to_string())
}

/// Generate system information section
///
/// Translation of getSystemInfoSection() from system-info.ts (lines 6-19)
pub fn system_info_section(workspace: &Path) -> String {
    let os_name = get_os_name();
    let shell = get_shell();
    let home_dir = get_home_dir();
    let cwd = workspace.display().to_string();
    
    format!(
        r#"====

SYSTEM INFORMATION

Operating System: {}
Default Shell: {}
Home Directory: {}
Current Workspace Directory: {}

The Current Workspace Directory is the active VS Code project directory, and is therefore the default directory for all tool operations. New terminals will be created in the current workspace directory, however if you change directories in a terminal it will then have a different working directory; changing directories in a terminal does not modify the workspace directory, because you do not have access to change the workspace directory. When the user initially gives you a task, a recursive list of all filepaths in the current workspace directory ('{}') will be included in environment_details. This provides an overview of the project's file structure, offering key insights into the project from directory/file names (how developers conceptualize and organize their code) and file extensions (the language used). This can also guide decision-making on which files to explore further. If you need to further explore directories such as outside the current workspace directory, you can use the list_files tool. If you pass 'true' for the recursive parameter, it will list files recursively. Otherwise, it will list files at the top level, which is better suited for generic directories where you don't necessarily need the nested structure, like the Desktop."#,
        os_name, shell, home_dir, cwd, cwd
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_get_os_name() {
        let os_name = get_os_name();
        assert!(!os_name.is_empty());
    }
    
    #[test]
    fn test_get_shell() {
        let shell = get_shell();
        assert!(!shell.is_empty());
    }
    
    #[test]
    fn test_get_home_dir() {
        let home = get_home_dir();
        assert!(!home.is_empty());
    }
    
    #[test]
    fn test_system_info_section() {
        let workspace = PathBuf::from("/home/user/project");
        let section = system_info_section(&workspace);
        
        assert!(section.contains("===="));
        assert!(section.contains("SYSTEM INFORMATION"));
        assert!(section.contains("Operating System:"));
        assert!(section.contains("Default Shell:"));
        assert!(section.contains("Home Directory:"));
        assert!(section.contains("Current Workspace Directory:"));
        assert!(section.contains("/home/user/project"));
    }
}
