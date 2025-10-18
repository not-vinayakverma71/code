// Terminal Pre-IPC: Terminal state persistence and snapshots
// Part of HP4: Terminal Snapshot feature

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use lapce_rpc::terminal::TermId;

use super::types::{CommandHistory, CommandRecord, CommandSource};

/// Maximum scrollback lines to save (limit memory usage)
const MAX_SCROLLBACK_LINES: usize = 10000;

/// Snapshot of terminal state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSnapshot {
    /// Version for compatibility checking
    pub version: u32,
    
    /// Terminal ID
    pub term_id: String,
    
    /// Working directory
    pub cwd: PathBuf,
    
    /// Environment variables (subset)
    pub env: HashMap<String, String>,
    
    /// Command execution history
    pub command_history: CommandHistory,
    
    /// Scrollback buffer (limited)
    #[serde(default)]
    pub scrollback: Vec<String>,
    
    /// Terminal title
    #[serde(default)]
    pub title: String,
    
    /// When this snapshot was created
    pub created_at: i64,
    
    /// Workspace path (for boundary validation)
    pub workspace_path: PathBuf,
}

impl TerminalSnapshot {
    /// Current snapshot version
    pub const VERSION: u32 = 1;
    
    /// Create a new snapshot
    pub fn new(
        term_id: TermId,
        cwd: PathBuf,
        workspace_path: PathBuf,
        title: String,
    ) -> Self {
        Self {
            version: Self::VERSION,
            term_id: format!("{:?}", term_id),
            cwd,
            env: HashMap::new(),
            command_history: CommandHistory::default(),
            scrollback: Vec::new(),
            title,
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            workspace_path,
        }
    }
    
    /// Set environment variables (filtered subset)
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        // Only save important env vars (filter out sensitive ones)
        let allowed_vars = [
            "PATH", "HOME", "USER", "SHELL", "TERM",
            "LANG", "LC_ALL", "PWD", "OLDPWD",
        ];
        
        self.env = env.into_iter()
            .filter(|(k, _)| allowed_vars.contains(&k.as_str()))
            .collect();
        
        self
    }
    
    /// Set command history
    pub fn with_history(mut self, history: CommandHistory) -> Self {
        self.command_history = history;
        self
    }
    
    /// Set scrollback buffer (truncated if too large)
    pub fn with_scrollback(mut self, scrollback: Vec<String>) -> Self {
        // Keep only last N lines
        let start = scrollback.len().saturating_sub(MAX_SCROLLBACK_LINES);
        self.scrollback = scrollback[start..].to_vec();
        self
    }
    
    /// Validate snapshot data
    pub fn validate(&self) -> Result<()> {
        // Check version compatibility
        if self.version > Self::VERSION {
            return Err(anyhow!(
                "Snapshot version {} is newer than supported version {}",
                self.version,
                Self::VERSION
            ));
        }
        
        // Validate workspace boundary
        if !self.cwd.starts_with(&self.workspace_path) {
            return Err(anyhow!(
                "CWD {:?} is outside workspace {:?}",
                self.cwd,
                self.workspace_path
            ));
        }
        
        // Validate paths exist
        if !self.cwd.exists() {
            return Err(anyhow!("CWD {:?} does not exist", self.cwd));
        }
        
        Ok(())
    }
    
    /// Get snapshot age
    pub fn age(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        Duration::from_secs((now - self.created_at).max(0) as u64)
    }
}

/// Manager for terminal snapshots
pub struct SnapshotManager {
    /// Base directory for snapshots
    snapshot_dir: PathBuf,
    
    /// Workspace path for boundary validation
    workspace_path: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(workspace_path: PathBuf) -> Result<Self> {
        let snapshot_dir = workspace_path.join(".lapce").join("terminal_snapshots");
        
        // Create directory if it doesn't exist
        if !snapshot_dir.exists() {
            fs::create_dir_all(&snapshot_dir)
                .with_context(|| format!("Failed to create snapshot directory: {:?}", snapshot_dir))?;
        }
        
        Ok(Self {
            snapshot_dir,
            workspace_path,
        })
    }
    
    /// Get snapshot file path for a terminal
    fn snapshot_path(&self, term_id: &TermId) -> PathBuf {
        self.snapshot_dir.join(format!("{:?}.json", term_id))
    }
    
