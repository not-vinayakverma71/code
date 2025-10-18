//! Custom Instructions Loader
//!
//! 1:1 Translation from Codex `src/core/prompts/sections/custom-instructions.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/sections/custom-instructions.ts (lines 1-472)

use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::pin::Pin;
use std::future::Future;
use tokio::fs;

use crate::core::prompt::errors::{PromptError, PromptResult};
use crate::core::prompt::settings::SystemPromptSettings;
use crate::core::tools::fs::{ensure_workspace_path, utils::{FileEncoding, detect_encoding}};

/// Maximum depth for symlink resolution to prevent cycles
const MAX_DEPTH: usize = 5;

/// File info with original and resolved paths for sorting
#[derive(Debug, Clone)]
struct FileInfo {
    original_path: PathBuf,
    resolved_path: PathBuf,
}

/// Safely read a file and return its trimmed content
///
/// Translation of safeReadFile() from custom-instructions.ts (lines 32-43)
async fn safe_read_file(file_path: &Path, workspace: &Path) -> PromptResult<String> {
    // Security: ensure file is within workspace
    ensure_workspace_path(workspace, file_path)
        .map_err(|e| PromptError::OutsideWorkspace(format!("{}: {}", file_path.display(), e)))?;
    
    // Check if file exists
    if !file_path.exists() {
        return Ok(String::new());
    }
    
    // Check if it's a directory
    if file_path.is_dir() {
        return Ok(String::new());
    }
    
    // Read file content
    match fs::read_to_string(file_path).await {
        Ok(content) => Ok(content.trim().to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(PromptError::RuleLoadError(e)),
    }
}

/// Check if a directory exists
///
/// Translation of directoryExists() from custom-instructions.ts (lines 48-55)
async fn directory_exists(dir_path: &Path) -> bool {
    match fs::metadata(dir_path).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

/// Resolve a symbolic link recursively
///
/// Translation of resolveSymLink() from custom-instructions.ts (lines 86-122)
async fn resolve_symlink(
    symlink_path: &Path,
    file_info: &mut Vec<FileInfo>,
    depth: usize,
    workspace: &Path,
) -> PromptResult<()> {
    // Avoid cyclic symlinks
    if depth > MAX_DEPTH {
        return Err(PromptError::SymlinkCycle(symlink_path.display().to_string()));
    }
    
    // Read the symlink target
    let link_target = fs::read_link(symlink_path).await?;
    
    // Resolve the target path (relative to the symlink location)
    let resolved_target = if link_target.is_absolute() {
        link_target
    } else {
        symlink_path.parent()
            .unwrap_or(Path::new("."))
            .join(link_target)
    };
    
    // Security check
    ensure_workspace_path(workspace, &resolved_target)
        .map_err(|e| PromptError::OutsideWorkspace(format!("{}: {}", resolved_target.display(), e)))?;
    
    // Check what the target is
    let metadata = fs::metadata(&resolved_target).await?;
    
    if metadata.is_file() {
        // For symlinks to files, store symlink path as original and target as resolved
        file_info.push(FileInfo {
            original_path: symlink_path.to_path_buf(),
            resolved_path: resolved_target,
        });
    } else if metadata.is_dir() {
        // Recursively process directory
        collect_files_from_directory(&resolved_target, file_info, depth + 1, workspace).await?;
    }
    
    Ok(())
}

/// Recursively collect files from a directory
fn collect_files_from_directory<'a>(
    dir_path: &'a Path,
    file_info: &'a mut Vec<FileInfo>,
    depth: usize,
    workspace: &'a Path,
) -> Pin<Box<dyn Future<Output = PromptResult<()>> + 'a>> {
    Box::pin(async move {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    
    let mut entries = fs::read_dir(dir_path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;
        
        if metadata.is_file() {
            file_info.push(FileInfo {
                original_path: path.clone(),
                resolved_path: path,
            });
        } else if metadata.is_symlink() {
            // Resolve symlink
            if let Err(e) = resolve_symlink(&path, file_info, depth + 1, workspace).await {
                // Skip invalid symlinks
                tracing::debug!("Skipping invalid symlink {:?}: {}", path, e);
            }
        } else if metadata.is_dir() {
            // Recursively process subdirectory
            collect_files_from_directory(&path, file_info, depth + 1, workspace).await?;
        }
    }
    
    Ok(())
    })
}

