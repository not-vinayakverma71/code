// Unified RooIgnore Enforcement System - Production-grade with hot reload
// Part of RooIgnore enforcement TODO #6 - pre-IPC

use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
use anyhow::{Result, Context, bail};
use parking_lot::RwLock;
use globset::{Glob, GlobSet, GlobSetBuilder};
use notify::{Watcher, RecursiveMode, Event, Config};
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;

/// Unified error shape for RooIgnore violations
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("RooIgnore blocked: {message}")]
pub struct RooIgnoreBlocked {
    pub path: PathBuf,
    pub pattern: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl From<RooIgnoreBlocked> for crate::core::tools::traits::ToolError {
    fn from(blocked: RooIgnoreBlocked) -> Self {
        crate::core::tools::traits::ToolError::RooIgnoreBlocked(blocked.to_string())
    }
}

/// RooIgnore configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RooIgnoreConfig {
    pub workspace: PathBuf,
    pub rooignore_path: PathBuf,
    pub enable_hot_reload: bool,
    pub cache_ttl: Duration,
    pub max_cache_size: usize,
    pub default_patterns: Vec<String>,
    pub strict_mode: bool,
}

impl Default for RooIgnoreConfig {
    fn default() -> Self {
        Self {
            workspace: PathBuf::from("."),
            rooignore_path: PathBuf::from(".rooignore"),
            enable_hot_reload: true,
            cache_ttl: Duration::from_secs(300), // 5 minutes
            max_cache_size: 10000,
            default_patterns: Self::default_security_patterns(),
            strict_mode: true,
        }
    }
}

impl RooIgnoreConfig {
    fn default_security_patterns() -> Vec<String> {
        vec![
            // Sensitive files
            ".env".to_string(),
            ".env.*".to_string(),
            "*.key".to_string(),
            "*.pem".to_string(),
            "*.p12".to_string(),
            "*.pfx".to_string(),
            "**/secrets/**".to_string(),
            "**/credentials/**".to_string(),
            "**/.git/objects/**".to_string(),
            "**/.git/hooks/**".to_string(),
            
            // System files
            "/etc/**".to_string(),
            "/sys/**".to_string(),
            "/proc/**".to_string(),
            "~/.ssh/**".to_string(),
            "~/.gnupg/**".to_string(),
            
            // Build artifacts (optional)
            "**/target/release/**".to_string(),
            "**/target/debug/**".to_string(),
            "**/node_modules/**".to_string(),
            
            // OS files
            ".DS_Store".to_string(),
            "Thumbs.db".to_string(),
            "desktop.ini".to_string(),
        ]
    }
}

/// Cache entry with TTL
#[derive(Debug, Clone)]
struct CacheEntry {
    allowed: bool,
    pattern: Option<String>,
    timestamp: SystemTime,
}

impl CacheEntry {
    fn is_expired(&self, ttl: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.timestamp)
            .map(|elapsed| elapsed > ttl)
            .unwrap_or(true)
    }
}

/// Unified RooIgnore enforcer with hot reload
pub struct UnifiedRooIgnore {
    config: Arc<RooIgnoreConfig>,
    patterns: Arc<RwLock<PatternSet>>,
    cache: Arc<RwLock<HashMap<PathBuf, CacheEntry>>>,
    watcher: Option<notify::RecommendedWatcher>,
    reload_tx: Option<mpsc::UnboundedSender<()>>,
    stats: Arc<RwLock<EnforcementStats>>,
}

#[derive(Debug, Clone)]
struct PatternSet {
    blocks: GlobSet,
    allows: GlobSet,
    pattern_map: HashMap<String, String>, // pattern -> original string
    last_reload: SystemTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnforcementStats {
    pub total_checks: u64,
    pub cache_hits: u64,
    pub blocks: u64,
    pub allows: u64,
    pub reloads: u64,
    pub last_reload: Option<SystemTime>,
}

impl UnifiedRooIgnore {
    /// Create new unified enforcer
    pub fn new(config: RooIgnoreConfig) -> Result<Self> {
        let mut enforcer = Self {
            config: Arc::new(config.clone()),
            patterns: Arc::new(RwLock::new(PatternSet {
                blocks: GlobSet::empty(),
                allows: GlobSet::empty(),
                pattern_map: HashMap::new(),
                last_reload: SystemTime::now(),
            })),
            cache: Arc::new(RwLock::new(HashMap::new())),
            watcher: None,
            reload_tx: None,
            stats: Arc::new(RwLock::new(EnforcementStats::default())),
        };
        
        // Load initial patterns
        enforcer.reload()?;
        
        // Setup hot reload if enabled
        if config.enable_hot_reload {
            enforcer.setup_hot_reload()?;
        }
        
        Ok(enforcer)
    }
    