    /// Save a snapshot
    pub fn save(&self, snapshot: &TerminalSnapshot) -> Result<()> {
        // Validate before saving
        snapshot.validate()?;
        
        let path = self.snapshot_dir.join(format!("{}.json", snapshot.term_id));
        
        // Serialize to JSON with pretty printing
        let json = serde_json::to_string_pretty(snapshot)
            .context("Failed to serialize snapshot")?;
        
        // Write atomically (write to temp file, then rename)
        let temp_path = path.with_extension("json.tmp");
        fs::write(&temp_path, json)
            .with_context(|| format!("Failed to write snapshot to {:?}", temp_path))?;
        
        fs::rename(&temp_path, &path)
            .with_context(|| format!("Failed to rename snapshot to {:?}", path))?;
        
        tracing::info!("Saved terminal snapshot: {:?}", path);
        
        Ok(())
    }
    
    /// Load a snapshot
    pub fn load(&self, term_id: &TermId) -> Result<TerminalSnapshot> {
        let path = self.snapshot_path(term_id);
        
        if !path.exists() {
            return Err(anyhow!("Snapshot not found: {:?}", path));
        }
        
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read snapshot from {:?}", path))?;
        
        let snapshot: TerminalSnapshot = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse snapshot from {:?}", path))?;
        
        // Validate after loading
        snapshot.validate()?;
        
        tracing::info!("Loaded terminal snapshot: {:?}", path);
        
        Ok(snapshot)
    }
    
    /// List all available snapshots
    pub fn list_snapshots(&self) -> Result<Vec<TerminalSnapshot>> {
        let mut snapshots = Vec::new();
        
        if !self.snapshot_dir.exists() {
            return Ok(snapshots);
        }
        
        for entry in fs::read_dir(&self.snapshot_dir)
            .with_context(|| format!("Failed to read snapshot directory: {:?}", self.snapshot_dir))?
        {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_from_path(&path) {
                    Ok(snapshot) => snapshots.push(snapshot),
                    Err(e) => {
                        tracing::warn!("Failed to load snapshot {:?}: {}", path, e);
                    }
                }
            }
        }
        
