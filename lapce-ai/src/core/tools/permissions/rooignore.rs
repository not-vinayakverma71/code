// .rooignore implementation - gitignore-style path filtering - P0-1: Implement .rooignore

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use globset::{Glob, GlobSet, GlobSetBuilder};
use anyhow::{Result, Context};

/// .rooignore manager with caching and gitignore-style patterns
pub struct RooIgnore {
    /// Workspace root directory
    workspace: PathBuf,
    
    /// Compiled glob patterns
    patterns: Arc<RwLock<GlobSet>>,
    
    /// Cache of evaluated paths (path -> allowed)
    cache: Arc<RwLock<HashMap<PathBuf, bool>>>,
    
    /// Negation patterns (start with !)
    negations: Arc<RwLock<GlobSet>>,
}

impl RooIgnore {
    /// Create new RooIgnore instance for a workspace
    pub fn new(workspace: PathBuf) -> Self {
        let mut instance = Self {
            workspace: workspace.clone(),
            patterns: Arc::new(RwLock::new(GlobSet::empty())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            negations: Arc::new(RwLock::new(GlobSet::empty())),
        };
        
        // Load .rooignore file if it exists
        let rooignore_path = workspace.join(".rooignore");
        if rooignore_path.exists() {
            let _ = instance.load_from_file(&rooignore_path);
        }
        
        instance
    }
    
    /// Load patterns from .rooignore file
    pub fn load_from_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)
            .context("Failed to read .rooignore file")?;
        
        self.load_from_string(&content)
    }
    
    /// Load patterns from string content
    pub fn load_from_string(&mut self, content: &str) -> Result<()> {
        let mut builder = GlobSetBuilder::new();
        let mut negation_builder = GlobSetBuilder::new();
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if line.starts_with('!') {
                // Negation pattern
                let pattern = &line[1..].trim();
                if !pattern.is_empty() {
                    let glob = Glob::new(pattern)?;
                    negation_builder.add(glob);
                }
            } else {
                // Regular pattern
                // If pattern ends with /, treat as directory pattern
                let pattern = if line.ends_with('/') {
                    // Match the directory and everything under it
                    format!("{}**", line)
                } else {
                    line.to_string()
                };
                
                let glob = Glob::new(&pattern)?;
                builder.add(glob);
            }
        }
        
        *self.patterns.write() = builder.build()?;
        *self.negations.write() = negation_builder.build()?;
        self.cache.write().clear(); // Clear cache on pattern reload
        
        Ok(())
    }
    
    /// Check if a path is allowed (not blocked by .rooignore)
    pub fn is_allowed(&self, path: &Path) -> bool {
        // Convert to absolute path if needed
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace.join(path)
        };
        
        // Check cache first
        if let Some(&cached) = self.cache.read().get(&abs_path) {
            return cached;
        }
        
        // Make path relative to workspace for pattern matching
        let rel_path = abs_path.strip_prefix(&self.workspace)
            .unwrap_or(&abs_path);
        
        // Check patterns
        let patterns = self.patterns.read();
        let negations = self.negations.read();
        
        // Check if path matches any ignore pattern
        let blocked = patterns.is_match(rel_path);
        
        // Check if negation pattern allows it
        let allowed = if blocked && negations.is_match(rel_path) {
            true
        } else {
            !blocked
        };
        
        // Cache the result
        self.cache.write().insert(abs_path, allowed);
        
        allowed
    }
    
    /// Check multiple paths efficiently
    pub fn filter_allowed(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        paths.iter()
            .filter(|p| self.is_allowed(p))
            .cloned()
            .collect()
    }
    
    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
    
    /// Get number of cached entries
    pub fn cache_size(&self) -> usize {
        self.cache.read().len()
    }
}

/// Default .rooignore patterns for security
impl Default for RooIgnore {
    fn default() -> Self {
        let mut instance = Self::new(PathBuf::from("."));
        
        // Load default security patterns
        let default_patterns = r#"
# System and sensitive files
.git/
.env
.env.*
*.key
*.pem
*.p12
*.pfx
secrets/
credentials/

# Build outputs
target/
dist/
build/
*.o
*.so
*.dylib
*.dll

# IDE and editor files
.vscode/
.idea/
*.swp
*.swo
*~

# OS files
.DS_Store
Thumbs.db
desktop.ini

# Temporary files
*.tmp
*.temp
*.log
tmp/
temp/
"#;
        
        let _ = instance.load_from_string(default_patterns);
        instance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::{self, File};
    use std::io::Write;
    use std::time::Instant;
    
    #[test]
    fn test_basic_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let mut rooignore = RooIgnore::new(temp_dir.path().to_path_buf());
        
        let patterns = r#"
*.log
temp/
build/
!build/important.txt
"#;
        
        rooignore.load_from_string(patterns).unwrap();
        
        // Test blocked patterns
        assert!(!rooignore.is_allowed(&temp_dir.path().join("debug.log")));
        assert!(!rooignore.is_allowed(&temp_dir.path().join("temp/file.txt")));
        assert!(!rooignore.is_allowed(&temp_dir.path().join("build/output.exe")));
        
        // Test negation pattern
        assert!(rooignore.is_allowed(&temp_dir.path().join("build/important.txt")));
        
        // Test allowed patterns
        assert!(rooignore.is_allowed(&temp_dir.path().join("src/main.rs")));
        assert!(rooignore.is_allowed(&temp_dir.path().join("readme.md")));
    }
    
