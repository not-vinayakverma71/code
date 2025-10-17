// Terminal Pre-IPC: Terminal restore flow on startup
// Part of HP4: Terminal Snapshot feature

use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

use super::persistence::{SnapshotManager, TerminalSnapshot};

/// Restore operation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreResult {
    /// Successfully restored terminal
    Success {
        term_id: String,
        cwd: PathBuf,
    },
    
    /// Skipped restore (user declined or snapshot invalid)
    Skipped {
        term_id: String,
        reason: String,
    },
    
    /// Failed to restore (error occurred)
    Failed {
        term_id: String,
        error: String,
    },
}

/// Restore session containing multiple snapshots
#[derive(Debug, Clone)]
pub struct RestoreSession {
    /// Available snapshots to restore
    pub snapshots: Vec<TerminalSnapshot>,
    
    /// Workspace path for validation
    pub workspace_path: PathBuf,
}

impl RestoreSession {
    /// Create a new restore session from available snapshots
    pub fn new(workspace_path: PathBuf) -> Result<Self> {
        let manager = SnapshotManager::new(workspace_path.clone())?;
        let snapshots = manager.list_snapshots()?;
        
        Ok(Self {
            snapshots,
            workspace_path,
        })
    }
    
    /// Check if there are any snapshots to restore
    pub fn has_snapshots(&self) -> bool {
        !self.snapshots.is_empty()
    }
    
    /// Get count of available snapshots
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
    
    /// Filter snapshots by validation
    pub fn validate_snapshots(&mut self) {
        self.snapshots.retain(|snapshot| {
            snapshot.validate().is_ok()
        });
    }
    
    /// Get snapshots sorted by age (newest first)
    pub fn get_sorted_snapshots(&self) -> Vec<TerminalSnapshot> {
        let mut snapshots = self.snapshots.clone();
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        snapshots
    }
    
    /// Get snapshots grouped by recency
    pub fn get_snapshot_summary(&self) -> SnapshotSummary {
        use std::time::Duration;
        
        let mut recent = Vec::new();
        let mut older = Vec::new();
        
        let one_day = Duration::from_secs(24 * 60 * 60);
        
        for snapshot in &self.snapshots {
            if snapshot.age() < one_day {
                recent.push(snapshot.clone());
            } else {
                older.push(snapshot.clone());
            }
        }
        
        SnapshotSummary {
            total: self.snapshots.len(),
            recent: recent.len(),
            older: older.len(),
            recent_snapshots: recent,
            older_snapshots: older,
        }
    }
}

/// Summary of snapshots grouped by age
#[derive(Debug, Clone)]
pub struct SnapshotSummary {
    /// Total snapshot count
    pub total: usize,
    
    /// Recent snapshots (< 24 hours)
    pub recent: usize,
    
    /// Older snapshots (>= 24 hours)
    pub older: usize,
    
    /// Recent snapshot list
    pub recent_snapshots: Vec<TerminalSnapshot>,
    
    /// Older snapshot list
    pub older_snapshots: Vec<TerminalSnapshot>,
}

/// Restore policy for automatic restoration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorePolicy {
    /// Never restore automatically
    Never,
    
    /// Ask user each time
    Ask,
    
    /// Always restore recent snapshots (< 24 hours)
    AlwaysRecent,
    
    /// Always restore all snapshots
    Always,
}

impl Default for RestorePolicy {
    fn default() -> Self {
        RestorePolicy::Ask
    }
}

/// Terminal restorer handles the restore flow
pub struct TerminalRestorer {
    /// Restore policy
    policy: RestorePolicy,
    
    /// Workspace path
    workspace_path: PathBuf,
}

impl TerminalRestorer {
    /// Create a new restorer
    pub fn new(workspace_path: PathBuf, policy: RestorePolicy) -> Self {
        Self {
            policy,
            workspace_path,
        }
    }
    
    /// Check if restoration should be offered based on policy
    pub fn should_offer_restore(&self, session: &RestoreSession) -> bool {
        match self.policy {
            RestorePolicy::Never => false,
            RestorePolicy::Ask => session.has_snapshots(),
            RestorePolicy::AlwaysRecent => session.has_snapshots(),
            RestorePolicy::Always => session.has_snapshots(),
        }
    }
    
    /// Get snapshots to auto-restore based on policy
    pub fn get_auto_restore_snapshots(
        &self,
        session: &RestoreSession,
    ) -> Vec<TerminalSnapshot> {
        match self.policy {
            RestorePolicy::Never | RestorePolicy::Ask => Vec::new(),
            RestorePolicy::AlwaysRecent => {
                let summary = session.get_snapshot_summary();
                summary.recent_snapshots
            }
            RestorePolicy::Always => session.snapshots.clone(),
        }
    }
    
