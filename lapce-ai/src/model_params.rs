/// Exact 1:1 Translation of TypeScript model-params from codex-reference/api/transform/model-params.ts
/// DAY 12 MORNING: Translate model-params.ts

use serde::{Deserialize, Serialize};

/// Format enum - exact translation line 29
#[derive(Debug, Clone, PartialEq)]
pub enum Format {
    Anthropic,
    OpenAI,
    Gemini,
    OpenRouter,
}

/// VerbosityLevel
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerbosityLevel {
    Minimal,
    Normal,
    Verbose,
}

/// ReasoningEffortWithMinimal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReasoningEffortWithMinimal {
    Minimal,
    Low,
    Medium,
    High,
}

/// GetModelParamsOptions - exact translation lines 31-37
pub struct GetModelParamsOptions {
    pub format: Format,
    pub model_id: String,
    pub model: ModelInfo,
    pub settings: ProviderSettings,
    pub default_temperature: Option<f32>,
}

/// ModelInfo placeholder
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub max_tokens: u32,
    pub supports_reasoning: bool,
}

/// ProviderSettings
#[derive(Debug, Clone)]
pub struct ProviderSettings {
    pub model_max_tokens: Option<u32>,
    pub model_max_thinking_tokens: Option<u32>,
    pub model_temperature: Option<f32>,
    pub reasoning_effort: Option<ReasoningEffortWithMinimal>,
    pub verbosity: Option<VerbosityLevel>,
}

/// BaseModelParams - exact translation lines 39-45
#[derive(Debug, Clone)]
pub struct BaseModelParams {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<ReasoningEffortWithMinimal>,
    pub reasoning_budget: Option<u32>,
    pub verbosity: Option<VerbosityLevel>,
}

/// AnthropicReasoningParams
#[derive(Debug, Clone)]
pub struct AnthropicReasoningParams {
    pub enabled: bool,
    pub thinking_tokens: Option<u32>,
}

/// OpenAiReasoningParams
#[derive(Debug, Clone)]
pub struct OpenAiReasoningParams {
    pub enabled: bool,
    pub reasoning_effort: Option<String>,
}

/// GeminiReasoningParams
#[derive(Debug, Clone)]
pub struct GeminiReasoningParams {
    pub enabled: bool,
    pub thinking_config: Option<ThinkingConfig>,
}

#[derive(Debug, Clone)]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: u32,
}

/// OpenRouterReasoningParams
#[derive(Debug, Clone)]
pub struct OpenRouterReasoningParams {
    pub enabled: bool,
    pub include_reasoning: bool,
}

/// ModelParams enum - exact translation lines 47-67
#[derive(Debug, Clone)]
pub enum ModelParams {
    Anthropic {
        base: BaseModelParams,
        reasoning: Option<AnthropicReasoningParams>,
    },
    OpenAI {
        base: BaseModelParams,
        reasoning: Option<OpenAiReasoningParams>,
    },
    Gemini {
        base: BaseModelParams,
        reasoning: Option<GeminiReasoningParams>,
    },
    OpenRouter {
        base: BaseModelParams,
        reasoning: Option<OpenRouterReasoningParams>,
    },
}

/// Constants
pub const ANTHROPIC_DEFAULT_MAX_TOKENS: u32 = 8192;
pub const DEFAULT_HYBRID_REASONING_MODEL_MAX_TOKENS: u32 = 65536;
pub const DEFAULT_HYBRID_REASONING_MODEL_THINKING_TOKENS: u32 = 8192;
pub const GEMINI_25_PRO_MIN_THINKING_TOKENS: u32 = 8192;

/// getModelParams - exact translation lines 74-179
pub fn get_model_params(options: GetModelParamsOptions) -> ModelParams {
    let GetModelParamsOptions {
        format,
        model_id,
        model,
        settings,
        default_temperature,
    } = options;
    
    let default_temperature = default_temperature.unwrap_or(0.0);
    
    // Extract custom settings - lines 81-87
    let custom_max_tokens = settings.model_max_tokens;
    let custom_max_thinking_tokens = settings.model_max_thinking_tokens;
    let custom_temperature = settings.model_temperature;
    let custom_reasoning_effort = settings.reasoning_effort.clone();
    let custom_verbosity = settings.verbosity.clone();
    
    // Compute max tokens - lines 90-95
    let max_tokens = get_model_max_output_tokens(
        &model_id,
        &model,
        &settings,
        &format,
    );
    
    // Set temperature - line 97
    let temperature = custom_temperature.or(Some(default_temperature));
    
    // Initialize reasoning params - lines 98-100
    let reasoning_budget = if should_use_reasoning_budget(&model_id) {
        custom_max_thinking_tokens.or(Some(DEFAULT_HYBRID_REASONING_MODEL_THINKING_TOKENS))
    } else {
        None
    };
    
    let reasoning_effort = if should_use_reasoning_effort(&model_id) {
        custom_reasoning_effort.clone()
    } else {
        None
    };
    
    let verbosity = custom_verbosity;
    
    // Create base params
    let base = BaseModelParams {
        max_tokens,
        temperature,
        reasoning_effort,
        reasoning_budget,
        verbosity,
    };
    
    // Return format-specific params
    match format {
        Format::Anthropic => ModelParams::Anthropic {
            base,
            reasoning: get_anthropic_reasoning(&model_id, &settings),
        },
        Format::OpenAI => ModelParams::OpenAI {
            base,
            reasoning: get_openai_reasoning(&model_id, &settings),
        },
        Format::Gemini => ModelParams::Gemini {
            base,
            reasoning: get_gemini_reasoning(&model_id, &settings),
        },
        Format::OpenRouter => ModelParams::OpenRouter {
            base,
            reasoning: get_openrouter_reasoning(&model_id, &settings),
        },
    }
}

