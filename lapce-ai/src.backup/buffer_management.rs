use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Buffer management module
/// Re-exports stream types from stream_transform.rs to avoid duplication

// Re-export all stream types from stream_transform module
pub use crate::streaming_pipeline::stream_transform::{
    ApiStream, 
    ApiStreamChunk, 
    ApiStreamTextChunk, 
    ApiStreamUsageChunk, 
    ApiStreamReasoningChunk, 
    ApiStreamError
};

// ApiStreamChunk methods are implemented in stream_transform.rs

impl ApiStreamChunk {
    pub fn is_text(&self) -> bool {
        matches!(self, ApiStreamChunk::Text(_))
    }
    
    pub fn is_usage(&self) -> bool {
        matches!(self, ApiStreamChunk::Usage(_))
    }
    
    pub fn is_reasoning(&self) -> bool {
        matches!(self, ApiStreamChunk::Reasoning(_))
    }
    
    pub fn is_error(&self) -> bool {
        matches!(self, ApiStreamChunk::Error(_))
    }
    
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ApiStreamChunk::Text(chunk) => Some(&chunk.text),
            ApiStreamChunk::Reasoning(chunk) => Some(&chunk.text),
            _ => None,
        }
    }
}

/// Stream buffer for accumulating chunks
pub struct StreamBuffer {
    chunks: Vec<ApiStreamChunk>,
    text_buffer: String,
    reasoning_buffer: String,
    total_input_tokens: u32,
    total_output_tokens: u32,
}

impl StreamBuffer {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            text_buffer: String::new(),
            reasoning_buffer: String::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
        }
    }
    
    pub fn push(&mut self, chunk: ApiStreamChunk) {
        match &chunk {
            ApiStreamChunk::Text(text_chunk) => {
                self.text_buffer.push_str(&text_chunk.text);
            }
            ApiStreamChunk::Reasoning(reasoning_chunk) => {
                self.reasoning_buffer.push_str(&reasoning_chunk.text);
            }
            ApiStreamChunk::Usage(usage_chunk) => {
                self.total_input_tokens += usage_chunk.input_tokens;
                self.total_output_tokens += usage_chunk.output_tokens;
            }
            _ => {}
        }
        self.chunks.push(chunk);
    }
    
    pub fn get_text(&self) -> &str {
        &self.text_buffer
    }
    
    pub fn get_reasoning(&self) -> &str {
        &self.reasoning_buffer
    }
    
    pub fn get_chunks(&self) -> &[ApiStreamChunk] {
        &self.chunks
    }
    
    pub fn get_usage(&self) -> (u32, u32) {
        (self.total_input_tokens, self.total_output_tokens)
    }
    
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.text_buffer.clear();
        self.reasoning_buffer.clear();
        self.total_input_tokens = 0;
        self.total_output_tokens = 0;
    }
}

/// AsyncGenerator implementation for API streams
pub struct ApiStreamGenerator {
    buffer: StreamBuffer,
    is_complete: bool,
}

impl ApiStreamGenerator {
    pub fn new() -> Self {
        Self {
            buffer: StreamBuffer::new(),
            is_complete: false,
        }
    }
    
    pub fn push_chunk(&mut self, chunk: ApiStreamChunk) {
        self.buffer.push(chunk);
    }
    
    pub fn complete(&mut self) {
        self.is_complete = true;
    }
    
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }
}

impl Stream for ApiStreamGenerator {
    type Item = ApiStreamChunk;
    
    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.is_complete && self.buffer.chunks.is_empty() {
            Poll::Ready(None)
        } else if !self.buffer.chunks.is_empty() {
            Poll::Ready(Some(self.buffer.chunks.remove(0)))
        } else {
            Poll::Pending
        }
    }
}

mod tests {
    
    
    #[test]
    fn test_stream_chunk_creation() {
        let text_chunk = crate::streaming_pipeline::stream_transform::ApiStreamChunk::text("Hello".to_string());
        assert!(text_chunk.is_text());
        assert_eq!(text_chunk.as_text(), Some("Hello"));
        
        let usage_chunk = crate::streaming_pipeline::stream_transform::ApiStreamChunk::usage(100, 50, None, None, None, None);
        assert!(usage_chunk.is_usage());
        
        let error_chunk = crate::streaming_pipeline::stream_transform::ApiStreamChunk::error("ERROR".to_string(), "Test error".to_string());
        assert!(error_chunk.is_error());
    }
    
    #[test]
    fn test_stream_buffer() {
        let mut buffer = StreamBuffer::new();
        
        buffer.push(crate::streaming_pipeline::stream_transform::ApiStreamChunk::text("Hello ".to_string()));
        buffer.push(crate::streaming_pipeline::stream_transform::ApiStreamChunk::text("World".to_string()));
        buffer.push(crate::streaming_pipeline::stream_transform::ApiStreamChunk::usage(10, 5, None, None, None, None));
        
        assert_eq!(buffer.get_text(), "Hello World");
        assert_eq!(buffer.get_usage(), (10, 5));
        assert_eq!(buffer.get_chunks().len(), 3);
    }
}