        // Sort by creation time (newest first)
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(snapshots)
    }
    
    /// Load snapshot from specific path
    fn load_from_path(&self, path: &Path) -> Result<TerminalSnapshot> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read snapshot from {:?}", path))?;
        
        let snapshot: TerminalSnapshot = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse snapshot from {:?}", path))?;
        
        Ok(snapshot)
    }
    
    /// Delete a snapshot
    pub fn delete(&self, term_id: &TermId) -> Result<()> {
        let path = self.snapshot_path(term_id);
        
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to delete snapshot: {:?}", path))?;
            
            tracing::info!("Deleted terminal snapshot: {:?}", path);
        }
        
        Ok(())
    }
    
    /// Clean up old snapshots (older than specified duration)
    pub fn cleanup_old(&self, max_age: Duration) -> Result<usize> {
        let snapshots = self.list_snapshots()?;
        let mut deleted = 0;
        
        for snapshot in snapshots {
            if snapshot.age() > max_age {
                // Delete by file path directly since term_id is stored as string
                let path = self.snapshot_dir.join(format!("{}.json", snapshot.term_id));
                if path.exists() {
                    if let Ok(_) = fs::remove_file(&path) {
                        deleted += 1;
                    }
                }
            }
        }
        
        if deleted > 0 {
            tracing::info!("Cleaned up {} old terminal snapshots", deleted);
        }
        
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_snapshot() -> TerminalSnapshot {
        let term_id = TermId::next();
        let workspace = PathBuf::from("/tmp/workspace");
        let cwd = workspace.join("project");
        
        let mut snapshot = TerminalSnapshot::new(
            term_id,
            cwd,
            workspace,
            "Test Terminal".to_string(),
        );
        
        // Add some env vars
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin".to_string());
        env.insert("HOME".to_string(), "/home/user".to_string());
        snapshot = snapshot.with_env(env);
        
        // Add command history
        let mut history = CommandHistory::default();
        history.push(CommandRecord::new(
            "ls -la".to_string(),
            CommandSource::User,
            PathBuf::from("/tmp"),
        ));
        snapshot = snapshot.with_history(history);
        
        // Add scrollback
        snapshot = snapshot.with_scrollback(vec![
            "line 1".to_string(),
            "line 2".to_string(),
        ]);
        
        snapshot
    }
    
    #[test]
    fn test_snapshot_creation() {
        let snapshot = create_test_snapshot();
        
        assert_eq!(snapshot.version, TerminalSnapshot::VERSION);
        assert_eq!(snapshot.title, "Test Terminal");
        assert_eq!(snapshot.env.len(), 2);
        assert_eq!(snapshot.command_history.len(), 1);
        assert_eq!(snapshot.scrollback.len(), 2);
    }
    
    #[test]
    fn test_snapshot_env_filtering() {
        let term_id = TermId::next();
        let workspace = PathBuf::from("/tmp");
        
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin".to_string());
        env.insert("SECRET_KEY".to_string(), "secret123".to_string());
        env.insert("HOME".to_string(), "/home/user".to_string());
        
        let snapshot = TerminalSnapshot::new(
            term_id,
            workspace.clone(),
            workspace,
            "Test".to_string(),
        ).with_env(env);
        
        // Should only keep allowed vars
        assert_eq!(snapshot.env.len(), 2);
        assert!(snapshot.env.contains_key("PATH"));
        assert!(snapshot.env.contains_key("HOME"));
        assert!(!snapshot.env.contains_key("SECRET_KEY"));
    }
    
    #[test]
    fn test_snapshot_scrollback_truncation() {
        let term_id = TermId::next();
        let workspace = PathBuf::from("/tmp");
        
        // Create large scrollback
        let large_scrollback: Vec<String> = (0..15000)
            .map(|i| format!("line {}", i))
            .collect();
        
        let snapshot = TerminalSnapshot::new(
            term_id,
            workspace.clone(),
            workspace,
            "Test".to_string(),
        ).with_scrollback(large_scrollback);
        
        // Should be truncated to MAX_SCROLLBACK_LINES
        assert_eq!(snapshot.scrollback.len(), MAX_SCROLLBACK_LINES);
        assert_eq!(snapshot.scrollback[0], "line 5000");
    }
    
    #[test]
    fn test_snapshot_serialization() {
        let snapshot = create_test_snapshot();
        
        // Serialize
        let json = serde_json::to_string(&snapshot).unwrap();
        
        // Deserialize
        let restored: TerminalSnapshot = serde_json::from_str(&json).unwrap();
        
        assert_eq!(restored.version, snapshot.version);
        assert_eq!(restored.term_id, snapshot.term_id);
        assert_eq!(restored.title, snapshot.title);
        assert_eq!(restored.command_history.len(), 1);
    }
    
    #[test]
    fn test_snapshot_manager_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let manager = SnapshotManager::new(workspace.clone()).unwrap();
        
        // Create snapshot with valid paths
        let term_id = TermId::next();
        let cwd = workspace.join("project");
        fs::create_dir_all(&cwd).unwrap();
        
        let snapshot = TerminalSnapshot::new(
            term_id,
            cwd,
            workspace,
            "Test".to_string(),
        );
        
        // Save
        manager.save(&snapshot).unwrap();
        
        // Load
        let loaded = manager.load(&term_id).unwrap();
        assert_eq!(loaded.term_id, snapshot.term_id);
        assert_eq!(loaded.title, snapshot.title);
    }
    
    #[test]
    fn test_snapshot_manager_list() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let manager = SnapshotManager::new(workspace.clone()).unwrap();
        
        // Create multiple snapshots
        for i in 0..3 {
            let term_id = TermId::next();
            let cwd = workspace.join(format!("project{}", i));
            fs::create_dir_all(&cwd).unwrap();
            
            let snapshot = TerminalSnapshot::new(
                term_id,
                cwd,
                workspace.clone(),
                format!("Terminal {}", i),
            );
            manager.save(&snapshot).unwrap();
        }
        
        // List all
        let snapshots = manager.list_snapshots().unwrap();
        assert_eq!(snapshots.len(), 3);
    }
    
    #[test]
    fn test_snapshot_age() {
        let snapshot = create_test_snapshot();
        
        let age = snapshot.age();
        assert!(age < Duration::from_secs(1)); // Should be very recent
    }
}
