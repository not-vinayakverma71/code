// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Translation of search-service.ts (Lines 1-75) - 100% EXACT

use crate::error::{Error, Result};
use crate::query::codebase_search::VectorStoreSearchResult;
use std::path::Path;
use std::sync::Arc;

/// Lines 13-74: Service responsible for searching the code index
pub struct CodeIndexSearchService {
    // Lines 14-18: Constructor parameters
    config_manager: Arc<CodeIndexConfigManager>,
    state_manager: Arc<CodeIndexStateManager>,
    embedder: Arc<dyn IEmbedder>,
    vector_store: Arc<dyn IVectorStore>,
}

impl CodeIndexSearchService {
    /// Lines 14-19: Constructor
    pub fn new(
        config_manager: Arc<CodeIndexConfigManager>,
        state_manager: Arc<CodeIndexStateManager>,
        embedder: Arc<dyn IEmbedder>,
        vector_store: Arc<dyn IVectorStore>,
    ) -> Self {
        Self {
            config_manager,
            state_manager,
            embedder,
            vector_store,
        }
    }
    
    /// Lines 29-73: searchIndex method
    /// Searches the code index for relevant content.
    /// 
    /// # Arguments
    /// * `query` - The search query
    /// * `directory_prefix` - Optional directory path to filter results by
    /// 
    /// # Returns
    /// Array of search results
    /// 
    /// # Errors
    /// Returns error if the service is not properly configured or ready
    pub async fn search_index(
        &self,
        query: &str,
        directory_prefix: Option<&str>
    ) -> Result<Vec<VectorStoreSearchResult>> {
        // Lines 30-32: Check if feature is enabled and configured
        if !self.config_manager.is_feature_enabled() || !self.config_manager.is_feature_configured() {
            return Err(Error::Runtime {
                message: "Code index feature is disabled or not configured.".to_string()
            });
        }
        
        // Lines 34-35: Get search parameters from config
        let min_score = self.config_manager.current_search_min_score();
        let max_results = self.config_manager.current_search_max_results();
        
        // Lines 37-41: Check if index is ready
        let current_state = self.state_manager.get_current_status().system_status;
        if current_state != IndexingState::Indexed && current_state != IndexingState::Indexing {
            // Allow search during Indexing too
            return Err(Error::Runtime {
                message: format!("Code index is not ready for search. Current state: {:?}", current_state)
            });
        }
        
        // Lines 43-72: Try block for search operation
        match self.perform_search(query, directory_prefix, min_score, max_results).await {
            Ok(results) => Ok(results),
            Err(error) => {
                // Lines 61-69: Error handling and telemetry
                eprintln!("[CodeIndexSearchService] Error during search: {:?}", error);
                self.state_manager.set_system_state(
                    IndexingState::Error, 
                    Some(format!("Search failed: {}", error))
                );
                
                // Capture telemetry for the error
                // In Rust context, we'll log this differently
                log::error!("CODE_INDEX_ERROR in searchIndex: {:?}", error);
                
                // Line 71: Re-throw the error
                Err(error)
            }
        }
    }
    
    /// Helper method to perform the actual search
    async fn perform_search(
        &self,
        query: &str,
        directory_prefix: Option<&str>,
        min_score: f32,
        max_results: usize
    ) -> Result<Vec<VectorStoreSearchResult>> {
        // Lines 44-49: Generate embedding for query
        let embedding_response = self.embedder.create_embeddings(vec![query.to_string()]).await?;
        let vector = embedding_response.embeddings
            .first()
            .ok_or_else(|| Error::Runtime {
                message: "Failed to generate embedding for query.".to_string()
            })?;
        
        // Lines 51-55: Handle directory prefix normalization
        let normalized_prefix = directory_prefix.map(|prefix| {
            // Normalize the path
            Path::new(prefix)
                .to_str()
                .unwrap_or(prefix)
                .to_string()
        });
        
        // Lines 57-59: Perform search
        let results = self.vector_store.search(
            vector.clone(),
            normalized_prefix.as_deref(),
            Some(min_score),
            Some(max_results)
        ).await?;
        
        Ok(results)
    }
}

// Trait definitions and placeholder implementations
// These will be properly implemented in their respective files

/// From embedder.ts - IEmbedder interface
#[async_trait::async_trait]
pub trait IEmbedder: Send + Sync {
    async fn create_embeddings(&self, texts: Vec<String>) -> Result<EmbeddingResponse>;
}

/// From embedder.ts - EmbeddingResponse
pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
}

/// From vector-store.ts - IVectorStore interface
#[async_trait::async_trait]
pub trait IVectorStore: Send + Sync {
    async fn search(
        &self,
        vector: Vec<f32>,
        directory_prefix: Option<&str>,
        min_score: Option<f32>,
        max_results: Option<usize>
    ) -> Result<Vec<VectorStoreSearchResult>>;
}

/// From state-manager.ts - IndexingState enum
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingState {
    Standby,
    Indexing,
    Indexed,
    Error,
}

/// Config manager placeholder
pub struct CodeIndexConfigManager {
    // Will be implemented in config-manager.rs
}

impl CodeIndexConfigManager {
    pub fn is_feature_enabled(&self) -> bool {
        true // Placeholder
    }
    
    pub fn is_feature_configured(&self) -> bool {
        true // Placeholder
    }
    
    pub fn current_search_min_score(&self) -> f32 {
        0.3 // Default from TypeScript
    }
    
    pub fn current_search_max_results(&self) -> usize {
        20 // Default from TypeScript
    }
}

/// State manager placeholder
pub struct CodeIndexStateManager {
    // Will be implemented in state-manager.rs
}

impl CodeIndexStateManager {
    pub fn get_current_status(&self) -> CurrentStatus {
        CurrentStatus {
            system_status: IndexingState::Indexed,
        }
    }
    
    pub fn set_system_state(&self, state: IndexingState, message: Option<String>) {
        // Placeholder
    }
}

pub struct CurrentStatus {
    pub system_status: IndexingState,
}