    /// Central allow check - all tools MUST use this
    pub fn check_allowed(&self, path: &Path) -> Result<(), RooIgnoreBlocked> {
        self.stats.write().total_checks += 1;
        
        // Normalize path
        let abs_path = self.normalize_path(path);
        
        // Check cache first
        if let Some(entry) = self.get_cache_entry(&abs_path) {
            if entry.allowed {
                self.stats.write().allows += 1;
                return Ok(());
            } else {
                self.stats.write().blocks += 1;
                return Err(RooIgnoreBlocked {
                    path: abs_path,
                    pattern: entry.pattern.unwrap_or_default(),
                    message: format!("Path blocked by .rooignore"),
                    suggestion: Some("Check .rooignore file or request exception".to_string()),
                });
            }
        }
        
        // Perform pattern check
        let (allowed, pattern) = self.check_patterns(&abs_path);
        
        // Cache result
        self.cache_result(&abs_path, allowed, pattern.clone());
        
        // Update stats and return
        if allowed {
            self.stats.write().allows += 1;
            Ok(())
        } else {
            self.stats.write().blocks += 1;
            Err(RooIgnoreBlocked {
                path: abs_path,
                pattern: pattern.unwrap_or_default(),
                message: format!("Path blocked by .rooignore pattern"),
                suggestion: Some("Add negation pattern to .rooignore if needed".to_string()),
            })
        }
    }
    
    /// Batch check multiple paths
    pub fn check_allowed_batch(&self, paths: &[&Path]) -> Vec<Result<(), RooIgnoreBlocked>> {
        paths.iter().map(|p| self.check_allowed(p)).collect()
    }
    
    /// Force reload of patterns
    pub fn reload(&mut self) -> Result<()> {
        let patterns = self.load_patterns()?;
        *self.patterns.write() = patterns;
        
        // Clear cache on reload
        self.cache.write().clear();
        
        // Update stats
        let mut stats = self.stats.write();
        stats.reloads += 1;
        stats.last_reload = Some(SystemTime::now());
        
        Ok(())
    }
    
    /// Load patterns from file
    fn load_patterns(&self) -> Result<PatternSet> {
        let mut block_builder = GlobSetBuilder::new();
        let mut allow_builder = GlobSetBuilder::new();
        let mut pattern_map = HashMap::new();
        
        // Load default patterns if strict mode
        if self.config.strict_mode {
            for pattern in &self.config.default_patterns {
                let glob = Glob::new(pattern)?;
                block_builder.add(glob);
                pattern_map.insert(pattern.clone(), pattern.clone());
            }
        }
        
        // Load from .rooignore file
        let rooignore_path = self.config.workspace.join(&self.config.rooignore_path);
        if rooignore_path.exists() {
            let content = fs::read_to_string(&rooignore_path)
                .context("Failed to read .rooignore")?;
            
            for line in content.lines() {
                let line = line.trim();
                
                // Skip comments and empty lines
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                
                if line.starts_with('!') {
                    // Allow pattern (negation)
                    let pattern = line[1..].trim();
                    if !pattern.is_empty() {
                        let glob_pattern = self.normalize_pattern(pattern);
                        let glob = Glob::new(&glob_pattern)?;
                        allow_builder.add(glob);
                        pattern_map.insert(glob_pattern, line.to_string());
                    }
                } else {
                    // Block pattern
                    let glob_pattern = self.normalize_pattern(line);
                    let glob = Glob::new(&glob_pattern)?;
                    block_builder.add(glob);
                    pattern_map.insert(glob_pattern, line.to_string());
                }
            }
        }
        
        Ok(PatternSet {
            blocks: block_builder.build()?,
            allows: allow_builder.build()?,
            pattern_map,
            last_reload: SystemTime::now(),
        })
    }
    
    /// Normalize pattern for consistent matching
    fn normalize_pattern(&self, pattern: &str) -> String {
        let pattern = pattern.trim();
        
        // Handle directory patterns
        if pattern.ends_with('/') {
            format!("{}**", pattern)
        } else if !pattern.contains('/') && !pattern.contains('*') {
            // Match anywhere in tree
            format!("**/{}", pattern)
        } else {
            pattern.to_string()
        }
    }
    
