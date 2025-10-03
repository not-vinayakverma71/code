// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Translation of interfaces/manager.ts (Lines 1-81) - 100% EXACT

use crate::error::Result;
use crate::table::vector_store_interface::VectorStoreSearchResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Lines 7-70: ICodeIndexManager interface
#[async_trait::async_trait]
pub trait ICodeIndexManager: Send + Sync {
    /// Lines 11-15: Event emitted when progress is updated
    fn on_progress_update(&self) -> broadcast::Receiver<ProgressEvent>;
    
    /// Lines 17-20: Current state of the indexing process
    fn state(&self) -> IndexingState;
    
    /// Lines 22-25: Whether the code indexing feature is enabled
    fn is_feature_enabled(&self) -> bool;
    
    /// Lines 27-30: Whether the code indexing feature is configured  
    fn is_feature_configured(&self) -> bool;
    
    /// Lines 32-35: Loads configuration from storage
    async fn load_configuration(&mut self) -> Result<()>;
    
    /// Lines 37-40: Starts the indexing process
    async fn start_indexing(&mut self) -> Result<()>;
    
    /// Lines 42-45: Stops the file watcher
    fn stop_watcher(&self);
    
    /// Lines 47-50: Clears the index data
    async fn clear_index_data(&self) -> Result<()>;
    
    /// Lines 52-58: Searches the index
    /// 
    /// # Arguments
    /// * `query` - Query string
    /// * `limit` - Maximum number of results to return
    async fn search_index(&self, query: &str, limit: usize) -> Result<Vec<VectorStoreSearchResult>>;
    
    /// Lines 60-64: Gets the current status of the indexing system
    fn get_current_status(&self) -> IndexStatus;
    
    /// Lines 66-69: Disposes of resources used by the manager
    fn dispose(&self);
}

/// Line 72: IndexingState type definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexingState {
    Standby,
    Indexing,
    Indexed,
    Error,
}

/// Line 73: EmbedderProvider type definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmbedderProvider {
    #[serde(rename = "openai")]
    OpenAi,
    Ollama,
    #[serde(rename = "openai-compatible")]
    OpenAiCompatible,
    Gemini,
    Mistral,
}

/// Lines 75-80: IndexProgressUpdate interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexProgressUpdate {
    #[serde(rename = "systemStatus")]
    pub system_status: IndexingState,
    pub message: Option<String>,
    #[serde(rename = "processedBlockCount")]
    pub processed_block_count: Option<usize>,
    #[serde(rename = "totalBlockCount")]
    pub total_block_count: Option<usize>,
}

/// Progress event structure for notifications
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub system_status: IndexingState,
    pub file_statuses: HashMap<String, String>,
    pub message: Option<String>,
}

/// Index status structure
#[derive(Debug, Clone)]
pub struct IndexStatus {
    pub system_status: IndexingState,
    pub file_statuses: HashMap<String, String>,
    pub message: Option<String>,
}