/// Read all text files from a directory in alphabetical order
///
/// Translation of readTextFilesFromDirectory() from custom-instructions.ts (lines 127-192)
async fn read_text_files_from_directory(
    dir_path: &Path,
    workspace: &Path,
) -> PromptResult<Vec<(String, String)>> {
    let mut file_info = Vec::new();
    
    // Collect all files (including through symlinks)
    collect_files_from_directory(dir_path, &mut file_info, 0, workspace).await?;
    
    // Read and filter files
    let mut files = Vec::new();
    
    for info in file_info {
        // Check if it's a file
        if !info.resolved_path.is_file() {
            continue;
        }
        
        // Filter out cache files
        if !should_include_rule_file(&info.resolved_path) {
            continue;
        }
        
        // Check if binary using encoding detection
        let encoding = detect_encoding(&info.resolved_path)?;
        if encoding == FileEncoding::Binary {
            continue;
        }
        
        // Read content
        let content = safe_read_file(&info.resolved_path, workspace).await?;
        
        if !content.is_empty() {
            files.push((
                info.resolved_path.display().to_string(),
                content,
                info.original_path.display().to_string(), // For sorting
            ));
        }
    }
    
    // Sort alphabetically by original filename (case-insensitive)
    files.sort_by(|a, b| {
        let name_a = Path::new(&a.2)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let name_b = Path::new(&b.2)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        name_a.cmp(&name_b)
    });
    
    // Return (filename, content) pairs
    Ok(files.into_iter().map(|(f, c, _)| (f, c)).collect())
}