    /// Normalize path for checking
    fn normalize_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.workspace.join(path)
        }
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
    }
    
    /// Check patterns against path
    fn check_patterns(&self, path: &Path) -> (bool, Option<String>) {
        let patterns = self.patterns.read();
        
        // Make path relative to workspace
        let rel_path = path.strip_prefix(&self.config.workspace)
            .unwrap_or(path);
        
        // Check block patterns
        if patterns.blocks.is_match(rel_path) {
            // Check if allowed by negation
            if patterns.allows.is_match(rel_path) {
                (true, None)
            } else {
                // Find matching pattern for error message
                let pattern = patterns.pattern_map.iter()
                    .find(|(glob_pattern, _)| {
                        Glob::new(glob_pattern).ok()
                            .and_then(|g| GlobSetBuilder::new().add(g).build().ok())
                            .map(|gs| gs.is_match(rel_path))
                            .unwrap_or(false)
                    })
                    .map(|(_, original)| original.clone());
                
                (false, pattern)
            }
        } else {
            (true, None)
        }
    }
    
    /// Get cache entry if valid
    fn get_cache_entry(&self, path: &Path) -> Option<CacheEntry> {
        let cache = self.cache.read();
        cache.get(path)
            .filter(|entry| !entry.is_expired(self.config.cache_ttl))
            .cloned()
            .map(|entry| {
                // Update cache hit stats
                self.stats.write().cache_hits += 1;
                entry
            })
    }
    
    /// Cache check result
    fn cache_result(&self, path: &Path, allowed: bool, pattern: Option<String>) {
        let mut cache = self.cache.write();
        
        // Enforce max cache size
        if cache.len() >= self.config.max_cache_size {
            // Remove oldest entries
            let mut entries: Vec<_> = cache.iter()
                .map(|(k, v)| (k.clone(), v.timestamp))
                .collect();
            entries.sort_by_key(|(_, time)| *time);
            
            for (key, _) in entries.iter().take(cache.len() / 4) {
                cache.remove(key);
            }
        }
        
        cache.insert(path.to_path_buf(), CacheEntry {
            allowed,
            pattern,
            timestamp: SystemTime::now(),
        });
    }
    
    /// Setup hot reload watcher
    fn setup_hot_reload(&mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.reload_tx = Some(tx.clone());
        
        let patterns = self.patterns.clone();
        let cache = self.cache.clone();
        let stats = self.stats.clone();
        let config = self.config.clone();
        
        // Spawn reload handler
        tokio::spawn(async move {
            while rx.recv().await.is_some() {
                // Debounce rapid changes
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                // Reload patterns
                if let Ok(new_patterns) = Self::load_patterns_static(&config) {
                    *patterns.write() = new_patterns;
                    cache.write().clear();
                    
                    let mut s = stats.write();
                    s.reloads += 1;
                    s.last_reload = Some(SystemTime::now());
                }
            }
        });
        
        // Setup file watcher
        let reload_tx = self.reload_tx.clone().unwrap();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if res.is_ok() {
                let _ = reload_tx.send(());
            }
        })?;
        
        let watch_path = self.config.workspace.join(&self.config.rooignore_path);
        watcher.watch(&watch_path, RecursiveMode::NonRecursive)?;
        
        self.watcher = Some(watcher);
        
        Ok(())
    }
    
    /// Static version for async context
    fn load_patterns_static(config: &RooIgnoreConfig) -> Result<PatternSet> {
        let mut block_builder = GlobSetBuilder::new();
        let mut allow_builder = GlobSetBuilder::new();
        let mut pattern_map = HashMap::new();
        
        // Load default patterns if strict mode
        if config.strict_mode {
            for pattern in &config.default_patterns {
                let glob = Glob::new(pattern)?;
                block_builder.add(glob);
                pattern_map.insert(pattern.clone(), pattern.clone());
            }
        }
        
        // Load from file
        let rooignore_path = config.workspace.join(&config.rooignore_path);
        if rooignore_path.exists() {
            let content = fs::read_to_string(&rooignore_path)?;
            
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                
                if line.starts_with('!') {
                    let pattern = line[1..].trim();
                    if !pattern.is_empty() {
                        let glob = Glob::new(pattern)?;
                        allow_builder.add(glob);
                        pattern_map.insert(pattern.to_string(), line.to_string());
                    }
                } else {
                    let glob = Glob::new(line)?;
                    block_builder.add(glob);
                    pattern_map.insert(line.to_string(), line.to_string());
                }
            }
        }
        
        Ok(PatternSet {
            blocks: block_builder.build()?,
            allows: allow_builder.build()?,
            pattern_map,
            last_reload: SystemTime::now(),
        })
    }
    
    /// Get enforcement statistics
    pub fn stats(&self) -> EnforcementStats {
        self.stats.read().clone()
    }
    
    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

