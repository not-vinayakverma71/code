/// Crash Recovery & Resilience (LSP-026)
/// Graceful handling on backend restart, IPC re-establishment, document rehydration

use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use parking_lot::RwLock;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tokio::fs;
use chrono::{DateTime, Utc};

/// Document state snapshot for recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    pub uri: String,
    pub content: String,
    pub version: u32,
    pub language_id: String,
    pub last_modified: DateTime<Utc>,
}

/// Diagnostics snapshot for recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsSnapshot {
    pub uri: String,
    pub version: Option<u32>,
    pub diagnostics_json: String,
    pub timestamp: DateTime<Utc>,
}

/// Gateway state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySnapshot {
    pub documents: Vec<DocumentSnapshot>,
    pub diagnostics: Vec<DiagnosticsSnapshot>,
    pub timestamp: DateTime<Utc>,
    pub gateway_version: String,
}

impl GatewaySnapshot {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            diagnostics: Vec::new(),
            timestamp: Utc::now(),
            gateway_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Default for GatewaySnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Recovery manager for persisting and restoring gateway state
pub struct RecoveryManager {
    snapshot_path: PathBuf,
    current_snapshot: Arc<RwLock<GatewaySnapshot>>,
    auto_save_interval: std::time::Duration,
}

impl RecoveryManager {
    pub fn new(snapshot_path: PathBuf, auto_save_interval_secs: u64) -> Self {
        Self {
            snapshot_path,
            current_snapshot: Arc::new(RwLock::new(GatewaySnapshot::new())),
            auto_save_interval: std::time::Duration::from_secs(auto_save_interval_secs),
        }
    }
    
    /// Start auto-save background task
    pub fn start_auto_save(&self) -> tokio::task::JoinHandle<()> {
        let snapshot_path = self.snapshot_path.clone();
        let current_snapshot = self.current_snapshot.clone();
        let interval = self.auto_save_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                interval.tick().await;
                
                let snapshot = current_snapshot.read().clone();
                
                if let Err(e) = Self::save_snapshot_to_disk(&snapshot_path, &snapshot).await {
                    tracing::error!(
                        error = %e,
                        path = %snapshot_path.display(),
                        "Failed to auto-save gateway snapshot"
                    );
                }
            }
        })
    }
    
    /// Update document in snapshot
    pub fn update_document(&self, doc: DocumentSnapshot) {
        let mut snapshot = self.current_snapshot.write();
        
        // Remove old version
        snapshot.documents.retain(|d| d.uri != doc.uri);
        
        // Add new version
        snapshot.documents.push(doc);
        snapshot.timestamp = Utc::now();
        
        tracing::debug!(
            uri = %snapshot.documents.last().unwrap().uri,
            document_count = snapshot.documents.len(),
            "Updated document snapshot"
        );
    }
    
    /// Remove document from snapshot
    pub fn remove_document(&self, uri: &str) {
        let mut snapshot = self.current_snapshot.write();
        snapshot.documents.retain(|d| d.uri != uri);
        snapshot.diagnostics.retain(|d| d.uri != uri);
        snapshot.timestamp = Utc::now();
        
        tracing::debug!(uri = %uri, "Removed document from snapshot");
    }
    
    /// Update diagnostics in snapshot
    pub fn update_diagnostics(&self, diag: DiagnosticsSnapshot) {
        let mut snapshot = self.current_snapshot.write();
        
        // Remove old version
        snapshot.diagnostics.retain(|d| d.uri != diag.uri);
        
        // Add new version
        snapshot.diagnostics.push(diag);
        snapshot.timestamp = Utc::now();
    }
    
    /// Save snapshot to disk
    pub async fn save_snapshot(&self) -> Result<()> {
        let snapshot = self.current_snapshot.read().clone();
        Self::save_snapshot_to_disk(&self.snapshot_path, &snapshot).await
    }
    
    async fn save_snapshot_to_disk(path: &PathBuf, snapshot: &GatewaySnapshot) -> Result<()> {
        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Serialize snapshot
        let json = serde_json::to_string_pretty(snapshot)?;
        
        // Write to temp file first
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, json).await?;
        
        // Atomic rename
        fs::rename(&temp_path, path).await?;
        
        tracing::info!(
            path = %path.display(),
            documents = snapshot.documents.len(),
            diagnostics = snapshot.diagnostics.len(),
            "Saved gateway snapshot"
        );
        
        Ok(())
    }
    
    /// Load snapshot from disk
    pub async fn load_snapshot(&self) -> Result<GatewaySnapshot> {
        if !self.snapshot_path.exists() {
            return Ok(GatewaySnapshot::new());
        }
        
        let json = fs::read_to_string(&self.snapshot_path).await?;
        let snapshot: GatewaySnapshot = serde_json::from_str(&json)?;
        
        tracing::info!(
            path = %self.snapshot_path.display(),
            documents = snapshot.documents.len(),
            diagnostics = snapshot.diagnostics.len(),
            age_secs = (Utc::now() - snapshot.timestamp).num_seconds(),
            "Loaded gateway snapshot"
        );
        
        // Update current snapshot
        *self.current_snapshot.write() = snapshot.clone();
        
        Ok(snapshot)
    }
    
    /// Clear snapshot
    pub async fn clear_snapshot(&self) -> Result<()> {
        *self.current_snapshot.write() = GatewaySnapshot::new();
        
        if self.snapshot_path.exists() {
            fs::remove_file(&self.snapshot_path).await?;
            tracing::info!("Cleared gateway snapshot");
        }
        
        Ok(())
    }
    
    /// Get current snapshot stats
    pub fn snapshot_stats(&self) -> (usize, usize, DateTime<Utc>) {
        let snapshot = self.current_snapshot.read();
        (snapshot.documents.len(), snapshot.diagnostics.len(), snapshot.timestamp)
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        let snapshot_path = std::env::temp_dir().join("lapce-lsp-gateway-snapshot.json");
        Self::new(snapshot_path, 30) // Auto-save every 30 seconds
    }
}

