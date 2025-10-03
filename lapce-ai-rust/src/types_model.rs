/// Model Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/model.ts
use serde::{Deserialize, Serialize};

/// ReasoningEffort - Direct translation from TypeScript
/// Lines 7-11 from model.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

/// ReasoningEffortWithMinimal - Direct translation from TypeScript
/// Lines 17-19 from model.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffortWithMinimal {
    Minimal,
    Low,
    Medium,
    High,
}

/// VerbosityLevel - Direct translation from TypeScript
/// Lines 25-29 from model.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerbosityLevel {
    Low,
    Medium,
    High,
}

/// ModelParameter - Direct translation from TypeScript
/// Lines 35-39 from model.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelParameter {
    MaxTokens,
    Temperature,
    Reasoning,
    IncludeReasoning,
}

pub fn is_model_parameter(value: &str) -> bool {
    matches!(value, "max_tokens" | "temperature" | "reasoning" | "include_reasoning")
}

/// ModelInfo - Direct translation from TypeScript
/// Lines 48-84 from model.ts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_thinking_tokens: Option<u32>,
    pub context_window: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_computer_use: Option<bool>,
    pub supports_prompt_cache: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_verbosity: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_reasoning_budget: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_reasoning_budget: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_reasoning_effort: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_parameters: Option<Vec<ModelParameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_writes_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_reads_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_tokens_per_cache_point: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cache_points: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cachable_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_index: Option<u32>, // kilocode_change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiers: Option<Vec<ModelTier>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTier {
    pub context_window: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_writes_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_reads_price: Option<f64>,
}
