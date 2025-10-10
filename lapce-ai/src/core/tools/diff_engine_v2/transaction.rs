// Diff transaction module for multi-file operations
use super::*;
use anyhow::{Result, Context, anyhow};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Type aliases for compatibility
type UnifiedPatchV2 = String;
type PatchResult = super::DiffResult;

use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
struct TransactionOp {
    path: PathBuf,
    patch: UnifiedPatchV2,
    original_content: String,
}

/// Transaction for multi-file operations with rollback support
pub struct DiffTransaction {
    operations: Vec<TransactionOp>,
    rollback_data: Arc<RwLock<HashMap<PathBuf, String>>>,
    committed: bool,
}

impl DiffTransaction {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            rollback_data: Arc::new(RwLock::new(HashMap::new())),
            committed: false,
        }
    }
    
    /// Add operation to transaction
    pub fn add_operation(&mut self, path: PathBuf, patch: UnifiedPatchV2, content: String) {
        if let Ok(mut data) = self.rollback_data.write() {
            data.insert(path.clone(), content.clone());
        }
        self.operations.push(TransactionOp {
            path,
            patch,
            original_content: content,
        });
    }
    
    /// Apply all operations with automatic rollback on failure
    pub async fn apply<F>(&mut self, engine: &DiffEngineV2, strategy: DiffStrategy, apply_fn: F) -> Result<Vec<PatchResult>>
    where
        F: Fn(&Path, &str) -> Result<()>,
    {
        let mut results = Vec::new();
        let mut applied_paths: Vec<PathBuf> = Vec::new();
        
        for op in &self.operations {
            let result = engine.apply_patch(
                &op.original_content,
                &op.patch,
                strategy,
                Default::default(),
            ).await?;
            
            // Check if patch was successful (simplified check)
            if result.content.is_empty() {
                // Rollback on failure
                for path in &applied_paths {
                    if let Ok(data) = self.rollback_data.read() {
                        if let Some(original) = data.get(path) {
                            let _ = apply_fn(path, original);
                        }
                    }
                }
                return Err(anyhow!("Transaction failed at {:?}", op.path));
            }
            
            if !result.content.is_empty() {
                let content = &result.content;
                apply_fn(&op.path, content)?;
                applied_paths.push(op.path.clone());
            }
            
            results.push(result);
        }
        
        self.committed = true;
        Ok(results)
    }
    
    /// Commit the transaction (marks as committed, prevents rollback)
    pub fn commit(&mut self) -> Result<()> {
        if self.committed {
            return Err(anyhow!("Transaction already committed"));
        }
        self.committed = true;
        Ok(())
    }
    
    /// Rollback transaction (restore original content)
    pub fn rollback<F>(&self, apply_fn: F) -> Result<()>
    where
        F: Fn(&Path, &str) -> Result<()>,
    {
        if self.committed {
            return Err(anyhow!("Cannot rollback committed transaction"));
        }
        
        if let Ok(data) = self.rollback_data.read() {
            for (path, content) in data.iter() {
                apply_fn(path, content)?;
            }
        }
        
        Ok(())
    }
    
    /// Check if transaction is committed
    pub fn is_committed(&self) -> bool {
        self.committed
    }
    
    /// Get number of operations in transaction
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}