    /// Validate a snapshot for restoration
    pub fn validate_snapshot(&self, snapshot: &TerminalSnapshot) -> Result<()> {
        // Validate snapshot data
        snapshot.validate()?;
        
        // Ensure workspace matches
        if snapshot.workspace_path != self.workspace_path {
            return Err(anyhow::anyhow!(
                "Snapshot workspace {:?} does not match current workspace {:?}",
                snapshot.workspace_path,
                self.workspace_path
            ));
        }
        
        // Ensure CWD is within workspace
        if !snapshot.cwd.starts_with(&self.workspace_path) {
            return Err(anyhow::anyhow!(
                "CWD {:?} is outside workspace {:?}",
                snapshot.cwd,
                self.workspace_path
            ));
        }
        
        Ok(())
    }
    
    /// Prepare snapshot for restoration (create directories if needed)
    pub fn prepare_snapshot(&self, snapshot: &TerminalSnapshot) -> Result<()> {
        // Ensure CWD exists, create if it doesn't
        if !snapshot.cwd.exists() {
            tracing::warn!(
                "CWD {:?} does not exist, creating it",
                snapshot.cwd
            );
            
            std::fs::create_dir_all(&snapshot.cwd)
                .with_context(|| format!(
                    "Failed to create CWD {:?}",
                    snapshot.cwd
                ))?;
        }
        
        Ok(())
    }
    
    /// Restore a single snapshot
    pub fn restore_snapshot(
        &self,
        snapshot: &TerminalSnapshot,
    ) -> Result<RestoreResult> {
        // Validate
        if let Err(e) = self.validate_snapshot(snapshot) {
            return Ok(RestoreResult::Skipped {
                term_id: snapshot.term_id.clone(),
                reason: format!("Validation failed: {}", e),
            });
        }
        
        // Prepare (create directories)
        if let Err(e) = self.prepare_snapshot(snapshot) {
            return Ok(RestoreResult::Failed {
                term_id: snapshot.term_id.clone(),
                error: format!("Preparation failed: {}", e),
            });
        }
        
        // TODO: Actual terminal restoration would happen here
        // For now, we just validate and prepare
        
        tracing::info!(
            "Restored terminal {} at {:?}",
            snapshot.term_id,
            snapshot.cwd
        );
        
        Ok(RestoreResult::Success {
            term_id: snapshot.term_id.clone(),
            cwd: snapshot.cwd.clone(),
        })
    }
    
    /// Restore multiple snapshots
    pub fn restore_snapshots(
        &self,
        snapshots: &[TerminalSnapshot],
    ) -> Vec<RestoreResult> {
        snapshots
            .iter()
            .map(|snapshot| {
                self.restore_snapshot(snapshot)
                    .unwrap_or_else(|e| RestoreResult::Failed {
                        term_id: snapshot.term_id.clone(),
                        error: e.to_string(),
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use lapce_rpc::terminal::TermId;
    use super::super::types::CommandHistory;
    
    fn create_test_snapshot(workspace: PathBuf, age_hours: i64) -> TerminalSnapshot {
        let term_id = TermId::next();
        let cwd = workspace.join("project");
        
        let mut snapshot = TerminalSnapshot::new(
            term_id,
            cwd,
            workspace,
            "Test Terminal".to_string(),
        );
        
        // Adjust timestamp for age
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        snapshot.created_at = now - (age_hours * 3600);
        
        snapshot
    }
    
    #[test]
    fn test_restore_session_creation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        // Create some snapshots
        let manager = SnapshotManager::new(workspace.clone()).unwrap();
        for i in 0..3 {
            let cwd = workspace.join(format!("project{}", i));
            std::fs::create_dir_all(&cwd).unwrap();
            
            let snapshot = TerminalSnapshot::new(
                TermId::next(),
                cwd,
                workspace.clone(),
                format!("Terminal {}", i),
            );
            manager.save(&snapshot).unwrap();
        }
        
        // Create restore session
        let session = RestoreSession::new(workspace).unwrap();
        assert_eq!(session.snapshot_count(), 3);
        assert!(session.has_snapshots());
    }
    
    #[test]
    fn test_snapshot_summary_grouping() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let recent1 = create_test_snapshot(workspace.clone(), 1); // 1 hour ago
        let recent2 = create_test_snapshot(workspace.clone(), 12); // 12 hours ago
        let older = create_test_snapshot(workspace.clone(), 48); // 2 days ago
        
        let session = RestoreSession {
            snapshots: vec![recent1, recent2, older],
            workspace_path: workspace,
        };
        
        let summary = session.get_snapshot_summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.recent, 2);
        assert_eq!(summary.older, 1);
    }
    
    #[test]
    fn test_restore_policy_should_offer() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let session = RestoreSession {
            snapshots: vec![create_test_snapshot(workspace.clone(), 1)],
            workspace_path: workspace.clone(),
        };
        
        let restorer_never = TerminalRestorer::new(
            workspace.clone(),
            RestorePolicy::Never,
        );
        assert!(!restorer_never.should_offer_restore(&session));
        
        let restorer_ask = TerminalRestorer::new(
            workspace.clone(),
            RestorePolicy::Ask,
        );
        assert!(restorer_ask.should_offer_restore(&session));
        
        let restorer_always = TerminalRestorer::new(
            workspace,
            RestorePolicy::Always,
        );
        assert!(restorer_always.should_offer_restore(&session));
    }
    
    #[test]
    fn test_auto_restore_snapshots_by_policy() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let recent = create_test_snapshot(workspace.clone(), 1);
        let older = create_test_snapshot(workspace.clone(), 48);
        
        let session = RestoreSession {
            snapshots: vec![recent.clone(), older.clone()],
            workspace_path: workspace.clone(),
        };
        
        // AlwaysRecent should only return recent
        let restorer_recent = TerminalRestorer::new(
            workspace.clone(),
            RestorePolicy::AlwaysRecent,
        );
        let auto_recent = restorer_recent.get_auto_restore_snapshots(&session);
        assert_eq!(auto_recent.len(), 1);
        
        // Always should return all
        let restorer_all = TerminalRestorer::new(
            workspace,
            RestorePolicy::Always,
        );
        let auto_all = restorer_all.get_auto_restore_snapshots(&session);
        assert_eq!(auto_all.len(), 2);
    }
    