/// IPC reconnection handler
pub struct IpcReconnectionHandler {
    max_retries: usize,
    retry_delay_ms: u64,
    backoff_multiplier: f64,
}

impl IpcReconnectionHandler {
    pub fn new(max_retries: usize, retry_delay_ms: u64, backoff_multiplier: f64) -> Self {
        Self {
            max_retries,
            retry_delay_ms,
            backoff_multiplier,
        }
    }
    
    /// Attempt to reconnect with exponential backoff
    pub async fn reconnect<F, Fut>(&self, connect_fn: F) -> Result<()>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut attempt = 0;
        let mut delay_ms = self.retry_delay_ms;
        
        loop {
            match connect_fn().await {
                Ok(_) => {
                    tracing::info!(attempt = attempt, "IPC reconnection successful");
                    return Ok(());
                }
                Err(e) => {
                    attempt += 1;
                    
                    if attempt >= self.max_retries {
                        tracing::error!(
                            error = %e,
                            attempts = attempt,
                            "IPC reconnection failed after max retries"
                        );
                        return Err(anyhow!("Max reconnection attempts exceeded: {}", e));
                    }
                    
                    tracing::warn!(
                        error = %e,
                        attempt = attempt,
                        max_retries = self.max_retries,
                        delay_ms = delay_ms,
                        "IPC reconnection attempt failed, retrying"
                    );
                    
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    
                    // Exponential backoff
                    delay_ms = (delay_ms as f64 * self.backoff_multiplier) as u64;
                }
            }
        }
    }
}

impl Default for IpcReconnectionHandler {
    fn default() -> Self {
        Self::new(5, 1000, 2.0) // 5 retries, 1s initial delay, 2x backoff
    }
}

/// Document rehydration manager
pub struct DocumentRehydrationManager {
    recovery_manager: Arc<RecoveryManager>,
}

