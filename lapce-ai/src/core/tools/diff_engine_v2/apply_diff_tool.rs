// Apply diff tool V2
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::sync::Arc;
use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use crate::core::tools::streaming_v2::{UnifiedStreamEmitter, DiffStatus};
use super::DiffEngineV2;
use uuid::Uuid;

pub struct ApplyDiffToolV2 {
    engine: DiffEngineV2,
    emitter: Option<Arc<UnifiedStreamEmitter>>,
}

impl ApplyDiffToolV2 {
    pub fn new() -> Self {
        Self {
            engine: DiffEngineV2::new(),
            emitter: None,
        }
    }
    
    pub fn with_emitter(emitter: Arc<UnifiedStreamEmitter>) -> Self {
        Self {
            engine: DiffEngineV2::new(),
            emitter: Some(emitter),
        }
    }
}

#[async_trait]
impl Tool for ApplyDiffToolV2 {
    fn name(&self) -> &'static str {
        "applyDiff"
    }
    
    fn description(&self) -> &'static str {
        "Apply diff patches to files"
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        let file_path = args["file"].as_str()
            .ok_or_else(|| ToolError::InvalidInput("File path is required".to_string()))?;
        let patch = args["patch"].as_str()
            .ok_or_else(|| ToolError::InvalidInput("Patch is required".to_string()))?;
        
        // Generate correlation ID
        let correlation_id = Uuid::new_v4().to_string();
        
        // Emit analyzing event
        if let Some(ref emitter) = self.emitter {
            let _ = emitter.emit_diff_update(
                &correlation_id,
                file_path,
                0,
                1,  // Simplified: treating entire patch as 1 hunk
                DiffStatus::Analyzing,
            ).await;
        }
        
        // Read file content
        let full_path = context.workspace.join(file_path);
        let content = tokio::fs::read_to_string(&full_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;
        
        // Emit applying event
        if let Some(ref emitter) = self.emitter {
            let _ = emitter.emit_diff_update(
                &correlation_id,
                file_path,
                1,
                1,
                DiffStatus::ApplyingHunk,
            ).await;
        }
        
        // Apply patch
        let result = self.engine.apply_patch(
            &content,
            patch,
            super::DiffStrategy::Exact,
            Default::default(),
        ).await
            .map_err(|e| {
                // Emit failed event on error
                if let Some(ref emitter) = self.emitter {
                    let _ = futures::executor::block_on(emitter.emit_diff_update(
                        &correlation_id,
                        file_path,
                        1,
                        1,
                        DiffStatus::HunkFailed,
                    ));
                }
                ToolError::ExecutionFailed(format!("Failed to apply patch: {}", e))
            })?;
        
        // Write back
        tokio::fs::write(&full_path, &result.content).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;
        
        // Emit completion event
        if let Some(ref emitter) = self.emitter {
            let _ = emitter.emit_diff_update(
                &correlation_id,
                file_path,
                1,
                1,
                DiffStatus::Complete,
            ).await;
        }
        
        Ok(ToolOutput {
            success: true,
            result: json!({
                "file": file_path,
                "applied": true,
                "lines_added": result.lines_added,
                "lines_removed": result.lines_removed,
                "correlation_id": correlation_id,
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}
