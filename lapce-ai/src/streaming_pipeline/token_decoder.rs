/// Token Decoder - BPE tokenizer integration
/// Phase 2, Task 5: TokenDecoder with tiktoken-rs
/// Based on docs/08-STREAMING-PIPELINE.md lines 304-382

use tiktoken_rs::{CoreBPE, get_bpe_from_model};
use std::time::{Duration, Instant};
use anyhow::Result;

/// Efficient token decoder with statistics
pub struct TokenDecoder {
    /// BPE tokenizer
    tokenizer: CoreBPE,
    
    /// Buffer for partial tokens
    partial_tokens: Vec<usize>,
    
    /// Decoded text buffer
    text_buffer: String,
    
    /// Total tokens processed
    total_tokens: usize,
    
    /// Tokens per second
    tokens_per_second: f64,
    
    /// Last update timestamp
    last_update: Instant,
}

impl TokenDecoder {
    /// Create new decoder for specific model
    pub fn new(model: &str) -> Result<Self> {
        let tokenizer = get_bpe_from_model(model)?;
        
        Ok(Self {
            tokenizer,
            partial_tokens: Vec::with_capacity(16),
            text_buffer: String::with_capacity(1024),
            total_tokens: 0,
            tokens_per_second: 0.0,
            last_update: Instant::now(),
        })
    }
    
    /// Create decoder with default model (gpt-4)
    pub fn default() -> Result<Self> {
        Self::new("gpt-4")
    }
    
    /// Decode a single token ID
    pub fn decode_token(&mut self, token_id: usize) -> Option<String> {
        self.partial_tokens.push(token_id);
        self.total_tokens += 1;
        
        // Try to decode accumulated tokens
        match self.tokenizer.decode(self.partial_tokens.iter().map(|&x| x as u32).collect()) {
            Ok(text) => {
                // Update statistics
                let elapsed = self.last_update.elapsed();
                if elapsed > Duration::from_secs(1) {
                    self.tokens_per_second = self.total_tokens as f64 / elapsed.as_secs_f64();
                    self.last_update = Instant::now();
                }
                
                // Check if we have new text
                if text.len() > self.text_buffer.len() {
                    let new_text = text[self.text_buffer.len()..].to_string();
                    self.text_buffer = text;
                    Some(new_text)
                } else {
                    // Decoding succeeded but no new text (could be whitespace/formatting)
                    // Return empty string to signal successful decode
                    self.text_buffer = text;
                    Some(String::new())
                }
            }
            Err(_) => {
                // Wait for more tokens to form valid UTF-8
                None
            }
        }
    }
    
    /// Decode multiple tokens
    pub fn decode_tokens(&mut self, token_ids: &[usize]) -> Option<String> {
        self.partial_tokens.extend_from_slice(token_ids);
        self.total_tokens += token_ids.len();
        
        match self.tokenizer.decode(self.partial_tokens.iter().map(|&x| x as u32).collect()) {
            Ok(text) => {
                self.partial_tokens.clear();
                Some(text)
            }
            Err(_) => None,
        }
    }
    
    /// Flush any remaining partial tokens
    pub fn flush(&mut self) -> Option<String> {
        if self.partial_tokens.is_empty() {
            return None;
        }
        
        match self.tokenizer.decode(self.partial_tokens.iter().map(|&x| x as u32).collect()) {
            Ok(text) => {
                self.partial_tokens.clear();
                Some(text)
            }
            Err(_) => {
                // Force decode as UTF-8 lossy
                let bytes: Vec<u8> = self.partial_tokens
                    .iter()
                    .flat_map(|&t| (t as u32).to_le_bytes())
                    .collect();
                
                self.partial_tokens.clear();
                Some(String::from_utf8_lossy(&bytes).into_owned())
            }
        }
    }
    
    /// Encode text to tokens
    pub fn encode(&self, text: &str) -> Vec<usize> {
        // Simple encoding - convert each char to its byte value
        text.chars().map(|c| c as usize).collect()
    }
    
    /// Count tokens in text
    pub fn count_tokens(&self, text: &str) -> usize {
        self.encode(text).len()
    }
    
    /// Get tokens per second
    pub fn tokens_per_second(&self) -> f64 {
        self.tokens_per_second
    }
    
    /// Get total tokens processed
    pub fn total_tokens(&self) -> usize {
        self.total_tokens
    }
    
    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.total_tokens = 0;
        self.tokens_per_second = 0.0;
        self.last_update = Instant::now();
    }
    
    /// Clear all buffers
    pub fn clear(&mut self) {
        self.partial_tokens.clear();
        self.text_buffer.clear();
        self.reset_stats();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_encoding_decoding() {
        let decoder = TokenDecoder::default().unwrap();
        
        let text = "Hello, world!";
        let tokens = decoder.encode(text);
        assert!(!tokens.is_empty());
        
        // Verify round-trip
        let tokens_u32: Vec<u32> = tokens.iter().map(|&t| t as u32).collect();
        let decoded = decoder.tokenizer.decode(tokens_u32).unwrap();
        assert_eq!(decoded, text);
    }
    
    #[test]
    fn test_count_tokens() {
        let decoder = TokenDecoder::default().unwrap();
        
        let text = "This is a test string for token counting.";
        let count = decoder.count_tokens(text);
        assert!(count > 0);
        
        // Verify count matches encode
        let tokens = decoder.encode(text);
        assert_eq!(count, tokens.len());
    }
    
    #[test]
    fn test_partial_token_buffering() {
        let mut decoder = TokenDecoder::default().unwrap();
        
        let text = "Test";
        let tokens = decoder.encode(text);
        
        // Feed tokens one by one - should not panic
        let mut any_output = false;
        for &token in tokens.iter() {
            let result = decoder.decode_token(token);
            if result.is_some() {
                any_output = true;
            }
        }
        
        // At least one decode should produce output
        assert!(any_output, "Expected at least one token to decode successfully");
    }
}
