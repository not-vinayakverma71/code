// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Translation of interfaces/config.ts (Lines 1-41) - 100% EXACT

use crate::database::manager_interface::EmbedderProvider;
use serde::{Deserialize, Serialize};

/// Lines 7-21: Configuration state for the code indexing feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIndexConfig {
    #[serde(rename = "isConfigured")]
    pub is_configured: bool,
    
    #[serde(rename = "embedderProvider")]
    pub embedder_provider: EmbedderProvider,
    
    #[serde(rename = "modelId")]
    pub model_id: Option<String>,
    
    #[serde(rename = "modelDimension")]
    pub model_dimension: Option<usize>,
    
    #[serde(rename = "openAiOptions")]
    pub open_ai_options: Option<ApiHandlerOptions>,
    
    #[serde(rename = "ollamaOptions")]
    pub ollama_options: Option<ApiHandlerOptions>,
    
    #[serde(rename = "openAiCompatibleOptions")]
    pub open_ai_compatible_options: Option<OpenAiCompatibleOptions>,
    
    #[serde(rename = "geminiOptions")]
    pub gemini_options: Option<GeminiOptions>,
    
    #[serde(rename = "mistralOptions")]
    pub mistral_options: Option<MistralOptions>,
    
    #[serde(rename = "qdrantUrl")]
    pub qdrant_url: Option<String>,
    
    #[serde(rename = "qdrantApiKey")]
    pub qdrant_api_key: Option<String>,
    
    #[serde(rename = "searchMinScore")]
    pub search_min_score: Option<f32>,
    
    #[serde(rename = "searchMaxResults")]
    pub search_max_results: Option<usize>,
}

/// Lines 26-40: Snapshot of previous configuration
/// Used to determine if a restart is required
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousConfigSnapshot {
    pub enabled: bool,
    pub configured: bool,
    
    #[serde(rename = "embedderProvider")]
    pub embedder_provider: EmbedderProvider,
    
    #[serde(rename = "modelId")]
    pub model_id: Option<String>,
    
    #[serde(rename = "modelDimension")]
    pub model_dimension: Option<usize>,
    
    #[serde(rename = "openAiKey")]
    pub open_ai_key: Option<String>,
    
    #[serde(rename = "ollamaBaseUrl")]
    pub ollama_base_url: Option<String>,
    
    #[serde(rename = "openAiCompatibleBaseUrl")]
    pub open_ai_compatible_base_url: Option<String>,
    
    #[serde(rename = "openAiCompatibleApiKey")]
    pub open_ai_compatible_api_key: Option<String>,
    
    #[serde(rename = "geminiApiKey")]
    pub gemini_api_key: Option<String>,
    
    #[serde(rename = "mistralApiKey")]
    pub mistral_api_key: Option<String>,
    
    #[serde(rename = "qdrantUrl")]
    pub qdrant_url: Option<String>,
    
    #[serde(rename = "qdrantApiKey")]
    pub qdrant_api_key: Option<String>,
}

/// From shared/api - ApiHandlerOptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiHandlerOptions {
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
}

/// OpenAI Compatible options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleOptions {
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    
    #[serde(rename = "apiKey")]
    pub api_key: String,
}

/// Gemini options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiOptions {
    #[serde(rename = "apiKey")]
    pub api_key: String,
}

/// Mistral options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralOptions {
    #[serde(rename = "apiKey")]
    pub api_key: String,
}
