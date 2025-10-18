// Token counting using tiktoken_rs
// Ported from: Codex provider token counting implementations
// Caches encoders per model for performance

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use tiktoken_rs::{cl100k_base, o200k_base, CoreBPE};

/// Cached token encoders per model
static ENCODER_CACHE: Lazy<Arc<Mutex<HashMap<String, Arc<CoreBPE>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Map model IDs to their appropriate tiktoken encoder
/// Based on OpenAI model families and Anthropic's use of cl100k_base
fn get_encoder_name(model_id: &str) -> &'static str {
    // O-series and GPT-4o use o200k_base
    if model_id.starts_with("o1") 
        || model_id.starts_with("o3") 
        || model_id.starts_with("o4")
        || model_id.starts_with("gpt-4o") {
        return "o200k_base";
    }
    
    // GPT-4.1, GPT-5, and most modern models use o200k_base
    if model_id.starts_with("gpt-4.1") 
        || model_id.starts_with("gpt-5") 
        || model_id.starts_with("codex-mini") {
        return "o200k_base";
    }
    
    // Anthropic models use cl100k_base (same as GPT-3.5/4)
    if model_id.starts_with("claude") {
        return "cl100k_base";
    }
    
    // Default to cl100k_base for unknown models
    "cl100k_base"
}

/// Get or create a cached encoder for a model
fn get_encoder(model_id: &str) -> Result<Arc<CoreBPE>, String> {
    let encoder_name = get_encoder_name(model_id);
    
    // Check cache first
    {
        let cache = ENCODER_CACHE.lock().unwrap();
        if let Some(encoder) = cache.get(model_id) {
            return Ok(Arc::clone(encoder));
        }
    }
    
    // Create new encoder
    let encoder = match encoder_name {
        "cl100k_base" => cl100k_base().map_err(|e| format!("Failed to load cl100k_base: {}", e))?,
        "o200k_base" => o200k_base().map_err(|e| format!("Failed to load o200k_base: {}", e))?,
        _ => return Err(format!("Unknown encoder: {}", encoder_name)),
    };
    
    let encoder_arc = Arc::new(encoder);
    
    // Cache it
    {
        let mut cache = ENCODER_CACHE.lock().unwrap();
        cache.insert(model_id.to_string(), Arc::clone(&encoder_arc));
    }
    
    Ok(encoder_arc)
}

/// Count tokens in a text string for a given model
pub fn count_tokens(text: &str, model_id: &str) -> Result<usize, String> {
    let encoder = get_encoder(model_id)?;
    let tokens = encoder.encode_with_special_tokens(text);
    Ok(tokens.len())
}

/// Count tokens for multiple text strings (batch)
pub fn count_tokens_batch(texts: &[String], model_id: &str) -> Result<usize, String> {
    let encoder = get_encoder(model_id)?;
    let mut total = 0;
    
    for text in texts {
        let tokens = encoder.encode_with_special_tokens(text);
        total += tokens.len();
    }
    
    Ok(total)
}

/// Clear the encoder cache (useful for tests or memory management)
pub fn clear_cache() {
    let mut cache = ENCODER_CACHE.lock().unwrap();
    cache.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_simple() {
        let text = "Hello, world!";
        let count = count_tokens(text, "claude-3-5-sonnet-20241022").unwrap();
        assert!(count > 0);
        assert!(count < 10); // Should be ~3-4 tokens
    }

    #[test]
    fn test_count_tokens_empty() {
        let count = count_tokens("", "gpt-4o").unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_tokens_batch() {
        let texts = vec![
            "Hello".to_string(),
            "World".to_string(),
            "Test".to_string(),
        ];
        
        let count = count_tokens_batch(&texts, "claude-sonnet-4-5").unwrap();
        assert!(count > 0);
        assert!(count < 20);
    }

    #[test]
    fn test_encoder_mapping_anthropic() {
        assert_eq!(get_encoder_name("claude-3-5-sonnet-20241022"), "cl100k_base");
        assert_eq!(get_encoder_name("claude-sonnet-4-5"), "cl100k_base");
        assert_eq!(get_encoder_name("claude-opus-4-1-20250805"), "cl100k_base");
    }

    #[test]
    fn test_encoder_mapping_openai() {
        assert_eq!(get_encoder_name("gpt-4o"), "o200k_base");
        assert_eq!(get_encoder_name("gpt-4o-mini"), "o200k_base");
        assert_eq!(get_encoder_name("gpt-4.1"), "o200k_base");
        assert_eq!(get_encoder_name("gpt-5-2025-08-07"), "o200k_base");
        assert_eq!(get_encoder_name("o1"), "o200k_base");
        assert_eq!(get_encoder_name("o3-mini"), "o200k_base");
    }

    #[test]
    fn test_encoder_caching() {
        clear_cache();
        
        // First call should cache
        let model = "claude-3-5-sonnet-20241022";
        let _ = count_tokens("test", model).unwrap();
        
        // Second call should use cache (verify by checking it doesn't error)
        let _ = count_tokens("test2", model).unwrap();
        
        // Verify cache has entry
        let cache = ENCODER_CACHE.lock().unwrap();
        assert!(cache.contains_key(model));
    }

    #[test]
    fn test_clear_cache() {
        let _ = count_tokens("test", "gpt-4o").unwrap();
        clear_cache();
        
        let cache = ENCODER_CACHE.lock().unwrap();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_token_counts_realistic() {
        // Test with a realistic code snippet
        let code = r#"
fn main() {
    println!("Hello, world!");
}
"#;
        
        let count = count_tokens(code, "claude-3-5-sonnet-20241022").unwrap();
        assert!(count > 10);
        assert!(count < 30);
    }

    #[test]
    fn test_token_counts_long_text() {
        // Test with longer text
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(100);
        let count = count_tokens(&text, "gpt-4o").unwrap();
        
        // Should be roughly 900-1100 tokens (9 tokens per sentence * 100)
        assert!(count > 800);
        assert!(count < 1200);
    }

    #[test]
    fn test_consistent_counts_same_model() {
        let text = "This is a test message for token counting.";
        
        let count1 = count_tokens(text, "claude-sonnet-4-5").unwrap();
        let count2 = count_tokens(text, "claude-sonnet-4-5").unwrap();
        
        assert_eq!(count1, count2);
    }

    #[test]
    fn test_batch_equals_sum() {
        let texts = vec![
            "Hello world".to_string(),
            "This is a test".to_string(),
        ];
        
        let batch_count = count_tokens_batch(&texts, "gpt-5-2025-08-07").unwrap();
        let sum_count = count_tokens(&texts[0], "gpt-5-2025-08-07").unwrap() 
                      + count_tokens(&texts[1], "gpt-5-2025-08-07").unwrap();
        
        assert_eq!(batch_count, sum_count);
    }
}