/// Format content from multiple files with filenames as headers
///
/// Translation of formatDirectoryContent() from custom-instructions.ts (lines 197-205)
fn format_directory_content(files: &[(String, String)]) -> String {
    if files.is_empty() {
        return String::new();
    }
    
    files
        .iter()
        .map(|(filename, content)| format!("# Rules from {}:\n{}", filename, content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Load rule files from .kilocode/rules/ directories
///
/// Translation of loadRuleFiles() from custom-instructions.ts (lines 211-250)
async fn load_rule_files(workspace: &Path) -> PromptResult<String> {
    let mut rules = Vec::new();
    
    // Check .kilocode/rules/ directory
    let kilocode_rules = workspace.join(".kilocode").join("rules");
    if directory_exists(&kilocode_rules).await {
        let files = read_text_files_from_directory(&kilocode_rules, workspace).await?;
        if !files.is_empty() {
            let content = format_directory_content(&files);
            rules.push(content);
        }
    }
    
    // If we found rules, return them
    if !rules.is_empty() {
        return Ok(format!("\n{}", rules.join("\n\n")));
    }
    
    // Fall back to legacy files
    let legacy_files = [".kilocoderules", ".roorules", ".clinerules"];
    
    for file in &legacy_files {
        let file_path = workspace.join(file);
        let content = safe_read_file(&file_path, workspace).await?;
        if !content.is_empty() {
            if *file != ".kilocoderules" {
                tracing::warn!("Loading legacy rules from {}, consider moving to .kilocode/rules/", file);
            }
            return Ok(format!("\n# Rules from {}:\n{}\n", file, content));
        }
    }
    
    Ok(String::new())
}

/// Load AGENTS.md or AGENT.md file from the project root
///
/// Translation of loadAgentRulesFile() from custom-instructions.ts (lines 256-295)
async fn load_agent_rules_file(workspace: &Path) -> PromptResult<String> {
    let filenames = ["AGENTS.md", "AGENT.md"];
    
    for filename in &filenames {
        let agent_path = workspace.join(filename);
        
        // Check if file exists
        if !agent_path.exists() {
            continue;
        }
        
        let mut resolved_path = agent_path.clone();
        
        // Handle symlinks
        let metadata = fs::symlink_metadata(&agent_path).await?;
        if metadata.is_symlink() {
            let mut file_info = Vec::new();
            if resolve_symlink(&agent_path, &mut file_info, 0, workspace).await.is_ok() {
                if !file_info.is_empty() {
                    resolved_path = file_info[0].resolved_path.clone();
                }
            }
        }
        
        // Read content
        let content = safe_read_file(&resolved_path, workspace).await?;
        if !content.is_empty() {
            return Ok(format!("# Agent Rules Standard ({}):\n{}", filename, content));
        }
    }
    
    Ok(String::new())
}

/// Add custom instructions to the prompt
///
/// Translation of addCustomInstructions() from custom-instructions.ts (lines 297-430)
///
/// # Arguments
///
/// * `mode_custom_instructions` - Instructions specific to the current mode
/// * `global_custom_instructions` - User's global custom instructions
/// * `workspace` - Workspace root directory
/// * `mode` - Mode slug (e.g., "code", "architect")
/// * `language` - User's language preference
/// * `roo_ignore_instructions` - Instructions from .rooignore
/// * `settings` - System prompt settings
///
/// # Returns
///
/// Formatted custom instructions section
pub async fn add_custom_instructions(
    mode_custom_instructions: &str,
    global_custom_instructions: &str,
    workspace: &Path,
    mode: &str,
    language: Option<&str>,
    roo_ignore_instructions: Option<&str>,
    settings: &SystemPromptSettings,
) -> PromptResult<String> {
    let mut sections = Vec::new();
    
    // Load mode-specific rules
    let mut mode_rule_content = String::new();
    let mut used_rule_file = String::new();
    
    if !mode.is_empty() {
        // Check for .kilocode/rules-{mode}/ directory
        let mode_rules_dir = workspace.join(".kilocode").join(format!("rules-{}", mode));
        if directory_exists(&mode_rules_dir).await {
            let files = read_text_files_from_directory(&mode_rules_dir, workspace).await?;
            if !files.is_empty() {
                mode_rule_content = format!("\n{}", format_directory_content(&files));
                used_rule_file = format!("rules-{} directories", mode);
            }
        } else {
            // Fall back to legacy file
            let legacy_file = workspace.join(format!(".kilocoderules-{}", mode));
            let content = safe_read_file(&legacy_file, workspace).await?;
            if !content.is_empty() {
                mode_rule_content = content;
                used_rule_file = format!(".kilocoderules-{}", mode);
            }
        }
    }
    
    // Add language preference if provided
    if let Some(lang) = language {
        sections.push(format!(
            "Language Preference:\nYou should always speak and think in the \"{}\" language unless the user gives you instructions below to do otherwise.",
            lang
        ));
    }
    
    // Add global instructions first
    if !global_custom_instructions.trim().is_empty() {
        sections.push(format!("Global Instructions:\n{}", global_custom_instructions.trim()));
    }
    
    // Add mode-specific instructions after
    if !mode_custom_instructions.trim().is_empty() {
        sections.push(format!("Mode-specific Instructions:\n{}", mode_custom_instructions.trim()));
    }
    
    // Add rules - include both mode-specific and generic rules
    let mut rules = Vec::new();
    
    // Add mode-specific rules first if they exist
    if !mode_rule_content.trim().is_empty() {
        if used_rule_file.contains(&format!("rules-{}", mode)) {
            rules.push(mode_rule_content.trim().to_string());
        } else {
            rules.push(format!("# Rules from {}:\n{}", used_rule_file, mode_rule_content));
        }
    }
    
    // Add rooignore instructions
    if let Some(roo_ignore) = roo_ignore_instructions {
        if !roo_ignore.trim().is_empty() {
            rules.push(roo_ignore.trim().to_string());
        }
    }
    
    // Add AGENTS.md content if enabled (default: true)
    if settings.use_agent_rules {
        let agent_rules = load_agent_rules_file(workspace).await?;
        if !agent_rules.trim().is_empty() {
            rules.push(agent_rules.trim().to_string());
        }
    }
    
    // Add generic rules
    let generic_rules = load_rule_files(workspace).await?;
    if !generic_rules.trim().is_empty() {
        rules.push(generic_rules.trim().to_string());
    }
    
    if !rules.is_empty() {
        sections.push(format!("Rules:\n\n{}", rules.join("\n\n")));
    }
    
    let joined_sections = sections.join("\n\n");
    
    if joined_sections.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!(
            r#"
====

USER'S CUSTOM INSTRUCTIONS

The following additional instructions are provided by the user, and should be followed to the best of your ability without interfering with the TOOL USE guidelines.

{}"#,
            joined_sections
        ))
    }
}

/// Check if a file should be included in rule compilation
///
/// Translation of shouldIncludeRuleFile() from custom-instructions.ts (lines 436-471)
fn should_include_rule_file(path: &Path) -> bool {
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    let cache_patterns = [
        ".DS_Store", ".bak", ".cache", ".crdownload", ".db", ".dmp",
        ".dump", ".eslintcache", ".lock", ".log", ".old", ".part",
        ".partial", ".pyc", ".pyo", ".stackdump", ".swo", ".swp",
        ".temp", ".tmp", "Thumbs.db",
    ];
    
    for pattern in &cache_patterns {
        if pattern.starts_with('.') {
            // Extension pattern
            if filename.ends_with(pattern) {
                return false;
            }
        } else {
            // Exact match
            if filename == *pattern {
                return false;
            }
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_safe_read_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");
        
        let content = safe_read_file(&file_path, temp_dir.path()).await.unwrap();
        assert_eq!(content, "");
    }
    
    #[tokio::test]
    async fn test_safe_read_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "  content  \n").await.unwrap();
        
        let content = safe_read_file(&file_path, temp_dir.path()).await.unwrap();
        assert_eq!(content, "content");
    }
    
    #[tokio::test]
    async fn test_directory_exists() {
        let temp_dir = TempDir::new().unwrap();
        assert!(directory_exists(temp_dir.path()).await);
        
        let nonexistent = temp_dir.path().join("nonexistent");
        assert!(!directory_exists(&nonexistent).await);
    }
    
    #[test]
    fn test_should_include_rule_file() {
        assert!(should_include_rule_file(Path::new("rules.md")));
        assert!(should_include_rule_file(Path::new("custom.txt")));
        
        assert!(!should_include_rule_file(Path::new(".DS_Store")));
        assert!(!should_include_rule_file(Path::new("file.log")));
        assert!(!should_include_rule_file(Path::new("cache.cache")));
        assert!(!should_include_rule_file(Path::new("Thumbs.db")));
    }
    
    #[tokio::test]
    async fn test_load_agent_rules_file() {
        let temp_dir = TempDir::new().unwrap();
        
        // No AGENTS.md
        let result = load_agent_rules_file(temp_dir.path()).await.unwrap();
        assert_eq!(result, "");
        
        // With AGENTS.md
        fs::write(temp_dir.path().join("AGENTS.md"), "Test agent rules").await.unwrap();
        let result = load_agent_rules_file(temp_dir.path()).await.unwrap();
        assert!(result.contains("Agent Rules Standard"));
        assert!(result.contains("Test agent rules"));
    }
    
    #[tokio::test]
    async fn test_add_custom_instructions_empty() {
        let temp_dir = TempDir::new().unwrap();
        let settings = SystemPromptSettings::default();
        
        let result = add_custom_instructions(
            "",
            "",
            temp_dir.path(),
            "code",
            None,
            None,
            &settings,
        ).await.unwrap();
        
        assert_eq!(result, "");
    }
    
    #[tokio::test]
    async fn test_add_custom_instructions_with_language() {
        let temp_dir = TempDir::new().unwrap();
        let settings = SystemPromptSettings::default();
        
        let result = add_custom_instructions(
            "",
            "",
            temp_dir.path(),
            "code",
            Some("English"),
            None,
            &settings,
        ).await.unwrap();
        
        assert!(result.contains("Language Preference"));
        assert!(result.contains("English"));
    }
    
    #[tokio::test]
    async fn test_add_custom_instructions_layered() {
        let temp_dir = TempDir::new().unwrap();
        let settings = SystemPromptSettings::default();
        
        // Create AGENTS.md
        fs::write(temp_dir.path().join("AGENTS.md"), "Agent rules content").await.unwrap();
        
        let result = add_custom_instructions(
            "Mode instructions here",
            "Global instructions here",
            temp_dir.path(),
            "code",
            Some("English"),
            Some("RooIgnore instructions"),
            &settings,
        ).await.unwrap();
        
        assert!(result.contains("USER'S CUSTOM INSTRUCTIONS"));
        assert!(result.contains("Language Preference"));
        assert!(result.contains("Global Instructions"));
        assert!(result.contains("Mode-specific Instructions"));
        assert!(result.contains("Rules:"));
        assert!(result.contains("RooIgnore instructions"));
        assert!(result.contains("Agent rules content"));
    }
}
