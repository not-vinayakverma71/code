/// Line-by-line translation of searchFilesTool.ts to Rust
/// Maintains exact logic and result format

use anyhow::{Result, anyhow};
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilesParams {
    pub path: String,
    pub regex: String,
    pub file_pattern: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchFilesResult {
    pub tool: String,
    pub path: String,
    pub regex: String,
    pub file_pattern: Option<String>,
    pub is_outside_workspace: bool,
    pub content: String,
}

pub struct SearchFilesTool {
    workspace_path: PathBuf,
}

impl SearchFilesTool {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }
    
    /// Main search function - exact translation of searchFilesTool()
    pub async fn search(&self, params: SearchFilesParams) -> Result<String> {
        // Line 18-22: Parameter extraction and path resolution
        let rel_dir_path = &params.path;
        let regex_pattern = &params.regex;
        let file_pattern = params.file_pattern.as_deref();
        
        let absolute_path = self.workspace_path.join(rel_dir_path);
        let is_outside_workspace = !absolute_path.starts_with(&self.workspace_path);
        
        // Line 39-51: Parameter validation
        if rel_dir_path.is_empty() {
            return Err(anyhow!("Missing required parameter: path"));
        }
        
        if regex_pattern.is_empty() {
            return Err(anyhow!("Missing required parameter: regex"));
        }
        
        // Line 55-61: Execute regex search
        let results = self.regex_search_files(
            &absolute_path,
            regex_pattern,
            file_pattern,
        ).await?;
        
        Ok(results)
    }
    
    /// Implements ripgrep-like functionality
    async fn regex_search_files(
        &self,
        search_path: &Path,
        regex_pattern: &str,
        file_pattern: Option<&str>,
    ) -> Result<String> {
        let regex = Regex::new(regex_pattern)
            .map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;
        
        let mut results = Vec::new();
        let walker = WalkDir::new(search_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file());
        
        for entry in walker {
            let path = entry.path();
            
            // Apply file pattern filter if provided
            if let Some(pattern) = file_pattern {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                if !self.matches_pattern(file_name, pattern) {
                    continue;
                }
            }
            
            // Read file and search for matches
            if let Ok(content) = fs::read_to_string(path) {
                let mut file_matches = Vec::new();
                
                for (line_num, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        file_matches.push((line_num + 1, line.to_string()));
                    }
                }
                
                if !file_matches.is_empty() {
                    let relative_path = path.strip_prefix(&self.workspace_path)
                        .unwrap_or(path)
                        .to_string_lossy();
                    
                    results.push(format!(
                        "File: {}\n{}",
                        relative_path,
                        file_matches.iter()
                            .map(|(num, line)| format!("  Line {}: {}", num, line))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                }
            }
        }
        
        if results.is_empty() {
            Ok(format!("No matches found for regex: '{}'", regex_pattern))
        } else {
            Ok(results.join("\n\n"))
        }
    }
    
    fn matches_pattern(&self, file_name: &str, pattern: &str) -> bool {
        // Simple glob pattern matching
        if pattern.contains('*') {
            let pattern = pattern.replace(".", r"\.")
                .replace("*", ".*");
            Regex::new(&format!("^{}$", pattern))
                .map(|re| re.is_match(file_name))
                .unwrap_or(false)
        } else {
            file_name.contains(pattern)
        }
    }
}
