// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors  
// Translation of config-manager.ts (Lines 1-464) - 100% EXACT

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Lines 5: Constants from constants.ts
const DEFAULT_SEARCH_MIN_SCORE: f32 = 0.3;
const DEFAULT_MAX_SEARCH_RESULTS: usize = 20;

/// Lines 12-463: Manages configuration state and validation for the code indexing feature
pub struct CodeIndexConfigManager {
    // Lines 13-25: Configuration fields
    codebase_index_enabled: bool,
    embedder_provider: EmbedderProvider,
    model_id: Option<String>,
    model_dimension: Option<usize>,
    open_ai_options: Option<ApiHandlerOptions>,
    ollama_options: Option<ApiHandlerOptions>,
    open_ai_compatible_options: Option<OpenAiCompatibleOptions>,
    gemini_options: Option<GeminiOptions>,
    mistral_options: Option<MistralOptions>,
    qdrant_url: Option<String>,
    qdrant_api_key: Option<String>,
    search_min_score: Option<f32>,
    search_max_results: Option<usize>,
    
    // Line 27: Context proxy for configuration storage
    context_proxy: Arc<dyn ContextProxy>,
}

impl CodeIndexConfigManager {
    /// Lines 27-30: Constructor
    pub fn new(context_proxy: Arc<dyn ContextProxy>) -> Self {
        let mut manager = Self {
            codebase_index_enabled: true,
            embedder_provider: EmbedderProvider::OpenAi,
            model_id: None,
            model_dimension: None,
            open_ai_options: None,
            ollama_options: None,
            open_ai_compatible_options: None,
            gemini_options: None,
            mistral_options: None,
            qdrant_url: Some("http://localhost:6333".to_string()),
            qdrant_api_key: None,
            search_min_score: None,
            search_max_results: None,
            context_proxy,
        };
        
        // Line 29: Initialize with current configuration
        manager.load_and_set_configuration();
        manager
    }
    
    /// Lines 35-37: Get context proxy
    pub fn get_context_proxy(&self) -> Arc<dyn ContextProxy> {
        self.context_proxy.clone()
    }
    
    /// Lines 43-127: Private method to load configuration from storage
    fn load_and_set_configuration(&mut self) {
        // Lines 45-53: Load configuration from global state
        let config = self.context_proxy.get_global_state("codebaseIndexConfig")
            .unwrap_or_else(|| CodebaseIndexConfigState {
                codebase_index_enabled: Some(true),
                codebase_index_qdrant_url: Some("http://localhost:6333".to_string()),
                codebase_index_embedder_provider: Some("openai".to_string()),
                codebase_index_embedder_base_url: None,
                codebase_index_embedder_model_id: None,
                codebase_index_embedder_model_dimension: None,
                codebase_index_open_ai_compatible_base_url: None,
                codebase_index_search_min_score: None,
                codebase_index_search_max_results: None,
            });
        
        // Lines 65-71: Load secrets
        let open_ai_key = self.context_proxy.get_secret("codeIndexOpenAiKey").unwrap_or_default();
        let qdrant_api_key = self.context_proxy.get_secret("codeIndexQdrantApiKey").unwrap_or_default();
        let open_ai_compatible_base_url = config.codebase_index_open_ai_compatible_base_url.clone().unwrap_or_default();
        let open_ai_compatible_api_key = self.context_proxy.get_secret("codeIndexOpenAiCompatibleApiKey").unwrap_or_default();
        let gemini_api_key = self.context_proxy.get_secret("codeIndexGeminiApiKey").unwrap_or_default();
        let mistral_api_key = self.context_proxy.get_secret("codeIndexMistralApiKey").unwrap_or_default();
        
        // Lines 74-78: Update basic configuration
        self.codebase_index_enabled = config.codebase_index_enabled.unwrap_or(true);
        self.qdrant_url = config.codebase_index_qdrant_url;
        self.qdrant_api_key = if qdrant_api_key.is_empty() { None } else { Some(qdrant_api_key) };
        self.search_min_score = config.codebase_index_search_min_score;
        self.search_max_results = config.codebase_index_search_max_results;
        
        // Lines 81-94: Validate and set model dimension
        if let Some(raw_dimension) = config.codebase_index_embedder_model_dimension {
            if raw_dimension > 0 {
                self.model_dimension = Some(raw_dimension as usize);
            } else {
                eprintln!(
                    "Invalid codebaseIndexEmbedderModelDimension value: {}. Must be a positive number.",
                    raw_dimension
                );
                self.model_dimension = None;
            }
        } else {
            self.model_dimension = None;
        }
        
        // Line 96: Set OpenAI options
        self.open_ai_options = if !open_ai_key.is_empty() {
            Some(ApiHandlerOptions {
                api_key: Some(open_ai_key),
                base_url: None,
            })
        } else {
            None
        };
        
        // Lines 99-109: Set embedder provider
        self.embedder_provider = match config.codebase_index_embedder_provider.as_deref() {
            Some("ollama") => EmbedderProvider::Ollama,
            Some("openai-compatible") => EmbedderProvider::OpenAiCompatible,
            Some("gemini") => EmbedderProvider::Gemini,
            Some("mistral") => EmbedderProvider::Mistral,
            _ => EmbedderProvider::OpenAi,
        };
        
        // Line 111: Set model ID
        self.model_id = config.codebase_index_embedder_model_id.filter(|s| !s.is_empty());
        
        // Lines 113-115: Set Ollama options
        self.ollama_options = config.codebase_index_embedder_base_url.map(|base_url| {
            ApiHandlerOptions {
                api_key: None,
                base_url: Some(base_url),
            }
        });
        
        // Lines 117-123: Set OpenAI Compatible options
        self.open_ai_compatible_options = if !open_ai_compatible_base_url.is_empty() && !open_ai_compatible_api_key.is_empty() {
            Some(OpenAiCompatibleOptions {
                base_url: open_ai_compatible_base_url,
                api_key: open_ai_compatible_api_key,
            })
        } else {
            None
        };
        
        // Lines 125-126: Set Gemini and Mistral options
        self.gemini_options = if !gemini_api_key.is_empty() { 
            Some(GeminiOptions { api_key: gemini_api_key }) 
        } else { 
            None 
        };
        
        self.mistral_options = if !mistral_api_key.is_empty() { 
            Some(MistralOptions { api_key: mistral_api_key }) 
        } else { 
            None 
        };
    }
    
