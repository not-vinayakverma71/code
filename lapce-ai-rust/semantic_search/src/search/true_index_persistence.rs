// True LanceDB Index Persistence Implementation
// This module properly leverages Lance's internal index storage

use crate::error::{Error, Result};
use crate::Table;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};

/// Index state tracker for true persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexState {
    pub table_name: String,
    pub index_name: String,
    pub index_type: String,
    pub columns: Vec<String>,
    pub num_partitions: usize,
    pub num_sub_vectors: usize,
    pub created_at: i64,
    pub row_count: usize,
    pub index_uuid: String,
}

/// True index persistence manager that works with Lance's internal structures
pub struct TrueIndexPersistence {
    base_path: PathBuf,
    state_file: PathBuf,
}

impl TrueIndexPersistence {
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let state_file = base_path.join(".index_state.json");
        
        Ok(Self {
            base_path,
            state_file,
        })
    }
    
    /// Check if an index truly exists in Lance's internal structure
    pub async fn index_exists_in_lance(&self, table: &Arc<Table>) -> Result<bool> {
        // List all indices from the table
        match table.list_indices().await {
            Ok(indices) => {
                debug!("Found {} indices in table", indices.len());
                
                // Check if any vector index exists
                for index in &indices {
                    let type_str = format!("{:?}", index.index_type);
                if type_str.contains("IVF") || type_str.contains("Pq") {
                        info!("Found existing vector index: {} (type: {})", 
                            index.name, index.index_type);
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Err(e) => {
                warn!("Failed to list indices: {}", e);
                Ok(false)
            }
        }
    }
    
    /// Save index state after creation
    pub async fn save_index_state(&self, state: &IndexState) -> Result<()> {
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| Error::Runtime { 
                message: format!("Failed to serialize index state: {}", e) 
            })?;
        
        fs::write(&self.state_file, json).await
            .map_err(|e| Error::Runtime { 
                message: format!("Failed to write index state: {}", e) 
            })?;
        
        info!("Saved index state to {:?}", self.state_file);
        Ok(())
    }
    
    /// Load saved index state
    pub async fn load_index_state(&self) -> Result<Option<IndexState>> {
        if !self.state_file.exists() {
            return Ok(None);
        }
        
        let json = fs::read_to_string(&self.state_file).await
            .map_err(|e| Error::Runtime { 
                message: format!("Failed to read index state: {}", e) 
            })?;
        
        let state: IndexState = serde_json::from_str(&json)
            .map_err(|e| Error::Runtime { 
                message: format!("Failed to parse index state: {}", e) 
            })?;
        
        Ok(Some(state))
    }
    
    /// Verify index integrity by checking Lance dataset structure
    pub async fn verify_index_integrity(
        &self, 
        table_path: &Path,
        expected_state: &IndexState
    ) -> Result<bool> {
        // Check if _indices directory exists in the Lance dataset
        let indices_path = table_path.join("_indices");
        
        if !indices_path.exists() {
            info!("No _indices directory found at {:?}", indices_path);
            return Ok(false);
        }
        
        // Check for index UUID directory
        let index_path = indices_path.join(&expected_state.index_uuid);
        if index_path.exists() {
            info!("Found index directory at {:?}", index_path);
            
            // Check for index metadata file
            let metadata_path = index_path.join("metadata.json");
            if metadata_path.exists() {
                debug!("Index metadata found, index is valid");
                return Ok(true);
            }
        }
        
        warn!("Index directory or metadata not found");
        Ok(false)
    }
    
    /// Force index loading by prewarming
    pub async fn prewarm_index(&self, table: &Arc<Table>, index_name: &str) -> Result<()> {
        info!("Prewarming index '{}'", index_name);
        
        match table.prewarm_index(index_name).await {
            Ok(_) => {
                info!("Successfully prewarmed index '{}'", index_name);
                Ok(())
            }
            Err(e) => {
                // Try with default name if custom name fails
                if index_name != "vector_idx" {
                    warn!("Failed to prewarm '{}', trying 'vector_idx': {}", index_name, e);
                    table.prewarm_index("vector_idx").await
                        .map_err(|e| Error::Runtime { 
                            message: format!("Failed to prewarm index: {}", e) 
                        })?;
                    Ok(())
                } else {
                    Err(Error::Runtime { 
                        message: format!("Failed to prewarm index: {}", e) 
                    })
                }
            }
        }
    }
    
    /// Get index statistics for performance monitoring
    pub async fn get_index_stats(&self, table: &Arc<Table>) -> Result<Vec<IndexStats>> {
        let indices = table.list_indices().await
            .map_err(|e| Error::Runtime { 
                message: format!("Failed to list indices: {}", e) 
            })?;
        
        let mut stats = Vec::new();
        
        for index in indices {
            if let Ok(Some(index_stats)) = table.index_stats(&index.name).await {
                stats.push(IndexStats {
                    name: index.name,
                    index_type: format!("{:?}", index.index_type),
                    num_indexed_rows: index_stats.num_indexed_rows,
                    num_unindexed_rows: index_stats.num_unindexed_rows,
                });
            }
        }
        
        Ok(stats)
    }
}

#[derive(Debug)]
pub struct IndexStats {
    pub name: String,
    pub index_type: String,
    pub num_indexed_rows: usize,
    pub num_unindexed_rows: usize,
}