/// Global singleton enforcer
lazy_static::lazy_static! {
    static ref GLOBAL_ENFORCER: Arc<RwLock<Option<UnifiedRooIgnore>>> = 
        Arc::new(RwLock::new(None));
}

/// Initialize global enforcer
pub fn init_global_enforcer(config: RooIgnoreConfig) -> Result<()> {
    let enforcer = UnifiedRooIgnore::new(config)?;
    *GLOBAL_ENFORCER.write() = Some(enforcer);
    Ok(())
}

/// Get global enforcer
pub fn global_enforcer() -> Result<Arc<RwLock<Option<UnifiedRooIgnore>>>> {
    Ok(GLOBAL_ENFORCER.clone())
}

/// Convenience function for checking paths
pub fn check_path_allowed(path: &Path) -> Result<(), RooIgnoreBlocked> {
    let enforcer = GLOBAL_ENFORCER.read();
    if let Some(ref e) = *enforcer {
        e.check_allowed(path)
    } else {
        // If not initialized, allow by default (non-strict mode)
        Ok(())
    }
}

// Add to Cargo.toml:
// notify = "6.1"
// thiserror = "1.0"

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_unified_enforcement() {
        let temp_dir = TempDir::new().unwrap();
        let rooignore_path = temp_dir.path().join(".rooignore");
        
        fs::write(&rooignore_path, "*.secret\nprivate/\n!private/public.txt").unwrap();
        
        let config = RooIgnoreConfig {
            workspace: temp_dir.path().to_path_buf(),
            rooignore_path: PathBuf::from(".rooignore"),
            enable_hot_reload: false,
            strict_mode: false,
            ..Default::default()
        };
        
        let enforcer = UnifiedRooIgnore::new(config).unwrap();
        
        // Test blocked paths
        assert!(enforcer.check_allowed(&temp_dir.path().join("api.secret")).is_err());
        assert!(enforcer.check_allowed(&temp_dir.path().join("private/data.txt")).is_err());
        
        // Test allowed paths
        assert!(enforcer.check_allowed(&temp_dir.path().join("private/public.txt")).is_ok());
        assert!(enforcer.check_allowed(&temp_dir.path().join("normal.txt")).is_ok());
        
        // Check stats
        let stats = enforcer.stats();
        assert_eq!(stats.total_checks, 4);
        assert_eq!(stats.blocks, 2);
        assert_eq!(stats.allows, 2);
    }
    
    #[test]
    fn test_error_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let rooignore_path = temp_dir.path().join(".rooignore");
        
        fs::write(&rooignore_path, "blocked.txt").unwrap();
        
        let config = RooIgnoreConfig {
            workspace: temp_dir.path().to_path_buf(),
            rooignore_path: PathBuf::from(".rooignore"),
            enable_hot_reload: false,
            strict_mode: false,
            ..Default::default()
        };
        
        let enforcer = UnifiedRooIgnore::new(config).unwrap();
        
        let result = enforcer.check_allowed(&temp_dir.path().join("blocked.txt"));
        assert!(result.is_err());
        
        if let Err(blocked) = result {
            assert!(blocked.message.contains("blocked"));
            assert!(blocked.suggestion.is_some());
            // File doesn't exist, so compare with non-canonicalized path
            assert_eq!(blocked.path, temp_dir.path().join("blocked.txt"));
        }
    }
    
    #[tokio::test]
    async fn test_hot_reload() {
        let temp_dir = TempDir::new().unwrap();
        let rooignore_path = temp_dir.path().join(".rooignore");
        
        fs::write(&rooignore_path, "initial.txt").unwrap();
        
        let config = RooIgnoreConfig {
            workspace: temp_dir.path().to_path_buf(),
            rooignore_path: PathBuf::from(".rooignore"),
            enable_hot_reload: true,
            strict_mode: false,
            ..Default::default()
        };
        
        let mut enforcer = UnifiedRooIgnore::new(config).unwrap();
        
        // Initial check
        assert!(enforcer.check_allowed(&temp_dir.path().join("initial.txt")).is_err());
        assert!(enforcer.check_allowed(&temp_dir.path().join("other.txt")).is_ok());
        
        // Update file
        fs::write(&rooignore_path, "initial.txt\nother.txt").unwrap();
        
        // Manual reload
        enforcer.reload().unwrap();
        
        // Check updated patterns
        assert!(enforcer.check_allowed(&temp_dir.path().join("initial.txt")).is_err());
        assert!(enforcer.check_allowed(&temp_dir.path().join("other.txt")).is_err());
        
        let stats = enforcer.stats();
        assert_eq!(stats.reloads, 2); // Initial load + manual reload
    }
}
