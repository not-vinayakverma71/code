// RooIgnore enforcement end-to-end - P1-9
// Ensures all tools respect .rooignore rules

use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use tokio::fs;

/// RooIgnore enforcer for end-to-end protection
pub struct RooIgnoreEnforcer {
    patterns: Arc<GlobSet>,
    workspace_root: PathBuf,
    allow_list: Arc<GlobSet>,
    deny_list: Arc<GlobSet>,
}

impl RooIgnoreEnforcer {
    /// Load .rooignore from workspace
    pub async fn load(workspace_root: PathBuf) -> Result<Self> {
        let rooignore_path = workspace_root.join(".rooignore");
        
        let (patterns, allow_list, deny_list) = if rooignore_path.exists() {
            let content = fs::read_to_string(&rooignore_path).await?;
            Self::parse_rooignore(&content)?
        } else {
            // Default patterns if no .rooignore exists
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("**/.git/**")?);
            builder.add(Glob::new("**/node_modules/**")?);
            builder.add(Glob::new("**/.env*")?);
            builder.add(Glob::new("**/*.key")?);
            builder.add(Glob::new("**/*.pem")?);
            
            (builder.build()?, GlobSetBuilder::new().build()?, GlobSetBuilder::new().build()?)
        };
        
        Ok(Self {
            patterns: Arc::new(patterns),
            workspace_root,
            allow_list: Arc::new(allow_list),
            deny_list: Arc::new(deny_list),
        })
    }
    
    /// Parse .rooignore content
    fn parse_rooignore(content: &str) -> Result<(GlobSet, GlobSet, GlobSet)> {
        let mut patterns = GlobSetBuilder::new();
        let mut allow_list = GlobSetBuilder::new();
        let mut deny_list = GlobSetBuilder::new();
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Allow list (starts with !)
            if let Some(pattern) = line.strip_prefix('!') {
                allow_list.add(Glob::new(pattern)?);
            }
            // Deny list (starts with -)
            else if let Some(pattern) = line.strip_prefix('-') {
                deny_list.add(Glob::new(pattern)?);
            }
            // Regular ignore pattern
            else {
                patterns.add(Glob::new(line)?);
            }
        }
        
        Ok((patterns.build()?, allow_list.build()?, deny_list.build()?))
    }
    
    /// Check if a path is allowed
    pub fn is_allowed(&self, path: &Path) -> bool {
        // Convert to relative path
        let relative = if path.is_absolute() {
            path.strip_prefix(&self.workspace_root).unwrap_or(path)
        } else {
            path
        };
        
        // Check deny list first (highest priority)
        if self.deny_list.is_match(relative) {
            return false;
        }
        
        // Check allow list (overrides ignore patterns)
        if self.allow_list.is_match(relative) {
            return true;
        }
        
        // Check ignore patterns
        !self.patterns.is_match(relative)
    }
    
    /// Filter a list of paths
    pub fn filter_paths(&self, paths: Vec<PathBuf>) -> Vec<PathBuf> {
        paths.into_iter()
            .filter(|p| self.is_allowed(p))
            .collect()
    }
    
    /// Enforce on directory listing
    pub async fn filter_directory(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = fs::read_dir(dir).await?;
        let mut allowed = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if self.is_allowed(&path) {
                allowed.push(path);
            }
        }
        
        Ok(allowed)
    }
    
    /// Get enforcement statistics
    pub fn get_stats(&self) -> RooIgnoreStats {
        RooIgnoreStats {
            total_patterns: self.patterns.len(),
            allow_patterns: self.allow_list.len(),
            deny_patterns: self.deny_list.len(),
        }
    }
}

/// RooIgnore statistics
#[derive(Debug, Clone)]
pub struct RooIgnoreStats {
    pub total_patterns: usize,
    pub allow_patterns: usize,
    pub deny_patterns: usize,
}

/// Global RooIgnore enforcer instance
lazy_static::lazy_static! {
    static ref GLOBAL_ENFORCER: Arc<tokio::sync::RwLock<Option<Arc<RooIgnoreEnforcer>>>> = 
        Arc::new(tokio::sync::RwLock::new(None));
}

/// Initialize global enforcer
pub async fn init_global_enforcer(workspace_root: PathBuf) -> Result<()> {
    let enforcer = RooIgnoreEnforcer::load(workspace_root).await?;
    *GLOBAL_ENFORCER.write().await = Some(Arc::new(enforcer));
    Ok(())
}

/// Get global enforcer
pub async fn get_global_enforcer() -> Option<Arc<RooIgnoreEnforcer>> {
    GLOBAL_ENFORCER.read().await.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_rooignore_parsing() {
        let content = r#"
# Ignore patterns
*.log
temp/
.env

# Allow specific files
!important.log

# Deny absolutely
-secrets/
"#;
        
        let (patterns, allow, deny) = RooIgnoreEnforcer::parse_rooignore(content).unwrap();
        
        assert!(patterns.len() > 0);
        assert!(allow.len() > 0);
        assert!(deny.len() > 0);
    }
    
    #[tokio::test]
    async fn test_path_filtering() {
        let temp = TempDir::new().unwrap();
        let enforcer = RooIgnoreEnforcer::load(temp.path().to_path_buf()).await.unwrap();
        
        // Should block .git by default
        assert!(!enforcer.is_allowed(Path::new(".git/config")));
        
        // Should allow normal files
        assert!(enforcer.is_allowed(Path::new("src/main.rs")));
    }
}
