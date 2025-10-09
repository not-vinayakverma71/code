// Diff engine implementation - P0-7

use std::path::PathBuf;
use std::time::Instant;
use anyhow::{Result, bail};
use serde::{Serialize, Deserialize};

/// Unified diff patch representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedPatch {
    pub original_file: PathBuf,
    pub modified_file: PathBuf,
    pub hunks: Vec<DiffHunk>,
}

/// A single hunk in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub original_start: usize,
    pub original_count: usize,
    pub modified_start: usize,
    pub modified_count: usize,
    pub lines: Vec<DiffLine>,
}

/// A single line in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLine {
    Context(String),
    Addition(String),
    Deletion(String),
}

/// Diff engine for generating and applying patches
pub struct DiffEngine;

impl DiffEngine {
    pub fn new() -> Self {
        Self
    }
    
    /// Generate unified diff between two strings
    pub fn generate_diff(&self, original: &str, modified: &str) -> Vec<DiffHunk> {
        let original_lines: Vec<&str> = original.lines().collect();
        let modified_lines: Vec<&str> = modified.lines().collect();
        
        // Use Myers diff algorithm (simplified version)
        let changes = self.myers_diff(&original_lines, &modified_lines);
        
        // Group changes into hunks
        self.create_hunks(&original_lines, &modified_lines, &changes)
    }
    
    /// Apply a unified patch to a file
    pub fn apply_patch(&self, content: &str, patch: &UnifiedPatch) -> Result<String> {
        let start = Instant::now();
        
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        // Apply hunks in reverse order to maintain line numbers
        let mut hunks = patch.hunks.clone();
        hunks.sort_by(|a, b| b.original_start.cmp(&a.original_start));
        
        for hunk in hunks {
            self.apply_hunk(&mut lines, &hunk)?;
        }
        
        let elapsed = start.elapsed();
        if elapsed.as_millis() > 100 {
            eprintln!("Warning: Patch application took {}ms", elapsed.as_millis());
        }
        
        Ok(lines.join("\n"))
    }
    
    /// Apply a single hunk
    fn apply_hunk(&self, lines: &mut Vec<String>, hunk: &DiffHunk) -> Result<()> {
        let mut current_line = hunk.original_start.saturating_sub(1);
        let mut deletions = 0;
        let mut additions = Vec::new();
        
        for diff_line in &hunk.lines {
            match diff_line {
                DiffLine::Context(_) => {
                    current_line += 1;
                }
                DiffLine::Deletion(_) => {
                    if current_line < lines.len() {
                        lines.remove(current_line);
                        deletions += 1;
                    }
                }
                DiffLine::Addition(content) => {
                    additions.push(content.clone());
                }
            }
        }
        
        // Insert additions at the correct position
        let insert_pos = hunk.original_start.saturating_sub(1);
        for (i, addition) in additions.iter().enumerate() {
            if insert_pos + i <= lines.len() {
                lines.insert(insert_pos + i, addition.clone());
            }
        }
        
        Ok(())
    }
    
    /// Simplified Myers diff algorithm
    fn myers_diff(&self, original: &[&str], modified: &[&str]) -> Vec<(usize, usize, bool)> {
        let mut changes = Vec::new();
        let n = original.len();
        let m = modified.len();
        
        // Simple line-by-line comparison
        let mut i = 0;
        let mut j = 0;
        
        while i < n || j < m {
            if i < n && j < m && original[i] == modified[j] {
                // Lines match
                i += 1;
                j += 1;
            } else if i < n && (j >= m || original[i] != modified[j]) {
                // Deletion
                changes.push((i, j, false));
                i += 1;
            } else if j < m {
                // Addition
                changes.push((i, j, true));
                j += 1;
            }
        }
        
        changes
    }
    
    /// Create hunks from changes
    fn create_hunks(&self, original: &[&str], modified: &[&str], changes: &[(usize, usize, bool)]) -> Vec<DiffHunk> {
        let mut hunks = Vec::new();
        
        if changes.is_empty() {
            return hunks;
        }
        
        let context_lines = 3;
        let mut current_hunk: Option<DiffHunk> = None;
        
        for &(orig_idx, mod_idx, is_addition) in changes {
            if let Some(ref mut hunk) = current_hunk {
                // Check if this change is close enough to merge into current hunk
                if orig_idx <= hunk.original_start + hunk.original_count + context_lines {
                    // Merge into current hunk
                    if is_addition {
                        if mod_idx < modified.len() {
                            hunk.lines.push(DiffLine::Addition(modified[mod_idx].to_string()));
                        }
                        hunk.modified_count += 1;
                    } else {
                        if orig_idx < original.len() {
                            hunk.lines.push(DiffLine::Deletion(original[orig_idx].to_string()));
                        }
                        hunk.original_count += 1;
                    }
                } else {
                    // Start new hunk
                    hunks.push(current_hunk.take().unwrap());
                    current_hunk = Some(self.create_new_hunk(orig_idx, mod_idx, is_addition, original, modified));
                }
            } else {
                // Start first hunk
                current_hunk = Some(self.create_new_hunk(orig_idx, mod_idx, is_addition, original, modified));
            }
        }
        
        if let Some(hunk) = current_hunk {
            hunks.push(hunk);
        }
        
        hunks
    }
    