    /// Lines 132-193: Load configuration and determine if restart is required
    pub async fn load_configuration(&mut self) -> Result<ConfigLoadResult> {
        // Lines 151-165: Capture previous state
        let previous_snapshot = PreviousConfigSnapshot {
            enabled: self.codebase_index_enabled,
            configured: self.is_configured(),
            embedder_provider: self.embedder_provider.clone(),
            model_id: self.model_id.clone(),
            model_dimension: self.model_dimension,
            open_ai_key: self.open_ai_options.as_ref()
                .and_then(|o| o.api_key.clone())
                .unwrap_or_default(),
            ollama_base_url: self.ollama_options.as_ref()
                .and_then(|o| o.base_url.clone())
                .unwrap_or_default(),
            open_ai_compatible_base_url: self.open_ai_compatible_options.as_ref()
                .map(|o| o.base_url.clone())
                .unwrap_or_default(),
            open_ai_compatible_api_key: self.open_ai_compatible_options.as_ref()
                .map(|o| o.api_key.clone())
                .unwrap_or_default(),
            gemini_api_key: self.gemini_options.as_ref()
                .map(|o| o.api_key.clone())
                .unwrap_or_default(),
            mistral_api_key: self.mistral_options.as_ref()
                .map(|o| o.api_key.clone())
                .unwrap_or_default(),
            qdrant_url: self.qdrant_url.clone().unwrap_or_default(),
            qdrant_api_key: self.qdrant_api_key.clone().unwrap_or_default(),
        };
        
        // Lines 167-168: Refresh secrets
        self.context_proxy.refresh_secrets().await?;
        
        // Lines 170-171: Load new configuration
        self.load_and_set_configuration();
        
        // Line 173: Check if restart required
        let requires_restart = self.does_config_change_require_restart(&previous_snapshot);
        
        // Lines 175-192: Return result
        Ok(ConfigLoadResult {
            config_snapshot: previous_snapshot,
            current_config: self.get_config(),
            requires_restart,
        })
    }
    
