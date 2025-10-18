//! Workflows Management
//!
//! Direct 1:1 port from Codex/src/core/context/instructions/workflows.ts
//! Lines 1-43 complete
//!
//! Manages global and local workflow toggles, synchronized with filesystem.
//! NOTE: VSCode-specific APIs replaced with IPC-ready equivalents.

use std::path::PathBuf;

use super::rule_helpers::{synchronize_rule_toggles, RuleToggles};

/// Global file names (mirroring GlobalFileNames from Codex)
/// TODO: Move to shared constants module
pub const WORKFLOWS_DIR_NAME: &str = ".workflows";

/// Refreshes local workflow toggles for a workspace
///
/// Port of refreshLocalWorkflowToggles() from workflows.ts lines 9-20
/// NOTE: Original uses VSCode ExtensionContext and ContextProxy
///       This version uses IPC-ready state management
///
/// # Arguments
/// * `workspace_state` - Persistent workspace state (to be provided via IPC)
/// * `working_directory` - Current workspace directory
///
/// # Returns
/// Updated local workflow toggles
pub async fn refresh_local_workflow_toggles(
    workspace_state: &mut dyn WorkspaceState,
    working_directory: &PathBuf,
) -> RuleToggles {
    let workflow_toggles = workspace_state
        .get_workspace_state("localWorkflowToggles")
        .await
        .unwrap_or_default();
    
    let workflows_dir_path = working_directory.join(WORKFLOWS_DIR_NAME);
    
    let updated_workflow_toggles = synchronize_rule_toggles(
        &workflows_dir_path,
        workflow_toggles,
        "",
        &[],
    )
    .await;
    
    workspace_state
        .update_workspace_state("localWorkflowToggles", updated_workflow_toggles.clone())
        .await;
    
    updated_workflow_toggles
}

/// Refreshes global workflow toggles
///
/// Port of refreshGlobalWorkflowToggles() from workflows.ts lines 22-28
///
/// # Arguments
/// * `global_state` - Persistent global state (to be provided via IPC)
///
/// # Returns
/// Updated global workflow toggles
pub async fn refresh_global_workflow_toggles(
    global_state: &mut dyn GlobalState,
) -> RuleToggles {
    let global_workflow_toggles = global_state
        .get_global_state("globalWorkflowToggles")
        .await
        .unwrap_or_default();
    
    // Global workflows dir: ~/.workflows
    let home_dir = dirs::home_dir().expect("Cannot determine home directory");
    let global_workflows_dir = home_dir.join(WORKFLOWS_DIR_NAME);
    
    let updated_global_workflow_toggles = synchronize_rule_toggles(
        &global_workflows_dir,
        global_workflow_toggles,
        "",
        &[],
    )
    .await;
    
    global_state
        .update_global_state("globalWorkflowToggles", updated_global_workflow_toggles.clone())
        .await;
    
    updated_global_workflow_toggles
}

/// Combined workflow toggles result
#[derive(Debug, Clone)]
pub struct WorkflowTogglesResult {
    pub global_workflow_toggles: RuleToggles,
    pub local_workflow_toggles: RuleToggles,
}

/// Refreshes both global and local workflow toggles
///
/// Port of refreshWorkflowToggles() from workflows.ts lines 30-42
///
/// # Arguments
/// * `global_state` - Persistent global state
/// * `workspace_state` - Persistent workspace state
/// * `working_directory` - Current workspace directory
///
/// # Returns
/// Combined global and local workflow toggles
pub async fn refresh_workflow_toggles(
    global_state: &mut dyn GlobalState,
    workspace_state: &mut dyn WorkspaceState,
    working_directory: &PathBuf,
) -> WorkflowTogglesResult {
    let global_workflow_toggles = refresh_global_workflow_toggles(global_state).await;
    let local_workflow_toggles = refresh_local_workflow_toggles(workspace_state, working_directory).await;
    
    WorkflowTogglesResult {
        global_workflow_toggles,
        local_workflow_toggles,
    }
}

// Trait definitions for state management (IPC-ready)
// These will be implemented by the IPC bridge on the Lapce side

#[async_trait::async_trait]
pub trait GlobalState {
    async fn get_global_state(&self, key: &str) -> Option<RuleToggles>;
    async fn update_global_state(&mut self, key: &str, value: RuleToggles);
}

#[async_trait::async_trait]
pub trait WorkspaceState {
    async fn get_workspace_state(&self, key: &str) -> Option<RuleToggles>;
    async fn update_workspace_state(&mut self, key: &str, value: RuleToggles);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    // Mock implementations for testing
    struct MockGlobalState {
        data: HashMap<String, RuleToggles>,
    }
    
    #[async_trait::async_trait]
    impl GlobalState for MockGlobalState {
        async fn get_global_state(&self, key: &str) -> Option<RuleToggles> {
            self.data.get(key).cloned()
        }
        
        async fn update_global_state(&mut self, key: &str, value: RuleToggles) {
            self.data.insert(key.to_string(), value);
        }
    }
    
    struct MockWorkspaceState {
        data: HashMap<String, RuleToggles>,
    }
    
    #[async_trait::async_trait]
    impl WorkspaceState for MockWorkspaceState {
        async fn get_workspace_state(&self, key: &str) -> Option<RuleToggles> {
            self.data.get(key).cloned()
        }
        
        async fn update_workspace_state(&mut self, key: &str, value: RuleToggles) {
            self.data.insert(key.to_string(), value);
        }
    }
    
    #[tokio::test]
    async fn test_refresh_local_workflow_toggles() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path().to_path_buf();
        
        // Create .workflows directory with a workflow file
        let workflows_dir = working_dir.join(WORKFLOWS_DIR_NAME);
        tokio::fs::create_dir_all(&workflows_dir).await.unwrap();
        tokio::fs::write(workflows_dir.join("workflow1.md"), "content")
            .await
            .unwrap();
        
        let mut workspace_state = MockWorkspaceState {
            data: HashMap::new(),
        };
        
        let toggles = refresh_local_workflow_toggles(&mut workspace_state, &working_dir).await;
        
        // Should have detected the workflow file
        assert!(!toggles.is_empty());
    }
}