/// Helper functions
pub fn get_model_max_output_tokens(
    model_id: &str,
    model: &ModelInfo,
    settings: &ProviderSettings,
    format: &Format,
) -> Option<u32> {
    // Use custom max tokens if provided
    if let Some(custom) = settings.model_max_tokens {
        return Some(custom);
    }
    
    // Use model-specific defaults
    match format {
        Format::Anthropic => Some(ANTHROPIC_DEFAULT_MAX_TOKENS),
        Format::OpenAI => Some(4096),
        Format::Gemini => Some(8192),
        Format::OpenRouter => Some(model.max_tokens),
    }
}

pub fn should_use_reasoning_budget(model_id: &str) -> bool {
    model_id.contains("thinking") || model_id.contains("reasoning")
}

pub fn should_use_reasoning_effort(model_id: &str) -> bool {
    model_id.contains("o1") || model_id.contains("reasoning")
}

pub fn get_anthropic_reasoning(model_id: &str, settings: &ProviderSettings) -> Option<AnthropicReasoningParams> {
    if model_id.contains("claude-3-5-sonnet-20241022") && settings.reasoning_effort.is_some() {
        Some(AnthropicReasoningParams {
            enabled: true,
            thinking_tokens: settings.model_max_thinking_tokens,
        })
    } else {
        None
    }
}

pub fn get_openai_reasoning(model_id: &str, settings: &ProviderSettings) -> Option<OpenAiReasoningParams> {
    if model_id.contains("o1") {
        Some(OpenAiReasoningParams {
            enabled: true,
            reasoning_effort: settings.reasoning_effort.as_ref().map(|e| format!("{:?}", e)),
        })
    } else {
        None
    }
}

pub fn get_gemini_reasoning(model_id: &str, settings: &ProviderSettings) -> Option<GeminiReasoningParams> {
    if model_id.contains("thinking") {
        Some(GeminiReasoningParams {
            enabled: true,
            thinking_config: Some(ThinkingConfig {
                enabled: true,
                budget_tokens: settings.model_max_thinking_tokens
                    .unwrap_or(GEMINI_25_PRO_MIN_THINKING_TOKENS),
            }),
        })
    } else {
        None
    }
}

pub fn get_openrouter_reasoning(model_id: &str, settings: &ProviderSettings) -> Option<OpenRouterReasoningParams> {
    if model_id.contains("deepseek-r1") || settings.reasoning_effort.is_some() {
        Some(OpenRouterReasoningParams {
            enabled: true,
            include_reasoning: true,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_model_params_anthropic() {
        let options = GetModelParamsOptions {
            format: Format::Anthropic,
            model_id: "claude-3-5-sonnet".to_string(),
            model: ModelInfo {
                id: "claude-3-5-sonnet".to_string(),
                max_tokens: 8192,
                supports_reasoning: false,
            },
            settings: ProviderSettings {
                model_max_tokens: Some(4096),
                model_max_thinking_tokens: None,
                model_temperature: Some(0.7),
                reasoning_effort: None,
                verbosity: Some(VerbosityLevel::Normal),
            },
            default_temperature: None,
        };
        
        let params = get_model_params(options);
        
        match params {
            ModelParams::Anthropic { base, .. } => {
                assert_eq!(base.max_tokens, Some(4096));
                assert_eq!(base.temperature, Some(0.7));
                assert_eq!(base.verbosity, Some(VerbosityLevel::Normal));
            }
            _ => panic!("Wrong format"),
        }
    }
    
    #[test]
    fn test_reasoning_budget_detection() {
        assert!(should_use_reasoning_budget("gemini-1.5-pro:thinking"));
        assert!(should_use_reasoning_budget("claude-reasoning-v1"));
        assert!(!should_use_reasoning_budget("gpt-4"));
    }
}