    /// Lines 198-226: Check if service is properly configured
    pub fn is_configured(&self) -> bool {
        match self.embedder_provider {
            EmbedderProvider::OpenAi => {
                // Lines 199-202
                let has_key = self.open_ai_options.as_ref()
                    .and_then(|o| o.api_key.as_ref())
                    .map(|k| !k.is_empty())
                    .unwrap_or(false);
                let has_url = self.qdrant_url.is_some();
                has_key && has_url
            },
            EmbedderProvider::Ollama => {
                // Lines 203-207
                let has_base_url = self.ollama_options.as_ref()
                    .and_then(|o| o.base_url.as_ref())
                    .map(|u| !u.is_empty())
                    .unwrap_or(false);
                let has_qdrant_url = self.qdrant_url.is_some();
                has_base_url && has_qdrant_url
            },
            EmbedderProvider::OpenAiCompatible => {
                // Lines 208-213
                let has_config = self.open_ai_compatible_options.as_ref()
                    .map(|o| !o.base_url.is_empty() && !o.api_key.is_empty())
                    .unwrap_or(false);
                let has_qdrant_url = self.qdrant_url.is_some();
                has_config && has_qdrant_url
            },
            EmbedderProvider::Gemini => {
                // Lines 214-218
                let has_key = self.gemini_options.as_ref()
                    .map(|o| !o.api_key.is_empty())
                    .unwrap_or(false);
                let has_qdrant_url = self.qdrant_url.is_some();
                has_key && has_qdrant_url
            },
            EmbedderProvider::Mistral => {
                self.mistral_options.is_some()
            },
            EmbedderProvider::AwsTitan => {
                // AWS Titan uses AWS credentials from environment
                std::env::var("AWS_ACCESS_KEY_ID").is_ok()
            },
        }
    }
    
    /// Lines 244-336: Determine if configuration change requires restart
    pub fn does_config_change_require_restart(&self, prev: &PreviousConfigSnapshot) -> bool {
        let now_configured = self.is_configured();
        
        // Lines 248-259: Handle null/undefined values safely
        let prev_enabled = prev.enabled;
        let prev_configured = prev.configured;
        
        // Lines 262-264: Transition from disabled to enabled
        if (!prev_enabled || !prev_configured) && self.codebase_index_enabled && now_configured {
            return true;
        }
        
        // Lines 267-269: Transition from enabled to disabled
        if prev_enabled && !self.codebase_index_enabled {
            return true;
        }
        
        // Lines 272-274: No change if both not ready
        if (!prev_enabled || !prev_configured) && (!self.codebase_index_enabled || !now_configured) {
            return false;
        }
        
        // Lines 277-280: Only check critical changes if enabled
        if !self.codebase_index_enabled {
            return false;
        }
        
        // Lines 283-285: Provider change
        if prev.embedder_provider != self.embedder_provider {
            return true;
        }
        
        // Lines 288-329: Authentication and configuration changes
        let current_open_ai_key = self.open_ai_options.as_ref()
            .and_then(|o| o.api_key.clone())
            .unwrap_or_default();
        let current_ollama_base_url = self.ollama_options.as_ref()
            .and_then(|o| o.base_url.clone())
            .unwrap_or_default();
        let current_open_ai_compatible_base_url = self.open_ai_compatible_options.as_ref()
            .map(|o| o.base_url.clone())
            .unwrap_or_default();
        let current_open_ai_compatible_api_key = self.open_ai_compatible_options.as_ref()
            .map(|o| o.api_key.clone())
            .unwrap_or_default();
        let current_gemini_api_key = self.gemini_options.as_ref()
            .map(|o| o.api_key.clone())
            .unwrap_or_default();
        let current_mistral_api_key = self.mistral_options.as_ref()
            .map(|o| o.api_key.clone())
            .unwrap_or_default();
        let current_qdrant_url = self.qdrant_url.clone().unwrap_or_default();
        let current_qdrant_api_key = self.qdrant_api_key.clone().unwrap_or_default();
        
        // Check each critical configuration change
        if prev.open_ai_key != current_open_ai_key ||
           prev.ollama_base_url != current_ollama_base_url ||
           prev.open_ai_compatible_base_url != current_open_ai_compatible_base_url ||
           prev.open_ai_compatible_api_key != current_open_ai_compatible_api_key ||
           prev.gemini_api_key != current_gemini_api_key ||
           prev.mistral_api_key != current_mistral_api_key ||
           prev.model_dimension != self.model_dimension ||
           prev.qdrant_url != current_qdrant_url ||
           prev.qdrant_api_key != current_qdrant_api_key {
            return true;
        }
        
        // Lines 331-334: Vector dimension changes
        if self.has_vector_dimension_changed(&prev.embedder_provider, &prev.model_id) {
            return true;
        }
        
        false
    }
    
