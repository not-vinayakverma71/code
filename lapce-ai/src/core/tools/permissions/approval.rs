// Approval system for destructive operations - P1-5
// Integrates with UI for user consent

use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// Approval request for a tool operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub tool_name: String,
    pub operation: String,
    pub description: String,
    pub affected_files: Vec<String>,
    pub risk_level: RiskLevel,
    pub timestamp: u64,
}

/// Risk level for operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,     // Read-only operations
    Medium,  // Modifying single files
    High,    // Bulk operations, system commands
    Critical // Irreversible operations
}

/// Approval decision from user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub request_id: String,
    pub approved: bool,
    pub remember_choice: bool,
    pub scope: ApprovalScope,
}

/// Scope of approval
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalScope {
    ThisOperation,
    ThisSession,
    ThisTool,
    Always,
}

/// Approval manager
pub struct ApprovalManager {
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<ApprovalDecision>>>>,
    remembered: Arc<Mutex<HashMap<String, ApprovalDecision>>>,
    ui_tx: Option<mpsc::UnboundedSender<ApprovalRequest>>,
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
            remembered: Arc::new(Mutex::new(HashMap::new())),
            ui_tx: None,
        }
    }
    
    /// Set UI channel for approval requests
    pub fn set_ui_channel(&mut self, tx: mpsc::UnboundedSender<ApprovalRequest>) {
        self.ui_tx = Some(tx);
    }
    
    /// Request approval for an operation
    pub async fn request_approval(&self, request: ApprovalRequest) -> Result<bool> {
        // Check remembered decisions
        let remembered = self.remembered.lock().await;
        if let Some(decision) = remembered.get(&request.tool_name) {
            if decision.scope == ApprovalScope::Always || 
               decision.scope == ApprovalScope::ThisTool {
                return Ok(decision.approved);
            }
        }
        drop(remembered);
        
        // Send to UI if channel available
        if let Some(tx) = &self.ui_tx {
            let (response_tx, response_rx) = oneshot::channel();
            
            // Store pending request
            self.pending.lock().await.insert(request.id.clone(), response_tx);
            
            // Send to UI
            tx.send(request.clone())?;
            
            // Wait for response
            match response_rx.await {
                Ok(decision) => {
                    // Remember if requested
                    if decision.remember_choice {
                        self.remembered.lock().await.insert(
                            request.tool_name.clone(),
                            decision.clone()
                        );
                    }
                    Ok(decision.approved)
                }
                Err(_) => {
                    // Timeout or cancelled
                    Ok(false)
                }
            }
        } else {
            // No UI channel, auto-approve based on risk
            Ok(request.risk_level == RiskLevel::Low)
        }
    }
    
    /// Handle approval decision from UI
    pub async fn handle_decision(&self, decision: ApprovalDecision) -> Result<()> {
        let mut pending = self.pending.lock().await;
        if let Some(tx) = pending.remove(&decision.request_id) {
            let _ = tx.send(decision);
        }
        Ok(())
    }
    
    /// Clear remembered approvals
    pub async fn clear_remembered(&self) {
        self.remembered.lock().await.clear();
    }
}

/// Global approval manager instance
lazy_static::lazy_static! {
    pub static ref APPROVAL_MANAGER: Arc<Mutex<ApprovalManager>> = 
        Arc::new(Mutex::new(ApprovalManager::new()));
}

/// Helper to check if operation needs approval
pub fn needs_approval(tool_name: &str, operation: &str) -> bool {
    match tool_name {
        "read_file" | "list_files" | "search_files" => false,
        "write_to_file" | "edit_file" | "apply_diff" => true,
        "execute_command" => true,
        _ => true, // Default to requiring approval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_approval_manager() {
        let manager = ApprovalManager::new();
        
        let request = ApprovalRequest {
            id: "test-123".to_string(),
            tool_name: "write_to_file".to_string(),
            operation: "write".to_string(),
            description: "Writing to test.txt".to_string(),
            affected_files: vec!["test.txt".to_string()],
            risk_level: RiskLevel::Medium,
            timestamp: 0,
        };
        
        // Without UI channel, should auto-deny medium risk
        let result = manager.request_approval(request).await.unwrap();
        assert_eq!(result, false);
    }
}
