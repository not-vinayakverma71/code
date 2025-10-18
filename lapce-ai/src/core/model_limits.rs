// Ported from: Codex/packages/types/src/providers/*.ts
// Source of truth for model context windows and token limits
// Must match Codex definitions exactly for parity

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Model information including context window and max output tokens
#[derive(Debug, Clone)]
pub struct ModelLimits {
    pub context_window: usize,
    pub max_tokens: usize,
}

/// Global model limits map - initialized once and cached
/// Ported from Codex provider definitions (anthropic.ts, openai.ts, etc.)
pub static MODEL_LIMITS: Lazy<HashMap<&'static str, ModelLimits>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // === Anthropic Models ===
    // From: Codex/packages/types/src/providers/anthropic.ts
    
    // Claude Sonnet 4.5
    map.insert("claude-sonnet-4-5", ModelLimits {
        context_window: 200_000,
        max_tokens: 64_000,
    });
    
    // Claude Sonnet 4 (2025-05-14)
    map.insert("claude-sonnet-4-20250514", ModelLimits {
        context_window: 200_000,
        max_tokens: 64_000,
    });
    
    // Claude Opus 4.1 (2025-08-05)
    map.insert("claude-opus-4-1-20250805", ModelLimits {
        context_window: 200_000,
        max_tokens: 32_000,
    });
    
    // Claude Opus 4 (2025-05-14)
    map.insert("claude-opus-4-20250514", ModelLimits {
        context_window: 200_000,
        max_tokens: 32_000,
    });
    
    // Claude 3.7 Sonnet (with thinking)
    map.insert("claude-3-7-sonnet-20250219:thinking", ModelLimits {
        context_window: 200_000,
        max_tokens: 128_000,
    });
    
    // Claude 3.7 Sonnet
    map.insert("claude-3-7-sonnet-20250219", ModelLimits {
        context_window: 200_000,
        max_tokens: 8192,
    });
    
    // Claude 3.5 Sonnet (2024-10-22)
    map.insert("claude-3-5-sonnet-20241022", ModelLimits {
        context_window: 200_000,
        max_tokens: 8192,
    });
    
    // Claude 3.5 Haiku (2024-10-22)
    map.insert("claude-3-5-haiku-20241022", ModelLimits {
        context_window: 200_000,
        max_tokens: 8192,
    });
    
    // Claude 3 Opus
    map.insert("claude-3-opus-20240229", ModelLimits {
        context_window: 200_000,
        max_tokens: 4096,
    });
    
    // Claude 3 Haiku
    map.insert("claude-3-haiku-20240307", ModelLimits {
        context_window: 200_000,
        max_tokens: 4096,
    });
    
    // Claude Haiku 4.5 (2025-10-01)
    map.insert("claude-haiku-4-5-20251001", ModelLimits {
        context_window: 200_000,
        max_tokens: 64_000,
    });
    
    // === OpenAI Models ===
    // From: Codex/packages/types/src/providers/openai.ts
    
    // GPT-5 series (400K context)
    map.insert("gpt-5-chat-latest", ModelLimits {
        context_window: 400_000,
        max_tokens: 128_000,
    });
    
    map.insert("gpt-5-2025-08-07", ModelLimits {
        context_window: 400_000,
        max_tokens: 128_000,
    });
    
    map.insert("gpt-5-mini-2025-08-07", ModelLimits {
        context_window: 400_000,
        max_tokens: 128_000,
    });
    
    map.insert("gpt-5-nano-2025-08-07", ModelLimits {
        context_window: 400_000,
        max_tokens: 128_000,
    });
    
    map.insert("gpt-5-codex", ModelLimits {
        context_window: 400_000,
        max_tokens: 128_000,
    });
    
    // GPT-4.1 series (1M+ context)
    map.insert("gpt-4.1", ModelLimits {
        context_window: 1_047_576,
        max_tokens: 32_768,
    });
    
    map.insert("gpt-4.1-mini", ModelLimits {
        context_window: 1_047_576,
        max_tokens: 32_768,
    });
    
    map.insert("gpt-4.1-nano", ModelLimits {
        context_window: 1_047_576,
        max_tokens: 32_768,
    });
    
    // O-series models (200K context)
    map.insert("o3", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    map.insert("o3-high", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    map.insert("o3-low", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    map.insert("o4-mini", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    map.insert("o4-mini-high", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    map.insert("o4-mini-low", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    map.insert("o3-mini", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    map.insert("o3-mini-high", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    map.insert("o3-mini-low", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    map.insert("o1", ModelLimits {
        context_window: 200_000,
        max_tokens: 100_000,
    });
    
    // O1 series (128K context)
    map.insert("o1-preview", ModelLimits {
        context_window: 128_000,
        max_tokens: 32_768,
    });
    
    map.insert("o1-mini", ModelLimits {
        context_window: 128_000,
        max_tokens: 65_536,
    });
    
    // GPT-4o series (128K context)
    map.insert("gpt-4o", ModelLimits {
        context_window: 128_000,
        max_tokens: 16_384,
    });
    
    map.insert("gpt-4o-mini", ModelLimits {
        context_window: 128_000,
        max_tokens: 16_384,
    });
    
    // Codex Mini Latest
    map.insert("codex-mini-latest", ModelLimits {
        context_window: 200_000,
        max_tokens: 16_384,
    });
    
    // === Fallback Default (matches openAiModelInfoSaneDefaults) ===
    map.insert("__default__", ModelLimits {
        context_window: 128_000,
        max_tokens: 16_384,
    });

    map
});

/// Get model limits for a given model ID
/// Returns fallback default if model not found
pub fn get_model_limits(model_id: &str) -> &'static ModelLimits {
    MODEL_LIMITS.get(model_id)
        .unwrap_or_else(|| MODEL_LIMITS.get("__default__").unwrap())
}

/// Get reserved tokens for a model (used by truncateConversationIfNeeded)
/// Matches Codex behavior: maxTokens ?? ANTHROPIC_DEFAULT_MAX_TOKENS
pub fn get_reserved_tokens(model_id: &str, custom_max_tokens: Option<usize>) -> usize {
    if let Some(max_tokens) = custom_max_tokens {
        return max_tokens;
    }
    
    let limits = get_model_limits(model_id);
    limits.max_tokens
}

// === Constants from Codex ===
pub const ANTHROPIC_DEFAULT_MAX_TOKENS: usize = 8192;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_models_exact_values() {
        // Claude Sonnet 4.5
        let limits = get_model_limits("claude-sonnet-4-5");
        assert_eq!(limits.context_window, 200_000);
        assert_eq!(limits.max_tokens, 64_000);
        
        // Claude 3.5 Sonnet
        let limits = get_model_limits("claude-3-5-sonnet-20241022");
        assert_eq!(limits.context_window, 200_000);
        assert_eq!(limits.max_tokens, 8192);
        
        // Claude 3 Opus
        let limits = get_model_limits("claude-3-opus-20240229");
        assert_eq!(limits.context_window, 200_000);
        assert_eq!(limits.max_tokens, 4096);
    }

    #[test]
    fn test_openai_models_exact_values() {
        // GPT-5
        let limits = get_model_limits("gpt-5-2025-08-07");
        assert_eq!(limits.context_window, 400_000);
        assert_eq!(limits.max_tokens, 128_000);
        
        // GPT-4.1
        let limits = get_model_limits("gpt-4.1");
        assert_eq!(limits.context_window, 1_047_576);
        assert_eq!(limits.max_tokens, 32_768);
        
        // O3
        let limits = get_model_limits("o3");
        assert_eq!(limits.context_window, 200_000);
        assert_eq!(limits.max_tokens, 100_000);
        
        // GPT-4o
        let limits = get_model_limits("gpt-4o");
        assert_eq!(limits.context_window, 128_000);
        assert_eq!(limits.max_tokens, 16_384);
    }

    #[test]
    fn test_fallback_default() {
        let limits = get_model_limits("unknown-model");
        assert_eq!(limits.context_window, 128_000);
        assert_eq!(limits.max_tokens, 16_384);
    }

    #[test]
    fn test_reserved_tokens() {
        // Custom max tokens takes precedence
        let reserved = get_reserved_tokens("claude-sonnet-4-5", Some(32_000));
        assert_eq!(reserved, 32_000);
        
        // Falls back to model max tokens
        let reserved = get_reserved_tokens("claude-3-5-sonnet-20241022", None);
        assert_eq!(reserved, 8192);
        
        // Unknown model uses default
        let reserved = get_reserved_tokens("unknown", None);
        assert_eq!(reserved, 16_384);
    }

    #[test]
    fn test_all_anthropic_models_present() {
        let expected_models = vec![
            "claude-sonnet-4-5",
            "claude-sonnet-4-20250514",
            "claude-opus-4-1-20250805",
            "claude-opus-4-20250514",
            "claude-3-7-sonnet-20250219:thinking",
            "claude-3-7-sonnet-20250219",
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
            "claude-3-opus-20240229",
            "claude-3-haiku-20240307",
            "claude-haiku-4-5-20251001",
        ];
        
        for model in expected_models {
            assert!(MODEL_LIMITS.contains_key(model), "Missing model: {}", model);
        }
    }

    #[test]
    fn test_all_openai_models_present() {
        let expected_models = vec![
            "gpt-5-chat-latest",
            "gpt-5-2025-08-07",
            "gpt-5-mini-2025-08-07",
            "gpt-5-nano-2025-08-07",
            "gpt-5-codex",
            "gpt-4.1",
            "gpt-4.1-mini",
            "gpt-4.1-nano",
            "o3", "o3-high", "o3-low",
            "o4-mini", "o4-mini-high", "o4-mini-low",
            "o3-mini", "o3-mini-high", "o3-mini-low",
            "o1", "o1-preview", "o1-mini",
            "gpt-4o", "gpt-4o-mini",
            "codex-mini-latest",
        ];
        
        for model in expected_models {
            assert!(MODEL_LIMITS.contains_key(model), "Missing model: {}", model);
        }
    }

    #[test]
    fn test_anthropic_default_constant() {
        assert_eq!(ANTHROPIC_DEFAULT_MAX_TOKENS, 8192);
    }
}
