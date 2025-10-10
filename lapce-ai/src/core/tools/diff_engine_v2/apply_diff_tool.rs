// Apply diff tool V2
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use crate::core::tools::traits::{Tool, ToolContext, ToolResult, ToolOutput, ToolError};
use super::DiffEngineV2;

pub struct ApplyDiffToolV2 {
    engine: DiffEngineV2,
}

impl ApplyDiffToolV2 {
    pub fn new() -> Self {
        Self {
            engine: DiffEngineV2::new(),
        }
    }
}

#[async_trait]
impl Tool for ApplyDiffToolV2 {
    fn name(&self) -> &'static str {
        "apply_diff"
    }
    
    fn description(&self) -> &'static str {
        "Apply diff patches to files"
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> ToolResult {
        let file_path = args["file"].as_str()
            .ok_or_else(|| ToolError::InvalidInput("File path is required".to_string()))?;
        let patch = args["patch"].as_str()
            .ok_or_else(|| ToolError::InvalidInput("Patch is required".to_string()))?;
        
        // Read file content
        let full_path = context.workspace.join(file_path);
        let content = tokio::fs::read_to_string(&full_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;
        
        // Apply patch
        let result = self.engine.apply_patch(
            &content,
            patch,
            super::DiffStrategy::Exact,
            Default::default(),
        ).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to apply patch: {}", e)))?;
        
        // Write back
        tokio::fs::write(&full_path, &result.content).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;
        
        Ok(ToolOutput {
            success: true,
            result: json!({
                "file": file_path,
                "applied": true,
                "lines_added": result.lines_added,
                "lines_removed": result.lines_removed,
            }),
            error: None,
            metadata: Default::default(),
        })
    }
}
