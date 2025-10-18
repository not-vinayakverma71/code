//! Rule Helpers
//!
//! Direct 1:1 port from Codex/src/core/context/instructions/rule-helpers.ts
//! Lines 1-102 complete
//!
//! Recursive directory traversal and rule toggle synchronization logic.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Helper: Check if path exists (async wrapper)
async fn file_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

/// Helper: Check if path is a directory (async wrapper)
async fn is_directory(path: &Path) -> bool {
    fs::metadata(path)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

/// Represents toggles for rules (path -> enabled)
pub type RuleToggles = HashMap<String, bool>;

/// Recursively traverses directory and finds all files, including checking for optional whitelisted file extension
///
/// Port of readDirectoryRecursive() from rule-helpers.ts lines 10-32
///
/// # Arguments
/// * `directory_path` - The directory to traverse
/// * `allowed_file_extension` - If non-empty, only include files with this extension (e.g., ".md")
/// * `excluded_paths` - Paths to exclude (not fully implemented yet)
///
/// # Returns
/// List of relative file paths
pub async fn read_directory_recursive(
    directory_path: &Path,
    allowed_file_extension: &str,
    _excluded_paths: &[Vec<String>], // TODO: implement exclusion logic
) -> Vec<String> {
    let mut results = Vec::new();
    let base = directory_path.to_path_buf();
    let mut stack: Vec<PathBuf> = vec![base.clone()];

    while let Some(dir) = stack.pop() {
        let read_dir_result = fs::read_dir(&dir).await;
        if let Ok(mut entries) = read_dir_result {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    // Check file extension filter
                    if !allowed_file_extension.is_empty() {
                        if let Some(ext) = path.extension() {
                            if ext != allowed_file_extension.trim_start_matches('.') {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    // Add relative path from base
                    if let Ok(rel_path) = path.strip_prefix(&base) {
                        results.push(rel_path.to_string_lossy().to_string());
                    }
                }
            }
        } else if let Err(e) = read_dir_result {
            eprintln!("Error reading directory {}: {}", dir.display(), e);
        }
    }

    results
}

/// Gets the up to date toggles
///
/// Port of synchronizeRuleToggles() from rule-helpers.ts lines 37-101
///
/// # Arguments
/// * `rules_directory_path` - Path to rules directory or file
/// * `current_toggles` - Current toggle state
/// * `allowed_file_extension` - Optional file extension filter
/// * `excluded_paths` - Paths to exclude
///
/// # Returns
/// Updated toggle state synchronized with filesystem
pub async fn synchronize_rule_toggles(
    rules_directory_path: &Path,
    current_toggles: RuleToggles,
    allowed_file_extension: &str,
    excluded_paths: &[Vec<String>],
) -> RuleToggles {
    // Create a copy of toggles to modify
    let mut updated_toggles = current_toggles.clone();
    
    let path_exists = file_exists(rules_directory_path).await;
    
    if path_exists {
        let is_dir = is_directory(rules_directory_path).await;
        
        if is_dir {
            // DIRECTORY CASE
            let file_paths = read_directory_recursive(
                rules_directory_path,
                allowed_file_extension,
                excluded_paths,
            )
            .await;
            
            let mut existing_rule_paths = std::collections::HashSet::new();
            
            for file_path in file_paths {
                let rule_file_path = rules_directory_path.join(&file_path);
                let rule_file_path_str = rule_file_path.to_string_lossy().to_string();
                existing_rule_paths.insert(rule_file_path_str.clone());
                
                let path_has_toggle = updated_toggles.contains_key(&rule_file_path_str);
                if !path_has_toggle {
                    updated_toggles.insert(rule_file_path_str, true);
                }
            }
            
            // Clean up toggles for non-existent files
            let keys_to_remove: Vec<String> = updated_toggles
                .keys()
                .filter(|k| !existing_rule_paths.contains(*k))
                .cloned()
                .collect();
            
            for key in keys_to_remove {
                updated_toggles.remove(&key);
            }
        } else {
            // FILE CASE
            let rules_path_str = rules_directory_path.to_string_lossy().to_string();
            
            // Add toggle for this file
            let path_has_toggle = updated_toggles.contains_key(&rules_path_str);
            if !path_has_toggle {
                updated_toggles.insert(rules_path_str.clone(), true);
            }
            
            // Remove toggles for any other paths
            let keys_to_remove: Vec<String> = updated_toggles
                .keys()
                .filter(|k| *k != &rules_path_str)
                .cloned()
                .collect();
            
            for key in keys_to_remove {
                updated_toggles.remove(&key);
            }
        }
    } else {
        // PATH DOESN'T EXIST CASE
        // Clear all toggles since the path doesn't exist
        updated_toggles.clear();
    }
    
    updated_toggles
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_read_directory_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        
        // Create test structure
        fs::create_dir_all(base.join("subdir")).await.unwrap();
        fs::write(base.join("file1.md"), "content").await.unwrap();
        fs::write(base.join("file2.txt"), "content").await.unwrap();
        fs::write(base.join("subdir/file3.md"), "content").await.unwrap();
        
        // Get all files
        let all_files = read_directory_recursive(base, "", &[]).await;
        assert_eq!(all_files.len(), 3);
        
        // Get only .md files
        let md_files = read_directory_recursive(base, ".md", &[]).await;
        assert_eq!(md_files.len(), 2);
        assert!(md_files.iter().any(|f| f.contains("file1.md")));
        assert!(md_files.iter().any(|f| f.contains("file3.md")));
    }
    
    #[tokio::test]
    async fn test_synchronize_rule_toggles_directory() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        
        // Create rules directory
        fs::create_dir_all(base).await.unwrap();
        fs::write(base.join("rule1.md"), "content").await.unwrap();
        fs::write(base.join("rule2.md"), "content").await.unwrap();
        
        // Synchronize with empty toggles
        let toggles = HashMap::new();
        let updated = synchronize_rule_toggles(base, toggles, ".md", &[]).await;
        
        // Should have 2 toggles, both true
        assert_eq!(updated.len(), 2);
        assert!(updated.values().all(|&v| v));
    }
    
    #[tokio::test]
    async fn test_synchronize_rule_toggles_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("single_rule.md");
        
        fs::write(&file_path, "content").await.unwrap();
        
        let toggles = HashMap::new();
        let updated = synchronize_rule_toggles(&file_path, toggles, "", &[]).await;
        
        // Should have exactly 1 toggle
        assert_eq!(updated.len(), 1);
        assert!(updated.values().all(|&v| v));
    }
    
    #[tokio::test]
    async fn test_synchronize_rule_toggles_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist");
        
        let mut toggles = HashMap::new();
        toggles.insert("some_old_path".to_string(), true);
        
        let updated = synchronize_rule_toggles(&nonexistent, toggles, "", &[]).await;
        
        // Should clear all toggles
        assert_eq!(updated.len(), 0);
    }
}
