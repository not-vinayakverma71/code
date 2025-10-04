// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Translation of interfaces/embedder.ts (Lines 1-36) - 100% EXACT

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Lines 5-21: IEmbedder interface for code index embedders
/// This interface is implemented by both OpenAI and Ollama embedders
#[async_trait::async_trait]
pub trait IEmbedder: Send + Sync {
    /// Lines 6-12: Creates embeddings for the given texts
    /// 
    /// # Arguments
    /// * `texts` - Array of text strings to create embeddings for
    /// * `model` - Optional model ID to use for embeddings
    /// 
    /// # Returns
    /// Promise resolving to an EmbeddingResponse
    async fn create_embeddings(
        &self,
        texts: Vec<String>,
        model: Option<&str>
    ) -> Result<EmbeddingResponse>;
    
    /// Lines 14-18: Validates the embedder configuration
    /// Tests connectivity and credentials
    /// 
    /// # Returns
    /// Validation result with success status and optional error message
    async fn validate_configuration(&self) -> Result<(bool, Option<String>)>;
    
    /// Line 20: Get embedder information
    fn embedder_info(&self) -> EmbedderInfo;
    
    /// Helper for downcasting to concrete types
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Lines 23-29: EmbeddingResponse interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub usage: Option<EmbeddingUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    #[serde(rename = "promptTokens")]
    pub prompt_tokens: usize,
    #[serde(rename = "totalTokens")]
    pub total_tokens: usize,
}

/// Line 31: AvailableEmbedders type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AvailableEmbedders {
    #[serde(rename = "openai")]
    OpenAi,
    Ollama,
    #[serde(rename = "openai-compatible")]
    OpenAiCompatible,
    Gemini,
    Mistral,
    #[serde(rename = "aws-bedrock")]
    AwsBedrock,
}

/// Lines 33-35: EmbedderInfo interface
#[derive(Debug, Clone)]
pub struct EmbedderInfo {
    pub name: AvailableEmbedders,
}

/// Validation result structure
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub error: Option<String>,
}