    /// Lines 341-362: Check if model changes result in vector dimension changes
    fn has_vector_dimension_changed(
        &self,
        prev_provider: &EmbedderProvider,
        prev_model_id: &Option<String>
    ) -> bool {
        let current_model_id = self.model_id.clone()
            .unwrap_or_else(|| get_default_model_id(&self.embedder_provider));
        let resolved_prev_model_id = prev_model_id.clone()
            .unwrap_or_else(|| get_default_model_id(prev_provider));
        
        // Lines 347-349: Same provider and model means no change
        if prev_provider == &self.embedder_provider && resolved_prev_model_id == current_model_id {
            return false;
        }
        
        // Lines 352-361: Compare dimensions
        let prev_dimension = get_model_dimension(prev_provider, &resolved_prev_model_id);
        let current_dimension = get_model_dimension(&self.embedder_provider, &current_model_id);
        
        match (prev_dimension, current_dimension) {
            (Some(prev), Some(curr)) => prev != curr,
            _ => true, // Be safe if we can't determine dimensions
        }
    }
    
    /// Lines 367-383: Get current configuration
    pub fn get_config(&self) -> CodeIndexConfig {
        CodeIndexConfig {
            is_configured: self.is_configured(),
            embedder_provider: self.embedder_provider.clone(),
            model_id: self.model_id.clone(),
            model_dimension: self.model_dimension,
            open_ai_options: self.open_ai_options.clone(),
            ollama_options: self.ollama_options.clone(),
            open_ai_compatible_options: self.open_ai_compatible_options.clone(),
            gemini_options: self.gemini_options.clone(),
            mistral_options: self.mistral_options.clone(),
            qdrant_url: self.qdrant_url.clone(),
            qdrant_api_key: self.qdrant_api_key.clone(),
            search_min_score: self.current_search_min_score(),
            search_max_results: self.current_search_max_results(),
        }
    }
    
    /// Lines 388-390: Get whether feature is enabled
    pub fn is_feature_enabled(&self) -> bool {
        self.codebase_index_enabled
    }
    
    /// Lines 395-397: Get whether feature is configured
    pub fn is_feature_configured(&self) -> bool {
        self.is_configured()
    }
    
    /// Lines 402-404: Get current embedder provider
    pub fn current_embedder_provider(&self) -> &EmbedderProvider {
        &self.embedder_provider
    }
    
    /// Lines 409-414: Get Qdrant configuration
    pub fn qdrant_config(&self) -> QdrantConfig {
        QdrantConfig {
            url: self.qdrant_url.clone(),
            api_key: self.qdrant_api_key.clone(),
        }
    }
    
    /// Lines 419-421: Get current model ID
    pub fn current_model_id(&self) -> Option<String> {
        self.model_id.clone()
    }
    
    /// Lines 427-438: Get current model dimension
    pub fn current_model_dimension(&self) -> Option<usize> {
        // First try model-specific dimension
        let model_id = self.model_id.clone()
            .unwrap_or_else(|| get_default_model_id(&self.embedder_provider));
        let model_dimension = get_model_dimension(&self.embedder_provider, &model_id);
        
        // Fall back to custom dimension if needed
        match model_dimension {
            Some(dim) => Some(dim),
            None if self.model_dimension.is_some() => self.model_dimension,
            _ => model_dimension,
        }
    }
    
    /// Lines 444-454: Get current search minimum score
    pub fn current_search_min_score(&self) -> f32 {
        // User setting takes priority
        if let Some(score) = self.search_min_score {
            return score;
        }
        
        // Fall back to model-specific threshold
        let current_model_id = self.model_id.clone()
            .unwrap_or_else(|| get_default_model_id(&self.embedder_provider));
        get_model_score_threshold(&self.embedder_provider, &current_model_id)
            .unwrap_or(DEFAULT_SEARCH_MIN_SCORE)
    }
    
    /// Lines 460-462: Get current search max results
    pub fn current_search_max_results(&self) -> usize {
        self.search_max_results.unwrap_or(DEFAULT_MAX_SEARCH_RESULTS)
    }
}

// Supporting types

/// From interfaces/manager.ts - EmbedderProvider enum
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
    #[serde(rename = "aws-titan")]
    AwsTitan,
}