impl DocumentRehydrationManager {
    pub fn new(recovery_manager: Arc<RecoveryManager>) -> Self {
        Self { recovery_manager }
    }
    
    /// Rehydrate documents from snapshot
    pub async fn rehydrate_documents(&self) -> Result<Vec<DocumentSnapshot>> {
        let snapshot = self.recovery_manager.load_snapshot().await?;
        
        if snapshot.documents.is_empty() {
            tracing::info!("No documents to rehydrate");
            return Ok(Vec::new());
        }
        
        tracing::info!(
            document_count = snapshot.documents.len(),
            snapshot_age_secs = (Utc::now() - snapshot.timestamp).num_seconds(),
            "Rehydrating documents from snapshot"
        );
        
        Ok(snapshot.documents)
    }
    
    /// Rehydrate diagnostics from snapshot
    pub async fn rehydrate_diagnostics(&self) -> Result<Vec<DiagnosticsSnapshot>> {
        let snapshot = self.recovery_manager.load_snapshot().await?;
        
        if snapshot.diagnostics.is_empty() {
            tracing::info!("No diagnostics to rehydrate");
            return Ok(Vec::new());
        }
        
        tracing::info!(
            diagnostics_count = snapshot.diagnostics.len(),
            "Rehydrating diagnostics from snapshot"
        );
        
        Ok(snapshot.diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_recovery_manager_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_path = temp_dir.path().join("snapshot.json");
        
        let manager = RecoveryManager::new(snapshot_path.clone(), 60);
        
        // Add document
        manager.update_document(DocumentSnapshot {
            uri: "file:///test.rs".to_string(),
            content: "fn main() {}".to_string(),
            version: 1,
            language_id: "rust".to_string(),
            last_modified: Utc::now(),
        });
        
        // Save
        manager.save_snapshot().await.unwrap();
        
        // Load
        let snapshot = manager.load_snapshot().await.unwrap();
        assert_eq!(snapshot.documents.len(), 1);
        assert_eq!(snapshot.documents[0].uri, "file:///test.rs");
    }
    
    #[tokio::test]
    async fn test_recovery_manager_remove_document() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_path = temp_dir.path().join("snapshot.json");
        
        let manager = RecoveryManager::new(snapshot_path, 60);
        
        manager.update_document(DocumentSnapshot {
            uri: "file:///test.rs".to_string(),
            content: "fn main() {}".to_string(),
            version: 1,
            language_id: "rust".to_string(),
            last_modified: Utc::now(),
        });
        
        manager.remove_document("file:///test.rs");
        
        let (doc_count, _, _) = manager.snapshot_stats();
        assert_eq!(doc_count, 0);
    }
    
    #[tokio::test]
    async fn test_ipc_reconnection_success() {
        let handler = IpcReconnectionHandler::new(3, 10, 1.5);
        
        let mut attempt = 0;
        let result = handler.reconnect(|| async {
            attempt += 1;
            if attempt >= 2 {
                Ok(())
            } else {
                Err(anyhow!("Connection failed"))
            }
        }).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_ipc_reconnection_failure() {
        let handler = IpcReconnectionHandler::new(2, 10, 1.5);
        
        let result = handler.reconnect(|| async {
            Err(anyhow!("Connection failed"))
        }).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_document_rehydration() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_path = temp_dir.path().join("snapshot.json");
        
        let recovery_manager = Arc::new(RecoveryManager::new(snapshot_path, 60));
        
        recovery_manager.update_document(DocumentSnapshot {
            uri: "file:///test.rs".to_string(),
            content: "fn main() {}".to_string(),
            version: 1,
            language_id: "rust".to_string(),
            last_modified: Utc::now(),
        });
        
        recovery_manager.save_snapshot().await.unwrap();
        
        let rehydration_manager = DocumentRehydrationManager::new(recovery_manager);
        let documents = rehydration_manager.rehydrate_documents().await.unwrap();
        
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].uri, "file:///test.rs");
    }
}
