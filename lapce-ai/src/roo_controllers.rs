// RooIgnore/RooProtected Controllers - CHUNK-03: T15
// Pattern-based gating for file and context references

use std::path::{Path, PathBuf};
use std::collections::HashSet;
use anyhow::{Result, Context};
use tracing::{debug, warn};

/// RooIgnore controller for filtering files
pub struct RooIgnoreController {
    /// Working directory
    cwd: PathBuf,
    
    /// Ignored patterns
    ignore_patterns: HashSet<String>,
    
    /// Negation patterns (force include)
    negation_patterns: HashSet<String>,
}

impl RooIgnoreController {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            ignore_patterns: HashSet::new(),
            negation_patterns: HashSet::new(),
        }
    }
    
    /// Initialize from .rooignore file
    pub async fn initialize(&mut self) -> Result<()> {
        let rooignore_path = self.cwd.join(".rooignore");
        
        if !rooignore_path.exists() {
            debug!("No .rooignore file found");
            return Ok(());
        }
        
        let content = tokio::fs::read_to_string(&rooignore_path).await
            .context("Failed to read .rooignore")?;
        
        self.parse_patterns(&content);
        debug!("Loaded {} ignore patterns", self.ignore_patterns.len());
        
        Ok(())
    }
    
    /// Parse patterns from .rooignore content
    fn parse_patterns(&mut self, content: &str) {
        for line in content.lines() {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            
            // Check for negation pattern
            if trimmed.starts_with('!') {
                self.negation_patterns.insert(trimmed[1..].to_string());
            } else {
                self.ignore_patterns.insert(trimmed.to_string());
            }
        }
    }
    
    /// Check if a file should be ignored
    pub fn should_ignore(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        
        // Check negation patterns first (force include)
        for pattern in &self.negation_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return false;
            }
        }
        
        // Check ignore patterns
        for pattern in &self.ignore_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// Simple glob-like pattern matching
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        // Simple wildcard matching
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                return path.starts_with(parts[0]) && path.ends_with(parts[1]);
            }
        }
        
        // Directory matching
        if pattern.ends_with('/') {
            return path.starts_with(pattern);
        }
        
        // Exact match
        path == pattern || path.ends_with(&format!("/{}", pattern))
    }
    
    /// Filter a list of files
    pub fn filter_files(&self, files: Vec<PathBuf>) -> Vec<PathBuf> {
        files.into_iter()
            .filter(|f| !self.should_ignore(f))
            .collect()
    }
    
    /// Add a pattern at runtime
    pub fn add_pattern(&mut self, pattern: String) {
        if pattern.starts_with('!') {
            self.negation_patterns.insert(pattern[1..].to_string());
        } else {
            self.ignore_patterns.insert(pattern);
        }
    }
}

/// RooProtected controller for protecting sensitive files
pub struct RooProtectedController {
    /// Working directory
    cwd: PathBuf,
    
    /// Protected patterns
    protected_patterns: HashSet<String>,
}

impl RooProtectedController {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            cwd,
            protected_patterns: HashSet::new(),
        }
    }
    
    /// Initialize from .rooprotected file
    pub async fn initialize(&mut self) -> Result<()> {
        let protected_path = self.cwd.join(".rooprotected");
        
        if !protected_path.exists() {
            debug!("No .rooprotected file found");
            return Ok(());
        }
        
        let content = tokio::fs::read_to_string(&protected_path).await
            .context("Failed to read .rooprotected")?;
        
        self.parse_patterns(&content);
        debug!("Loaded {} protected patterns", self.protected_patterns.len());
        
        Ok(())
    }
    
    /// Parse patterns
    fn parse_patterns(&mut self, content: &str) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            self.protected_patterns.insert(trimmed.to_string());
        }
    }
    
    /// Check if a file is protected
    pub fn is_protected(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        
        for pattern in &self.protected_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return true;
            }
        }
        
        false
    }
    
    /// Simple pattern matching
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                return path.starts_with(parts[0]) && path.ends_with(parts[1]);
            }
        }
        
        path == pattern || path.ends_with(&format!("/{}", pattern))
    }
    
    /// Require approval for protected file operation
    pub fn require_approval(&self, file_path: &Path, operation: &str) -> bool {
        if self.is_protected(file_path) {
            warn!("Protected file operation requires approval: {} on {:?}", 
                operation, file_path);
            true
        } else {
            false
        }
    }
    
    /// Add a protected pattern
    pub fn add_pattern(&mut self, pattern: String) {
        self.protected_patterns.insert(pattern);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rooignore_pattern_matching() {
        let mut controller = RooIgnoreController::new(PathBuf::from("/tmp"));
        controller.add_pattern("*.log".to_string());
        controller.add_pattern("node_modules/".to_string());
        
        assert!(controller.should_ignore(Path::new("test.log")));
        assert!(controller.should_ignore(Path::new("node_modules/foo.js")));
        assert!(!controller.should_ignore(Path::new("src/main.rs")));
    }
    
    #[test]
    fn test_rooignore_negation() {
        let mut controller = RooIgnoreController::new(PathBuf::from("/tmp"));
        controller.add_pattern("*.log".to_string());
        controller.add_pattern("!important.log".to_string());
        
        assert!(controller.should_ignore(Path::new("debug.log")));
        assert!(!controller.should_ignore(Path::new("important.log")));
    }
    
    #[test]
    fn test_rooignore_filter_files() {
        let mut controller = RooIgnoreController::new(PathBuf::from("/tmp"));
        controller.add_pattern("*.txt".to_string());
        
        let files = vec![
            PathBuf::from("a.txt"),
            PathBuf::from("b.rs"),
            PathBuf::from("c.txt"),
        ];
        
        let filtered = controller.filter_files(files);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], PathBuf::from("b.rs"));
    }
    
    #[test]
    fn test_rooprotected_matching() {
        let mut controller = RooProtectedController::new(PathBuf::from("/tmp"));
        controller.add_pattern("*.env".to_string());
        controller.add_pattern(".ssh/*".to_string());
        
        assert!(controller.is_protected(Path::new(".env")));
        assert!(controller.is_protected(Path::new(".ssh/id_rsa")));
        assert!(!controller.is_protected(Path::new("config.json")));
    }
    
    #[test]
    fn test_rooprotected_approval() {
        let mut controller = RooProtectedController::new(PathBuf::from("/tmp"));
        controller.add_pattern("secrets/*".to_string());
        
        assert!(controller.require_approval(Path::new("secrets/api_key"), "write"));
        assert!(!controller.require_approval(Path::new("public/data"), "write"));
    }
    
    #[tokio::test]
    async fn test_rooignore_initialization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let rooignore_path = temp_dir.path().join(".rooignore");
        
        tokio::fs::write(&rooignore_path, "*.log\nnode_modules/\n!important.log\n# comment\n")
            .await
            .unwrap();
        
        let mut controller = RooIgnoreController::new(temp_dir.path().to_path_buf());
        controller.initialize().await.unwrap();
        
        assert_eq!(controller.ignore_patterns.len(), 2);
        assert_eq!(controller.negation_patterns.len(), 1);
    }
    
    #[tokio::test]
    async fn test_rooprotected_initialization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let protected_path = temp_dir.path().join(".rooprotected");
        
        tokio::fs::write(&protected_path, "*.env\n.ssh/*\n# sensitive files\n")
            .await
            .unwrap();
        
        let mut controller = RooProtectedController::new(temp_dir.path().to_path_buf());
        controller.initialize().await.unwrap();
        
        assert_eq!(controller.protected_patterns.len(), 2);
    }
}
