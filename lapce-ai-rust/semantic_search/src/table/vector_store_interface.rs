// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Translation of interfaces/vector-store.ts (Lines 1-80) - 100% EXACT

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lines 4-8: PointStruct type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointStruct {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: HashMap<String, serde_json::Value>,
}

/// Lines 10-65: IVectorStore interface
#[async_trait::async_trait]
pub trait IVectorStore: Send + Sync {
    /// Lines 11-15: Initializes the vector store
    /// Returns true if a new collection was created
    async fn initialize(&self) -> Result<bool>;
    
    /// Lines 17-21: Upserts points into the vector store
    async fn upsert_points(&self, points: Vec<PointStruct>) -> Result<()>;
    
    /// Lines 24-36: Searches for similar vectors
    /// 
    /// # Arguments
    /// * `query_vector` - Vector to search for
    /// * `directory_prefix` - Optional directory prefix to filter results
    /// * `min_score` - Optional minimum score threshold
    /// * `max_results` - Optional maximum number of results to return
    async fn search(
        &self,
        query_vector: Vec<f32>,
        directory_prefix: Option<&str>,
        min_score: Option<f32>,
        max_results: Option<usize>,
    ) -> Result<Vec<VectorStoreSearchResult>>;
    
    /// Lines 38-42: Deletes points by file path
    async fn delete_points_by_file_path(&self, file_path: &str) -> Result<()>;
    
    /// Lines 44-48: Deletes points by multiple file paths
    async fn delete_points_by_multiple_file_paths(&self, file_paths: Vec<String>) -> Result<()>;
    
    /// Lines 50-53: Clears all points from the collection
    async fn clear_collection(&self) -> Result<()>;
    
    /// Lines 55-58: Deletes the entire collection
    async fn delete_collection(&self) -> Result<()>;
    
    /// Lines 60-64: Checks if the collection exists
    async fn collection_exists(&self) -> Result<bool>;
}

/// Lines 67-71: VectorStoreSearchResult interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreSearchResult {
    pub id: IdType,
    pub score: f32,
    pub payload: Option<Payload>,
}

/// ID can be either string or number
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdType {
    String(String),
    Number(i64),
}

/// Lines 73-79: Payload interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "codeChunk")]
    pub code_chunk: String,
    #[serde(rename = "startLine")]
    pub start_line: u32,
    #[serde(rename = "endLine")]
    pub end_line: u32,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