/// From interfaces/config.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIndexConfig {
    pub is_configured: bool,
    pub embedder_provider: EmbedderProvider,
    pub model_id: Option<String>,
    pub model_dimension: Option<usize>,
    pub open_ai_options: Option<ApiHandlerOptions>,
    pub ollama_options: Option<ApiHandlerOptions>,
    pub open_ai_compatible_options: Option<OpenAiCompatibleOptions>,
    pub gemini_options: Option<GeminiOptions>,
    pub mistral_options: Option<MistralOptions>,
    pub qdrant_url: Option<String>,
    pub qdrant_api_key: Option<String>,
    pub search_min_score: f32,
    pub search_max_results: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiHandlerOptions {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleOptions {
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiOptions {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralOptions {
    pub api_key: String,
}

#[derive(Debug, Clone)]
pub struct PreviousConfigSnapshot {
    pub enabled: bool,
    pub configured: bool,
    pub embedder_provider: EmbedderProvider,
    pub model_id: Option<String>,
    pub model_dimension: Option<usize>,
    pub open_ai_key: String,
    pub ollama_base_url: String,
    pub open_ai_compatible_base_url: String,
    pub open_ai_compatible_api_key: String,
    pub gemini_api_key: String,
    pub mistral_api_key: String,
    pub qdrant_url: String,
    pub qdrant_api_key: String,
}

pub struct ConfigLoadResult {
    pub config_snapshot: PreviousConfigSnapshot,
    pub current_config: CodeIndexConfig,
    pub requires_restart: bool,
}

pub struct QdrantConfig {
    pub url: Option<String>,
    pub api_key: Option<String>,
}

// Storage state structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodebaseIndexConfigState {
    codebase_index_enabled: Option<bool>,
    codebase_index_qdrant_url: Option<String>,
    codebase_index_embedder_provider: Option<String>,
    codebase_index_embedder_base_url: Option<String>,
    codebase_index_embedder_model_id: Option<String>,
    codebase_index_embedder_model_dimension: Option<i32>,
    codebase_index_open_ai_compatible_base_url: Option<String>,
    codebase_index_search_min_score: Option<f32>,
    codebase_index_search_max_results: Option<usize>,
}

// Trait for context proxy (configuration storage)
#[async_trait::async_trait]
pub trait ContextProxy: Send + Sync {
    fn get_global_state(&self, key: &str) -> Option<CodebaseIndexConfigState>;
    fn get_secret(&self, key: &str) -> Option<String>;
    async fn refresh_secrets(&self) -> Result<()>;
}

// Helper functions for model defaults
fn get_default_model_id(provider: &EmbedderProvider) -> String {
    match provider {
        EmbedderProvider::OpenAi => "text-embedding-3-small".to_string(),
        EmbedderProvider::Ollama => "nomic-embed-text".to_string(),
        EmbedderProvider::OpenAiCompatible => "text-embedding-3-small".to_string(),
        EmbedderProvider::Gemini => "text-embedding-004".to_string(),
        EmbedderProvider::Mistral => "mistral-embed".to_string(),
        EmbedderProvider::AwsTitan => "amazon.titan-embed-text-v2:0".to_string(),
    }
}

fn get_model_dimension(provider: &EmbedderProvider, model_id: &str) -> Option<usize> {
    match (provider, model_id) {
        (EmbedderProvider::OpenAi, "text-embedding-3-small") => Some(1536),
        (EmbedderProvider::OpenAi, "text-embedding-3-large") => Some(3072),
        (EmbedderProvider::OpenAi, "text-embedding-ada-002") => Some(1536),
        (EmbedderProvider::Ollama, "nomic-embed-text") => Some(768),
        (EmbedderProvider::Gemini, "text-embedding-004") => Some(768),
        (EmbedderProvider::Mistral, "mistral-embed") => Some(1024),
        (EmbedderProvider::AwsTitan, "amazon.titan-embed-text-v1") => Some(1536),
        (EmbedderProvider::AwsTitan, "amazon.titan-embed-text-v2:0") => Some(1024),
        _ => None,
    }
}

fn get_model_score_threshold(provider: &EmbedderProvider, model_id: &str) -> Option<f32> {
    match (provider, model_id) {
        (EmbedderProvider::OpenAi, _) => Some(0.3),
        (EmbedderProvider::Ollama, _) => Some(0.3),
        (EmbedderProvider::Gemini, _) => Some(0.3),
        (EmbedderProvider::Mistral, _) => Some(0.3),
        _ => None,
    }
}