    #[test]
    fn test_validate_snapshot_workspace_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let workspace1 = temp_dir.path().join("workspace1");
        let workspace2 = temp_dir.path().join("workspace2");
        std::fs::create_dir_all(&workspace1).unwrap();
        std::fs::create_dir_all(&workspace2).unwrap();
        
        // Create snapshot with workspace1 but valid CWD within workspace1
        let cwd = workspace1.join("project");
        std::fs::create_dir_all(&cwd).unwrap();
        
        let mut snapshot = TerminalSnapshot::new(
            TermId::next(),
            cwd,
            workspace1.clone(),
            "Test".to_string(),
        );
        snapshot.workspace_path = workspace1;
        
        // Try to restore with workspace2 (mismatch)
        let restorer = TerminalRestorer::new(
            workspace2,
            RestorePolicy::Ask,
        );
        
        let result = restorer.validate_snapshot(&snapshot);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not match"));
    }
    
    #[test]
    fn test_prepare_snapshot_creates_cwd() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let cwd = workspace.join("new_project");
        
        let snapshot = TerminalSnapshot::new(
            TermId::next(),
            cwd.clone(),
            workspace.clone(),
            "Test".to_string(),
        );
        
        assert!(!cwd.exists());
        
        let restorer = TerminalRestorer::new(workspace, RestorePolicy::Ask);
        restorer.prepare_snapshot(&snapshot).unwrap();
        
        assert!(cwd.exists());
    }
    
    #[test]
    fn test_restore_snapshot_success() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let cwd = workspace.join("project");
        std::fs::create_dir_all(&cwd).unwrap();
        
        let snapshot = TerminalSnapshot::new(
            TermId::next(),
            cwd.clone(),
            workspace.clone(),
            "Test".to_string(),
        );
        
        let restorer = TerminalRestorer::new(workspace, RestorePolicy::Ask);
        let result = restorer.restore_snapshot(&snapshot).unwrap();
        
        match result {
            RestoreResult::Success { term_id: _, cwd: restored_cwd } => {
                assert_eq!(restored_cwd, cwd);
            }
            _ => panic!("Expected Success result"),
        }
    }
    
    #[test]
    fn test_restore_multiple_snapshots() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        
        let mut snapshots = Vec::new();
        for i in 0..3 {
            let cwd = workspace.join(format!("project{}", i));
            std::fs::create_dir_all(&cwd).unwrap();
            
            snapshots.push(TerminalSnapshot::new(
                TermId::next(),
                cwd,
                workspace.clone(),
                format!("Terminal {}", i),
            ));
        }
        
        let restorer = TerminalRestorer::new(workspace, RestorePolicy::Ask);
        let results = restorer.restore_snapshots(&snapshots);
        
        assert_eq!(results.len(), 3);
        for result in results {
            assert!(matches!(result, RestoreResult::Success { .. }));
        }
    }
    
    #[test]
    fn test_validate_snapshots_filter() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let cwd_valid = workspace.join("valid");
        let cwd_invalid = PathBuf::from("/invalid/path");
        
        std::fs::create_dir_all(&cwd_valid).unwrap();
        
        let valid_snapshot = TerminalSnapshot::new(
            TermId::next(),
            cwd_valid,
            workspace.clone(),
            "Valid".to_string(),
        );
        
        let invalid_snapshot = TerminalSnapshot::new(
            TermId::next(),
            cwd_invalid,
            workspace.clone(),
            "Invalid".to_string(),
        );
        
        let mut session = RestoreSession {
            snapshots: vec![valid_snapshot, invalid_snapshot],
            workspace_path: workspace,
        };
        
        assert_eq!(session.snapshot_count(), 2);
        
        session.validate_snapshots();
        
        assert_eq!(session.snapshot_count(), 1);
    }
}
