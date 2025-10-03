/// Stream Token Types - Core streaming infrastructure
/// Phase 1, Task 2: Create StreamToken type

use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Stream token representing a chunk of AI response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamToken {
    /// Plain text content
    Text(String),
    
    /// Delta text with metadata
    Delta(TextDelta),
    
    /// Function call in progress
    FunctionCall(FunctionCall),
    
    /// Tool call in progress
    ToolCall(ToolCall),
    
    /// Stream completed
    Done,
    
    /// Error occurred
    Error(String),
}

/// Text delta with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDelta {
    pub content: String,
    pub index: usize,
    pub logprob: Option<f32>,
}

/// Function call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

impl StreamToken {
    /// Merge two tokens if compatible
    pub fn merge(&mut self, other: StreamToken) -> Result<()> {
        match (self, other) {
            (StreamToken::Text(s1), StreamToken::Text(s2)) => {
                s1.push_str(&s2);
                Ok(())
            }
            (StreamToken::Delta(d1), StreamToken::Delta(d2)) => {
                d1.content.push_str(&d2.content);
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Incompatible tokens for merging")),
        }
    }
    
    /// Extract text content if available
    pub fn as_text(&self) -> Option<&str> {
        match self {
            StreamToken::Text(s) => Some(s),
            StreamToken::Delta(d) => Some(&d.content),
            _ => None,
        }
    }
    
    /// Check if token represents completion
    pub fn is_done(&self) -> bool {
        matches!(self, StreamToken::Done)
    }
    
    /// Check if token is an error
    pub fn is_error(&self) -> bool {
        matches!(self, StreamToken::Error(_))
    }
}

/// Stream data from providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamData {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

/// Individual choice in stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: usize,
    pub delta: ChoiceDelta,
    pub finish_reason: Option<String>,
}

/// Delta content in a choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub function_call: Option<FunctionCall>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl From<StreamData> for StreamToken {
    fn from(data: StreamData) -> Self {
        if let Some(choice) = data.choices.first() {
            if let Some(content) = &choice.delta.content {
                return StreamToken::Text(content.clone());
            }
            if let Some(function_call) = &choice.delta.function_call {
                return StreamToken::FunctionCall(function_call.clone());
            }
            if let Some(tool_calls) = &choice.delta.tool_calls {
                if let Some(tool_call) = tool_calls.first() {
                    return StreamToken::ToolCall(tool_call.clone());
                }
            }
            if choice.finish_reason.is_some() {
                return StreamToken::Done;
            }
        }
        StreamToken::Text(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stream_token_merge() {
        let mut token1 = StreamToken::Text("Hello ".to_string());
        let token2 = StreamToken::Text("World".to_string());
        assert!(token1.merge(token2).is_ok());
        
        match token1 {
            StreamToken::Text(s) => assert_eq!(s, "Hello World"),
            _ => panic!("Expected Text variant"),
        }
    }
    
    #[test]
    fn test_token_predicates() {
        let done = StreamToken::Done;
        assert!(done.is_done());
        assert!(!done.is_error());
        
        let error = StreamToken::Error("test error".to_string());
        assert!(error.is_error());
        assert!(!error.is_done());
    }
}
