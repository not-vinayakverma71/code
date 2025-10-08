/// Stream Transformers - Transform tokens in the pipeline
/// Phase 2, Task 9: StreamTransformers (ContentFilter, TokenAccumulator)
/// Based on docs/08-STREAMING-PIPELINE.md lines 498-556

use regex::Regex;
use anyhow::Error;
use crate::streaming_pipeline::stream_token::StreamToken;

/// Result of a transformation
#[derive(Debug)]
pub enum TransformResult {
    /// Pass token unchanged
    Pass,
    /// Skip this token
    Skip,
    /// Replace with new token
    Replace(StreamToken),
    /// Error occurred
    Error(Error),
}

/// Trait for stream transformers
pub trait StreamTransformer: Send + Sync {
    /// Transform a token
    fn transform(&mut self, token: &mut StreamToken) -> TransformResult;
    
    /// Reset transformer state
    fn reset(&mut self) {}
}

/// Content filter transformer - filters/replaces content
pub struct ContentFilter {
    /// Patterns to block
    blocked_patterns: Vec<Regex>,
    
    /// Replacement text
    replacement: String,
}

impl ContentFilter {
    /// Create new content filter
    pub fn new(patterns: Vec<String>, replacement: String) -> Result<Self, regex::Error> {
        let blocked_patterns = patterns
            .into_iter()
            .map(|p| Regex::new(&p))
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(Self {
            blocked_patterns,
            replacement,
        })
    }
    
    /// Create filter for common profanity
    pub fn profanity_filter() -> Result<Self, regex::Error> {
        let patterns = vec![
            r"(?i)\b(damn|hell|crap)\b".to_string(),
            // Add more patterns as needed
        ];
        
        Self::new(patterns, "[filtered]".to_string())
    }
}

impl StreamTransformer for ContentFilter {
    fn transform(&mut self, token: &mut StreamToken) -> TransformResult {
        match token {
            StreamToken::Text(text) => {
                let mut modified = false;
                let mut result = text.clone();
                
                for pattern in &self.blocked_patterns {
                    if pattern.is_match(&result) {
                        result = pattern.replace_all(&result, &self.replacement).into_owned();
                        modified = true;
                    }
                }
                
                if modified {
                    *text = result;
                }
                
                TransformResult::Pass
            }
            StreamToken::Delta(delta) => {
                let mut modified = false;
                let mut result = delta.content.clone();
                
                for pattern in &self.blocked_patterns {
                    if pattern.is_match(&result) {
                        result = pattern.replace_all(&result, &self.replacement).into_owned();
                        modified = true;
                    }
                }
                
                if modified {
                    delta.content = result;
                }
                
                TransformResult::Pass
            }
            _ => TransformResult::Pass,
        }
    }
}

/// Token accumulator - buffers tokens until size threshold
pub struct TokenAccumulator {
    /// Buffer for accumulating text
    buffer: String,
    
    /// Minimum chunk size before emitting
    min_chunk_size: usize,
    
    /// Maximum chunk size (force emit)
    max_chunk_size: usize,
}

impl TokenAccumulator {
    /// Create new accumulator
    pub fn new(min_chunk_size: usize, max_chunk_size: usize) -> Self {
        Self {
            buffer: String::new(),
            min_chunk_size,
            max_chunk_size,
        }
    }
    
    /// Create with default sizes
    pub fn default() -> Self {
        Self::new(10, 100)
    }
}

impl StreamTransformer for TokenAccumulator {
    fn transform(&mut self, token: &mut StreamToken) -> TransformResult {
        match token {
            StreamToken::Text(text) => {
                self.buffer.push_str(text);
                
                if self.buffer.len() >= self.min_chunk_size {
                    let chunk = std::mem::take(&mut self.buffer);
                    TransformResult::Replace(StreamToken::Text(chunk))
                } else {
                    TransformResult::Skip
                }
            }
            StreamToken::Done => {
                if !self.buffer.is_empty() {
                    let chunk = std::mem::take(&mut self.buffer);
                    TransformResult::Replace(StreamToken::Text(chunk))
                } else {
                    TransformResult::Pass
                }
            }
            _ => TransformResult::Pass,
        }
    }
    
    fn reset(&mut self) {
        self.buffer.clear();
    }
}

/// Rate limiter transformer - limits tokens per second
pub struct RateLimiter {
    /// Maximum tokens per second
    max_tokens_per_second: usize,
    
    /// Last emission time
    last_emission: std::time::Instant,
    
    /// Token count in current second
    tokens_this_second: usize,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(max_tokens_per_second: usize) -> Self {
        Self {
            max_tokens_per_second,
            last_emission: std::time::Instant::now(),
            tokens_this_second: 0,
        }
    }
}

impl StreamTransformer for RateLimiter {
    fn transform(&mut self, _token: &mut StreamToken) -> TransformResult {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_emission);
        
        if elapsed >= std::time::Duration::from_secs(1) {
            // Reset counter for new second
            self.tokens_this_second = 0;
            self.last_emission = now;
        }
        
        self.tokens_this_second += 1;
        
        if self.tokens_this_second > self.max_tokens_per_second {
            // Skip token to maintain rate limit
            TransformResult::Skip
        } else {
            TransformResult::Pass
        }
    }
    
    fn reset(&mut self) {
        self.tokens_this_second = 0;
        self.last_emission = std::time::Instant::now();
    }
}

/// Identity transformer - passes everything through unchanged
pub struct IdentityTransformer;

impl StreamTransformer for IdentityTransformer {
    fn transform(&mut self, _token: &mut StreamToken) -> TransformResult {
        TransformResult::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_content_filter() {
        let mut filter = ContentFilter::new(
            vec![r"bad".to_string()],
            "[filtered]".to_string()
        ).unwrap();
        
        let mut token = StreamToken::Text("This is bad content".to_string());
        filter.transform(&mut token);
        
        match token {
            StreamToken::Text(text) => {
                assert!(text.contains("[filtered]"));
                assert!(!text.contains("bad"));
            }
            _ => panic!("Expected Text token"),
        }
    }
    
    #[test]
    fn test_token_accumulator() {
        let mut accumulator = TokenAccumulator::new(10, 100);
        
        // Small token - should be skipped
        let mut token1 = StreamToken::Text("Hi".to_string());
        let result1 = accumulator.transform(&mut token1);
        assert!(matches!(result1, TransformResult::Skip));
        
        // Add more to reach threshold
        let mut token2 = StreamToken::Text(" there friend".to_string());
        let result2 = accumulator.transform(&mut token2);
        
        match result2 {
            TransformResult::Replace(StreamToken::Text(text)) => {
                assert_eq!(text, "Hi there friend");
            }
            _ => panic!("Expected Replace with accumulated text"),
        }
    }
    
    #[test]
    fn test_accumulator_flush_on_done() {
        let mut accumulator = TokenAccumulator::new(10, 100);
        
        let mut token1 = StreamToken::Text("Small".to_string());
        accumulator.transform(&mut token1);
        
        let mut done = StreamToken::Done;
        let result = accumulator.transform(&mut done);
        
        match result {
            TransformResult::Replace(StreamToken::Text(text)) => {
                assert_eq!(text, "Small");
            }
            _ => panic!("Expected Replace with buffered text on Done"),
        }
    }
}