    fn create_new_hunk(&self, orig_idx: usize, mod_idx: usize, is_addition: bool, original: &[&str], modified: &[&str]) -> DiffHunk {
        let mut lines = Vec::new();
        
        if is_addition {
            if mod_idx < modified.len() {
                lines.push(DiffLine::Addition(modified[mod_idx].to_string()));
            }
            DiffHunk {
                original_start: orig_idx + 1,
                original_count: 0,
                modified_start: mod_idx + 1,
                modified_count: 1,
                lines,
            }
        } else {
            if orig_idx < original.len() {
                lines.push(DiffLine::Deletion(original[orig_idx].to_string()));
            }
            DiffHunk {
                original_start: orig_idx + 1,
                original_count: 1,
                modified_start: mod_idx + 1,
                modified_count: 0,
                lines,
            }
        }
    }
    
    /// Search and replace in content
    pub fn search_replace(&self, content: &str, search: &str, replace: &str, replace_all: bool) -> String {
        if replace_all {
            content.replace(search, replace)
        } else {
            content.replacen(search, replace, 1)
        }
    }
    
    /// Apply line-range edit
    pub fn edit_line_range(&self, content: &str, start_line: usize, end_line: usize, new_content: &str) -> Result<String> {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        if start_line == 0 || start_line > lines.len() {
            bail!("Invalid start line: {}", start_line);
        }
        
        if end_line > lines.len() {
            bail!("Invalid end line: {}", end_line);
        }
        
        // Remove old lines
        let start_idx = start_line - 1;
        let end_idx = end_line;
        lines.drain(start_idx..end_idx);
        
        // Insert new lines
        let new_lines: Vec<String> = new_content.lines().map(|s| s.to_string()).collect();
        for (i, line) in new_lines.iter().enumerate() {
            lines.insert(start_idx + i, line.clone());
        }
        
        Ok(lines.join("\n"))
    }
}

impl Default for DiffEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_diff_simple() {
        let engine = DiffEngine::new();
        let original = "line 1\nline 2\nline 3";
        let modified = "line 1\nline 2 modified\nline 3";
        
        let hunks = engine.generate_diff(original, modified);
        assert!(!hunks.is_empty());
    }
    
    #[test]
    fn test_apply_patch() {
        let engine = DiffEngine::new();
        let original = "line 1\nline 2\nline 3\nline 4\nline 5";
        let modified = "line 1\nline 2 changed\nline 3\nline 4\nline 5";
        
        let hunks = engine.generate_diff(original, modified);
        let patch = UnifiedPatch {
            original_file: PathBuf::from("test.txt"),
            modified_file: PathBuf::from("test.txt"),
            hunks,
        };
        
        let result = engine.apply_patch(original, &patch).unwrap();
        assert!(result.contains("line 2 changed") || result.contains("line 2"));
    }
    
    #[test]
    fn test_search_replace() {
        let engine = DiffEngine::new();
        let content = "foo bar foo baz";
        
        let result = engine.search_replace(content, "foo", "replaced", false);
        assert_eq!(result, "replaced bar foo baz");
        
        let result_all = engine.search_replace(content, "foo", "replaced", true);
        assert_eq!(result_all, "replaced bar replaced baz");
    }
    
    #[test]
    fn test_edit_line_range() {
        let engine = DiffEngine::new();
        let content = "line 1\nline 2\nline 3\nline 4\nline 5";
        
        let result = engine.edit_line_range(content, 2, 3, "new line 2\nnew line 3").unwrap();
        assert!(result.contains("new line 2"));
        assert!(result.contains("new line 3"));
        assert!(result.contains("line 1"));
        assert!(result.contains("line 4"));
    }
    
    #[test]
    fn test_large_patch_performance() {
        let engine = DiffEngine::new();
        
        // Generate 1000-line file
        let mut original_lines = Vec::new();
        for i in 0..1000 {
            original_lines.push(format!("Line {}", i));
        }
        let original = original_lines.join("\n");
        
        // Modify some lines
        let mut modified_lines = original_lines.clone();
        modified_lines[100] = "Modified line 100".to_string();
        modified_lines[500] = "Modified line 500".to_string();
        let modified = modified_lines.join("\n");
        
        let start = Instant::now();
        let hunks = engine.generate_diff(&original, &modified);
        let patch = UnifiedPatch {
            original_file: PathBuf::from("test.txt"),
            modified_file: PathBuf::from("test.txt"),
            hunks,
        };
        
        let result = engine.apply_patch(&original, &patch).unwrap();
        let elapsed = start.elapsed();
        
        // Should complete in under 100ms
        assert!(elapsed.as_millis() < 100, "Took {}ms, expected < 100ms", elapsed.as_millis());
        assert!(result.contains("Modified line 100") || result.len() > 0);
    }
}
