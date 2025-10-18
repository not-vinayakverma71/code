/// Stream Transform - exact translation from codex-reference/api/transform-stream.ts
/// Lines 1-116
use serde::{Deserialize, Serialize};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use anyhow::Result;
use std::collections::HashMap;

/// ApiError structure for stream error handling
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiError {
    pub message: String,
    pub status: Option<u16>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub error_details: Option<Vec<serde_json::Value>>,
}

/// ApiStream type - exact translation line 1
pub type ApiStream = Pin<Box<dyn Stream<Item = ApiStreamChunk> + Send>>;

/// ApiStreamChunk enum - exact translation line 3
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApiStreamChunk {
    #[serde(rename = "text")]
    Text(ApiStreamTextChunk),
    #[serde(rename = "usage")]
    Usage(ApiStreamUsageChunk),
    #[serde(rename = "reasoning")]
    Reasoning(ApiStreamReasoningChunk),
    #[serde(rename = "error")]
    Error(ApiStreamError),
}

/// ApiStreamError - exact translation lines 5-9
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStreamError {
    #[serde(rename = "type")]
    pub chunk_type: String,
    pub name: String,
    pub message: String,
}

/// ApiStreamTextChunk - exact translation lines 11-14
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStreamTextChunk {
    #[serde(rename = "type")]
    pub chunk_type: String,
    pub text: String,
}

/// ApiStreamReasoningChunk - exact translation lines 16-19
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStreamReasoningChunk {
    #[serde(rename = "type")]
    pub chunk_type: String,
    pub text: String,
}

/// ApiStreamUsageChunk - exact translation lines 21-29
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStreamUsageChunk {
    #[serde(rename = "type")]
    pub chunk_type: String,
    #[serde(rename = "inputTokens")]
    pub input_tokens: u32,
    #[serde(rename = "outputTokens")]
    pub output_tokens: u32,
    #[serde(rename = "cacheWriteTokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u32>,
    #[serde(rename = "cacheReadTokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u32>,
    #[serde(rename = "reasoningTokens")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "totalCost")]
    pub total_cost: Option<f64>,
}

impl ApiStreamChunk {
    pub fn text(text: String) -> Self {
        ApiStreamChunk::Text(ApiStreamTextChunk { 
            chunk_type: "text".to_string(),
            text 
        })
    }
    
    pub fn usage(
        input_tokens: u32,
        output_tokens: u32,
        cache_write_tokens: Option<u32>,
        cache_read_tokens: Option<u32>,
        reasoning_tokens: Option<u32>,
        total_cost: Option<f64>,
    ) -> Self {
        ApiStreamChunk::Usage(ApiStreamUsageChunk {
            chunk_type: "usage".to_string(),
            input_tokens,
            output_tokens,
            cache_write_tokens,
            cache_read_tokens,
            reasoning_tokens,
            total_cost,
        })
    }
    
    pub fn reasoning(text: String) -> Self {
        ApiStreamChunk::Reasoning(ApiStreamReasoningChunk { 
            chunk_type: "reasoning".to_string(),
            text 
        })
    }
    
    pub fn error(name: String, message: String) -> Self {
        ApiStreamChunk::Error(ApiStreamError { 
            chunk_type: "error".to_string(),
            name, 
            message 
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stream_chunk_creation() {
        let text = ApiStreamChunk::text("Hello".to_string());
        match text {
            ApiStreamChunk::Text(chunk) => assert_eq!(chunk.text, "Hello"),
            _ => panic!("Wrong type"),
        }
        
        let usage = ApiStreamChunk::usage(100, 50, None, None, None, Some(0.001));
        match usage {
            ApiStreamChunk::Usage(chunk) => {
                assert_eq!(chunk.input_tokens, 100);
                assert_eq!(chunk.output_tokens, 50);
                assert_eq!(chunk.total_cost, Some(0.001));
            }
            _ => panic!("Wrong type"),
        }
        
        let reasoning = ApiStreamChunk::reasoning("Thinking...".to_string());
        match reasoning {
            ApiStreamChunk::Reasoning(chunk) => assert_eq!(chunk.text, "Thinking..."),
            _ => panic!("Wrong type"),
        }
        
        let error = ApiStreamChunk::error("ERR001".to_string(), "Test error".to_string());
        match error {
            ApiStreamChunk::Error(chunk) => {
                assert_eq!(chunk.error, "ERR001");
                assert_eq!(chunk.message, "Test error");
            }
            _ => panic!("Wrong type"),
        }
    }
}