    #[test]
    fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let rooignore_path = temp_dir.path().join(".rooignore");
        
        let mut file = File::create(&rooignore_path).unwrap();
        writeln!(file, "*.secret").unwrap();
        writeln!(file, "private/").unwrap();
        writeln!(file, "# This is a comment").unwrap();
        writeln!(file, "!private/public.txt").unwrap();
        
        let mut rooignore = RooIgnore::new(temp_dir.path().to_path_buf());
        rooignore.load_from_file(&rooignore_path).unwrap();
        
        assert!(!rooignore.is_allowed(&temp_dir.path().join("api.secret")));
        assert!(!rooignore.is_allowed(&temp_dir.path().join("private/data.txt")));
        assert!(rooignore.is_allowed(&temp_dir.path().join("private/public.txt")));
        assert!(rooignore.is_allowed(&temp_dir.path().join("normal.txt")));
    }
    
    #[test]
    fn test_caching() {
        let temp_dir = TempDir::new().unwrap();
        let mut rooignore = RooIgnore::new(temp_dir.path().to_path_buf());
        rooignore.load_from_string("*.cache").unwrap();
        
        let path = temp_dir.path().join("test.cache");
        
        // First call should cache
        assert_eq!(rooignore.cache_size(), 0);
        assert!(!rooignore.is_allowed(&path));
        assert_eq!(rooignore.cache_size(), 1);
        
        // Second call should use cache
        assert!(!rooignore.is_allowed(&path));
        assert_eq!(rooignore.cache_size(), 1);
        
        // Clear cache
        rooignore.clear_cache();
        assert_eq!(rooignore.cache_size(), 0);
    }
    
    #[test]
    fn test_filter_allowed() {
        let temp_dir = TempDir::new().unwrap();
        let mut rooignore = RooIgnore::new(temp_dir.path().to_path_buf());
        rooignore.load_from_string("*.blocked").unwrap();
        
        let paths = vec![
            temp_dir.path().join("file1.txt"),
            temp_dir.path().join("file2.blocked"),
            temp_dir.path().join("file3.rs"),
            temp_dir.path().join("file4.blocked"),
        ];
        
        let allowed = rooignore.filter_allowed(&paths);
        assert_eq!(allowed.len(), 2);
        assert!(allowed.contains(&temp_dir.path().join("file1.txt")));
        assert!(allowed.contains(&temp_dir.path().join("file3.rs")));
    }
    
    #[test]
    fn test_performance() {
        let temp_dir = TempDir::new().unwrap();
        let mut rooignore = RooIgnore::new(temp_dir.path().to_path_buf());
        
        // Load complex patterns
        let patterns = r#"
*.log
*.tmp
*.cache
node_modules/
target/
build/
dist/
.git/
.vscode/
"#;
        rooignore.load_from_string(patterns).unwrap();
        
        // Create test paths
        let mut paths = Vec::new();
        for i in 0..1000 {
            paths.push(temp_dir.path().join(format!("file{}.txt", i)));
            paths.push(temp_dir.path().join(format!("file{}.log", i)));
        }
        
        // Measure performance
        let start = Instant::now();
        for path in &paths {
            let _ = rooignore.is_allowed(path);
        }
        let elapsed = start.elapsed();
        
        // Should process 2000 paths in under 100ms
        assert!(
            elapsed.as_millis() < 100,
            "Processing took {}ms, expected < 100ms",
            elapsed.as_millis()
        );
        
        // Cached access should be much faster
        let start = Instant::now();
        for path in &paths {
            let _ = rooignore.is_allowed(path);
        }
        let cached_elapsed = start.elapsed();
        
        // Cached should be at least 2x faster (more realistic)
        assert!(
            cached_elapsed < elapsed / 2,
            "Cached access not fast enough: cached {}ms vs uncached {}ms",
            cached_elapsed.as_millis(),
            elapsed.as_millis()
        );
    }
}
