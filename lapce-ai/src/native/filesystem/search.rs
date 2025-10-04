// Native file search operations - Direct I/O without MCP overhead
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use regex::Regex;
use tokio::fs;
use std::io;

/// Search files by pattern in directory
pub async fn search_files(dir: &Path, pattern: &str, recursive: bool) -> io::Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    let pattern_lower = pattern.to_lowercase();
    
    for entry in WalkDir::new(dir).into_iter().filter_entry(|entry| recursive || entry.depth() == 1) {
        if let Ok(entry) = entry {
            if entry.file_name().to_string_lossy().to_lowercase().contains(&pattern_lower) {
                results.push(entry.path().to_path_buf());
            }
        }
    }
    
    Ok(results)
}
