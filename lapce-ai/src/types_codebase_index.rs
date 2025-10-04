/// Codebase Index Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/codebase-index.ts
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Codebase Index Constants - Direct translation from TypeScript
/// Lines 6-15 from codebase-index.ts
pub const CODEBASE_INDEX_DEFAULTS: CodebaseIndexDefaults = CodebaseIndexDefaults {
    min_search_results: 10,
    max_search_results: 200,
    default_search_results: 50,
    search_results_step: 10,
    min_search_score: 0.0,
    max_search_score: 1.0,
    default_search_min_score: 0.4,
    search_score_step: 0.05,
};

pub struct CodebaseIndexDefaults {
    pub min_search_results: u32,
    pub max_search_results: u32,
    pub default_search_results: u32,
    pub search_results_step: u32,
    pub min_search_score: f32,
    pub max_search_score: f32,
    pub default_search_min_score: f32,
    pub search_score_step: f32,
}

/// CodebaseIndexConfig - Direct translation from TypeScript
/// Lines 21-39 from codebase-index.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebaseIndexConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_qdrant_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_embedder_provider: Option<CodebaseIndexEmbedderProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_embedder_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_embedder_model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_embedder_model_dimension: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_search_min_score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_search_max_results: Option<u32>,
    // OpenAI Compatible specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_open_ai_compatible_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_open_ai_compatible_model_dimension: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodebaseIndexEmbedderProvider {
    Openai,
    Ollama,
    OpenaiCompatible,
    Gemini,
    Mistral,
}

/// CodebaseIndexModels - Direct translation from TypeScript
/// Lines 45-53 from codebase-index.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CodebaseIndexModels {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai: Option<HashMap<String, ModelDimension>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ollama: Option<HashMap<String, ModelDimension>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "openai-compatible")]
    pub openai_compatible: Option<HashMap<String, ModelDimension>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini: Option<HashMap<String, ModelDimension>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mistral: Option<HashMap<String, ModelDimension>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDimension {
    pub dimension: u32,
}

/// CodebaseIndexProvider - Direct translation from TypeScript
/// Lines 59-69 from codebase-index.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebaseIndexProvider {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_index_open_ai_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_index_qdrant_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_open_ai_compatible_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_open_ai_compatible_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_open_ai_compatible_model_dimension: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_gemini_api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase_index_mistral_api_key: Option<String>,
}

impl CodebaseIndexConfig {
    pub fn validate_search_score(&self) -> bool {
        if let Some(score) = self.codebase_index_search_min_score {
            score >= 0.0 && score <= 1.0
        } else {
            true
        }
    }
    
    pub fn validate_search_results(&self) -> bool {
        if let Some(results) = self.codebase_index_search_max_results {
            results >= CODEBASE_INDEX_DEFAULTS.min_search_results &&
            results <= CODEBASE_INDEX_DEFAULTS.max_search_results
        } else {
            true
        }
    }
}
