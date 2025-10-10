// Diff application strategies module
use super::*;
use anyhow::{Result, bail, anyhow};
use serde::{Serialize, Deserialize};

// Type aliases and structs for compatibility
type UnifiedPatchV2 = DiffPatch;
type PatchResult = super::DiffResult;

#[derive(Debug, Clone)]
pub struct DiffPatch {
    pub hunks: Vec<DiffHunkV2>,
    pub metadata: DiffMetadata,
}

#[derive(Debug, Clone)]
pub struct DiffHunkV2 {
    pub original_start: usize,
    pub original_count: usize,
    pub modified_start: usize,
    pub modified_count: usize,
    pub lines: Vec<DiffLine>,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DiffLine {
    Context(String),
    Addition(String),
    Deletion(String),
}

#[derive(Debug, Clone, Default)]
pub struct DiffMetadata {
    pub line_ending: Option<String>,
    pub has_trailing_newline: bool,
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub line: usize,
    pub message: String,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub resolved: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffStrategy {
    /// Exact match - lines must match exactly
    Exact,
    /// Fuzzy match - allows whitespace differences and minor variations
    Fuzzy,
    /// Force - applies changes regardless of current content
    Force,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictStrategy {
    /// Fail on conflict
    Fail,
    /// Use theirs (incoming changes)
    Theirs,
    /// Use ours (existing content)
    Ours,
    /// Create conflict markers
    Markers,
    /// Interactive resolution (for future UI)
    Interactive,
}

pub struct StrategyHandler;

impl StrategyHandler {
    pub fn new() -> Self {
        Self
    }
    
    pub fn apply(
        &self,
        content: &str,
        patch: &UnifiedPatchV2,
        strategy: DiffStrategy,
        conflict_strategy: ConflictStrategy,
    ) -> Result<PatchResult> {
        match strategy {
            DiffStrategy::Exact => self.apply_exact(content, patch, conflict_strategy),
            DiffStrategy::Fuzzy => self.apply_fuzzy(content, patch, conflict_strategy),
            DiffStrategy::Force => self.apply_force(content, patch),
        }
    }
    
    fn apply_exact(
        &self,
        content: &str,
        patch: &UnifiedPatchV2,
        conflict_strategy: ConflictStrategy,
    ) -> Result<PatchResult> {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut conflicts = Vec::new();
        let mut applied = 0;
        let mut failed = 0;
        
        // Apply hunks in reverse order to maintain line numbers
        let mut hunks = patch.hunks.clone();
        hunks.sort_by(|a, b| b.original_start.cmp(&a.original_start));
        
        for hunk in &hunks {
            match self.apply_hunk_exact(&mut lines, hunk, conflict_strategy) {
                Ok(hunk_conflicts) => {
                    conflicts.extend(hunk_conflicts);
                    applied += 1;
                }
                Err(_) => {
                    failed += 1;
                    if conflict_strategy == ConflictStrategy::Fail {
                        bail!("Failed to apply hunk at line {}", hunk.original_start);
                    }
                }
            }
        }
        
        let final_content = lines.join("\n");
        
        Ok(PatchResult {
            content: final_content,
            lines_added: applied,
            lines_removed: failed,
        })
    }
    
    fn apply_fuzzy(
        &self,
        content: &str,
        patch: &UnifiedPatchV2,
        conflict_strategy: ConflictStrategy,
    ) -> Result<PatchResult> {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut conflicts = Vec::new();
        let mut applied = 0;
        let mut failed = 0;
        
        for hunk in &patch.hunks {
            // Try exact match first
            if let Ok(hunk_conflicts) = self.apply_hunk_exact(&mut lines, hunk, conflict_strategy) {
                conflicts.extend(hunk_conflicts);
                applied += 1;
                continue;
            }
            
            // Try fuzzy match
            if let Some(offset) = self.find_fuzzy_match(&lines, hunk) {
                let adjusted_hunk = self.adjust_hunk(hunk, offset);
                if let Ok(hunk_conflicts) = self.apply_hunk_exact(&mut lines, &adjusted_hunk, conflict_strategy) {
                    conflicts.extend(hunk_conflicts);
                    applied += 1;
                } else {
                    failed += 1;
                }
            } else {
                failed += 1;
            }
        }
        
        let final_content = lines.join("\n");
        
        Ok(PatchResult {
            content: final_content,
            lines_added: 0,
            lines_removed: 0,
        })
    }
    
    fn apply_force(&self, content: &str, patch: &UnifiedPatchV2) -> Result<PatchResult> {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut applied = 0;
        
        for hunk in &patch.hunks {
            // Remove old lines
let start = hunk.original_start.saturating_sub(1);
            let end = (start + hunk.original_count).min(lines.len());
            
            if start < lines.len() {
                lines.drain(start..end);
            }
            
            // Insert new lines
            let mut insertions = Vec::new();
            for line in &hunk.lines {
                match line {
                    DiffLine::Addition(s) | DiffLine::Context(s) => {
                        insertions.push(s.clone());
                    }
                    _ => {}
                }
            }
            
            for (i, line) in insertions.iter().enumerate() {
                if start + i <= lines.len() {
                    lines.insert(start + i, line.clone());
                }
            }
            
            applied += 1;
        }
        
        let final_content = lines.join("\n");
        
        Ok(PatchResult {
            content: final_content,
            lines_added: applied,
            lines_removed: 0,
        })
    }
    
    fn apply_hunk_exact(
        &self,
        lines: &mut Vec<String>,
        hunk: &DiffHunkV2,
        conflict_strategy: ConflictStrategy,
    ) -> Result<Vec<ConflictInfo>> {
        let mut conflicts = Vec::new();
        let start = hunk.original_start.saturating_sub(1);
        
        // Verify context matches
        if !self.verify_context(lines, hunk) {
            bail!("Context mismatch");
        }
        
        // Apply changes
        let mut current_line = start;
        let mut deletions = Vec::new();
        let mut additions = Vec::new();
        
        for line in &hunk.lines {
            match line {
                DiffLine::Context(_) => {
                    current_line += 1;
                }
                DiffLine::Deletion(s) => {
                    deletions.push((current_line, s.clone()));
                    current_line += 1;
                }
                DiffLine::Addition(s) => {
                    additions.push(s.clone());
                }
                // Note: DiffLine doesn't have a Conflict variant
                // Conflicts should be handled at a higher level
            }
        }
        
        // Apply deletions in reverse order
        for (line_num, _) in deletions.iter().rev() {
            if *line_num < lines.len() {
                lines.remove(*line_num);
            }
        }
        
        // Apply additions
        for (i, addition) in additions.iter().enumerate() {
            if start + i <= lines.len() {
                lines.insert(start + i, addition.clone());
            }
        }
        
        Ok(conflicts)
    }
    
    fn verify_context(&self, lines: &[String], hunk: &DiffHunkV2) -> bool {
        let start = hunk.original_start.saturating_sub(1);
        
        // Check context before
        for (i, context_line) in hunk.context_before.iter().enumerate() {
            let line_idx = start.saturating_sub(hunk.context_before.len() - i);
            if line_idx >= lines.len() || lines[line_idx] != *context_line {
                return false;
            }
        }
        
        // Check context after
        for (i, context_line) in hunk.context_after.iter().enumerate() {
            let line_idx = start + hunk.original_count + i;
            if line_idx >= lines.len() || lines[line_idx] != *context_line {
                return false;
            }
        }
        
        true
    }
    
    fn find_fuzzy_match(&self, lines: &[String], hunk: &DiffHunkV2) -> Option<isize> {
        let window = 20; // Search window
        let start = hunk.original_start.saturating_sub(1);
        
        // Search for matching context
        for offset in -window..=window {
            let test_start = (start as isize + offset).max(0) as usize;
            if test_start >= lines.len() {
                continue;
            }
            
            if self.fuzzy_context_match(lines, hunk, test_start) {
                return Some(offset);
            }
        }
        
        None
    }
    
    fn fuzzy_context_match(&self, lines: &[String], hunk: &DiffHunkV2, start: usize) -> bool {
        // Check if deletion lines match (with fuzzy comparison)
        for line in &hunk.lines {
            if let DiffLine::Deletion(expected) = line {
                if start >= lines.len() {
                    return false;
                }
                
                let actual = &lines[start];
                if !utils::fuzzy_line_match(actual, expected) {
                    return false;
                }
            }
        }
        
        true
    }
    
    fn adjust_hunk(&self, hunk: &DiffHunkV2, offset: isize) -> DiffHunkV2 {
        let mut adjusted = hunk.clone();
        adjusted.original_start = (hunk.original_start as isize + offset).max(1) as usize;
        adjusted.modified_start = (hunk.modified_start as isize + offset).max(1) as usize;
        adjusted
    }
}
