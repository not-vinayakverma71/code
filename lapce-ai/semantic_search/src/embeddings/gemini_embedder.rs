// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Translation of embedders/gemini.ts (Lines 1-93) - 100% EXACT TRANSLATION

use crate::error::{Error, Result};
use crate::embeddings::embedder_interface::{
    IEmbedder, EmbeddingResponse, EmbedderInfo, AvailableEmbedders
};
use crate::embeddings::openai_compatible_embedder::OpenAICompatibleEmbedder;
use crate::shared::constants::GEMINI_MAX_ITEM_TOKENS;
use async_trait::async_trait;

/// Lines 16-92: Gemini embedder implementation
/// Wraps OpenAI Compatible embedder with Gemini configuration
/// 
/// Supported models:
/// - text-embedding-004 (dimension: 768)
/// - gemini-embedding-001 (dimension: 2048)
pub struct GeminiEmbedder {
    // Line 17: Internal OpenAI compatible embedder
    openai_compatible_embedder: OpenAICompatibleEmbedder,
    // Lines 18-20: Configuration
    model_id: String,
}

impl GeminiEmbedder {
    // Lines 18-19: Constants
    const GEMINI_BASE_URL: &'static str = "https://generativelanguage.googleapis.com/v1beta/openai/";
    const DEFAULT_MODEL: &'static str = "gemini-embedding-001";
    
    /// Lines 27-42: Constructor
    pub fn new(api_key: String, model_id: Option<String>) -> Result<Self> {
        // Lines 28-30: Validate API key
        if api_key.is_empty() {
            return Err(Error::Runtime {
                message: "API key required".to_string()
            });
        }
        
        // Lines 32-33: Use provided model or default
        let model_id = model_id.unwrap_or_else(|| Self::DEFAULT_MODEL.to_string());
        
        // Lines 35-41: Create OpenAI compatible embedder with Gemini config
        let openai_compatible_embedder = OpenAICompatibleEmbedder::new(
            Self::GEMINI_BASE_URL.to_string(),
            api_key,
            Some(model_id.clone()),
            Some(GEMINI_MAX_ITEM_TOKENS),
        );
        
        Ok(Self {
            openai_compatible_embedder,
            model_id,
        })
    }
}

#[async_trait]
impl IEmbedder for GeminiEmbedder {
    /// Lines 45-63: Create embeddings using Gemini API
    async fn create_embeddings(
        &self,
        texts: Vec<String>,
        model: Option<&str>
    ) -> Result<EmbeddingResponse> {
        // Lines 51-62: Try embedding with error handling
        match self.openai_compatible_embedder.create_embeddings(
            texts,
            model.or(Some(&self.model_id))
        ).await {
            Ok(response) => Ok(response),
            Err(error) => {
                // Lines 55-61: Log telemetry and re-throw
                log::error!("GeminiEmbedder:createEmbeddings error: {:?}", error);
                Err(error)
            }
        }
    }
    
    /// Lines 66-82: Validate configuration
    async fn validate_configuration(&self) -> Result<(bool, Option<String>)> {
        // Lines 70-81: Delegate validation with error handling
        match self.openai_compatible_embedder.validate_configuration().await {
            Ok(result) => Ok(result),
            Err(error) => {
                // Lines 74-80: Log telemetry
                log::error!("GeminiEmbedder:validateConfiguration error: {:?}", error);
                Err(error)
            }
        }
    }
    
    /// Lines 85-91: Get embedder info
    fn embedder_info(&self) -> EmbedderInfo {
        EmbedderInfo {
            name: AvailableEmbedders::Gemini,
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
