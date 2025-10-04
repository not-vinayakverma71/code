// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// EXACT Translation of searchFilesTool.ts (Lines 1-79)

use crate::error::{Error, Result};
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// Lines 4-5: Tool structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    pub params: SearchParams,
    pub partial: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParams {
    pub path: Option<String>,      // Line 18: relDirPath
    pub regex: Option<String>,     // Line 19: regex
    pub file_pattern: Option<String>, // Line 20: filePattern
}

// Line 5: ClineSayTool equivalent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineSayTool {
    pub tool: String,
    pub path: Option<String>,
    pub regex: Option<String>,
    #[serde(rename = "filePattern")]
    pub file_pattern: Option<String>,
    #[serde(rename = "isOutsideWorkspace")]
    pub is_outside_workspace: bool,
    pub content: Option<String>,
}

pub struct SearchFilesTool {
    cwd: PathBuf,
    workspace_path: PathBuf,
}

impl SearchFilesTool {
    pub fn new(cwd: PathBuf, workspace_path: PathBuf) -> Self {
        Self {
            cwd,
            workspace_path,
        }
    }
    
    // Lines 10-78: Main function implementation
    pub async fn search_files_tool(
        &self,
        block: ToolUse,
        consecutive_mistake_count: &mut usize,
    ) -> Result<String> {
        // Line 18-20: Parameter extraction
        let rel_dir_path = block.params.path.clone();
        let regex = block.params.regex.clone();
        let file_pattern = block.params.file_pattern.clone();
        
        // Line 22-23: Path resolution
        let absolute_path = if let Some(ref path) = rel_dir_path {
            self.cwd.join(path)
        } else {
            self.cwd.clone()
        };
        let is_outside_workspace = self.is_path_outside_workspace(&absolute_path);
        
        // Lines 25-31: Shared message properties
        let mut shared_message_props = ClineSayTool {
            tool: "searchFiles".to_string(),
            path: self.get_readable_path(Self::remove_closing_tag("path", rel_dir_path.clone())),
            regex: Self::remove_closing_tag("regex", regex.clone()),
            file_pattern: Self::remove_closing_tag("file_pattern", file_pattern.clone()),
            is_outside_workspace,
            content: None,
        };
        
        // Line 33: Try block
        // Lines 34-37: Handle partial
        if block.partial {
            shared_message_props.content = Some(String::new());
            return Ok(serde_json::to_string(&shared_message_props)?);
        }
        
        // Lines 39-44: Validate path parameter
        let rel_dir_path = match rel_dir_path {
            Some(p) if !p.is_empty() => p,
            _ => {
                *consecutive_mistake_count += 1;
                // Line 41: record_tool_error("search_files")
                return Err(Error::Runtime {
                    message: "Missing required parameter 'path' for search_files".to_string()
                });
            }
        };
        
        // Lines 46-51: Validate regex parameter
        let regex_str = match regex {
            Some(r) if !r.is_empty() => r,
            _ => {
                *consecutive_mistake_count += 1;
                // Line 48: record_tool_error("search_files")
                return Err(Error::Runtime {
                    message: "Missing required parameter 'regex' for search_files".to_string()
                });
            }
        };
        
        // Line 53: Reset consecutive mistake count
        *consecutive_mistake_count = 0;
        
        // Lines 55-61: Execute regex search
        let results = self.regex_search_files(
            &self.cwd,
            &absolute_path,
            &regex_str,
            file_pattern.as_deref(),
        ).await?;
        
        // Line 70: Push result
        Ok(results)
    }
    
    // Line 55: regexSearchFiles implementation
    async fn regex_search_files(
        &self,
        cwd: &Path,
        absolute_path: &Path,
        regex_pattern: &str,
        file_pattern: Option<&str>,
    ) -> Result<String> {
        // Compile regex
        let re = Regex::new(regex_pattern)
            .map_err(|e| Error::Runtime {
                message: format!("Invalid regex pattern: {}", e)
            })?;
        
        let mut results = Vec::new();
        let mut files_searched = 0;
        let mut matches_found = 0;
        
        // Walk directory tree
        for entry in WalkDir::new(absolute_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Skip directories
            if !entry.file_type().is_file() {
                continue;
            }
            
            // Apply file pattern filter if provided
            if let Some(pattern) = file_pattern {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                // Simple glob pattern matching
                if !Self::matches_pattern(file_name, pattern) {
                    continue;
                }
            }
            
            files_searched += 1;
            
            // Read file and search for matches
            if let Ok(content) = std::fs::read_to_string(path) {
                let mut file_has_match = false;
                
                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        if !file_has_match {
                            // First match in this file - add file header
                            let relative_path = path.strip_prefix(cwd)
                                .unwrap_or(path)
                                .to_string_lossy();
                            results.push(format!("\n{}", relative_path));
                            file_has_match = true;
                        }
                        
                        // Add matching line
                        results.push(format!("  {}: {}", line_num + 1, line.trim()));
                        matches_found += 1;
                    }
                }
            }
        }
        
        // Format final results
        if results.is_empty() {
            Ok(format!(
                "No matches found for regex '{}' in {} files",
                regex_pattern, files_searched
            ))
        } else {
            Ok(format!(
                "Found {} matches in {} files:{}",
                matches_found,
                files_searched,
                results.join("\n")
            ))
        }
    }
    
    // Helper functions
    fn remove_closing_tag(_tag: &str, value: Option<String>) -> Option<String> {
        value.map(|v| v.trim().to_string())
    }
    
    fn get_readable_path(&self, path: Option<String>) -> Option<String> {
        path.map(|p| {
            if let Ok(relative) = Path::new(&p).strip_prefix(&self.cwd) {
                relative.to_string_lossy().to_string()
            } else {
                p
            }
        })
    }
    
    fn is_path_outside_workspace(&self, path: &Path) -> bool {
        !path.starts_with(&self.workspace_path)
    }
    
    fn matches_pattern(file_name: &str, pattern: &str) -> bool {
        // Simple glob pattern matching
        if pattern.contains('*') {
            let pattern = pattern.replace("*", ".*");
            if let Ok(re) = Regex::new(&format!("^{}$", pattern)) {
                return re.is_match(file_name);
            }
        }
        file_name.contains(pattern)
    }
}
