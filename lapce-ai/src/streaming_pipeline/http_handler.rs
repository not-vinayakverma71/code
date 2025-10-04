/// HTTP Stream Handler - Convert HTTP responses to token streams
/// Phase 2, Task 6: HttpStreamHandler
/// Based on docs/08-STREAMING-PIPELINE.md lines 240-299

use bytes::{Bytes, BytesMut};
use futures::stream::{Stream, StreamExt};
use reqwest::Response;
use std::pin::Pin;
use std::task::{Context, Poll};
use anyhow::Result;
use async_stream::stream;

use crate::streaming_pipeline::sse_parser::SseParser;
use crate::streaming_pipeline::sse_event::SseEvent;
use crate::streaming_pipeline::stream_token::{StreamToken, StreamData};

/// HTTP streaming response handler
pub struct HttpStreamHandler {
    /// SSE parser for processing chunks
    sse_parser: SseParser,
    
    /// Buffer for incomplete data
    buffer: BytesMut,
}

impl HttpStreamHandler {
    /// Create new HTTP stream handler
    pub fn new() -> Self {
        Self {
            sse_parser: SseParser::new(),
            buffer: BytesMut::with_capacity(4096),
        }
    }
    
    /// Convert HTTP response to stream of tokens
    pub fn into_stream(
        mut self,
        response: Response,
    ) -> impl Stream<Item = Result<StreamToken>> + Send + 'static {
        stream! {
            let mut stream = response.bytes_stream();
            
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        // Parse SSE events from chunk
                        let events = self.sse_parser.parse_chunk(&chunk);
                        
                        for event in events {
                            // Convert SSE event to StreamToken
                            if let Some(token) = Self::parse_token_from_event(event) {
                                yield Ok(token);
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(anyhow::anyhow!("HTTP stream error: {}", e));
                        break;
                    }
                }
            }
            
            // Flush any remaining data
            if let Some(token) = self.flush_remaining() {
                yield Ok(token);
            }
        }
    }
    
    /// Parse token from SSE event
    fn parse_token_from_event(event: SseEvent) -> Option<StreamToken> {
        // Check for [DONE] marker
        if event.is_done() {
            return Some(StreamToken::Done);
        }
        
        // Try to parse as JSON
        match serde_json::from_slice::<StreamData>(&event.data) {
            Ok(stream_data) => Some(StreamToken::from(stream_data)),
            Err(_) => {
                // Try parsing as plain text for simple responses
                if let Ok(text) = std::str::from_utf8(&event.data) {
                    if !text.trim().is_empty() {
                        return Some(StreamToken::Text(text.to_string()));
                    }
                }
                None
            }
        }
    }
    
    /// Flush any remaining data as final token
    fn flush_remaining(&mut self) -> Option<StreamToken> {
        let remaining = self.sse_parser.remaining().to_vec();
        if !remaining.is_empty() {
            if let Ok(text) = std::str::from_utf8(&remaining) {
                if !text.trim().is_empty() {
                    self.sse_parser.clear();
                    return Some(StreamToken::Text(text.to_string()));
                }
            }
        }
        None
    }
    
    /// Process raw bytes into SSE events
    pub fn process_bytes(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        self.sse_parser.parse_chunk(bytes)
    }
    
    /// Create stream from raw byte stream
    pub fn from_byte_stream<S>(
        mut self,
        mut stream: S,
    ) -> impl Stream<Item = Result<StreamToken>> + Send + 'static
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin + 'static,
    {
        stream! {
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let events = self.sse_parser.parse_chunk(&chunk);
                        
                        for event in events {
                            if let Some(token) = Self::parse_token_from_event(event) {
                                yield Ok(token);
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Stream error: {}", e));
                        break;
                    }
                }
            }
            
            if let Some(token) = self.flush_remaining() {
                yield Ok(token);
            }
        }
    }
}

impl Default for HttpStreamHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for Response
pub trait ResponseExt {
    /// Convert response to token stream
    fn into_token_stream(self) -> Pin<Box<dyn Stream<Item = Result<StreamToken>> + Send + 'static>>;
}

impl ResponseExt for Response {
    fn into_token_stream(self) -> Pin<Box<dyn Stream<Item = Result<StreamToken>> + Send + 'static>> {
        let handler = HttpStreamHandler::new();
        Box::pin(handler.into_stream(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures::stream;
    
    #[tokio::test]
    async fn test_parse_stream_tokens() {
        let handler = HttpStreamHandler::new();
        
        // Create mock SSE stream
        let chunks = vec![
            Ok(Bytes::from("data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n")),
            Ok(Bytes::from("data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n")),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];
        
        let byte_stream = stream::iter(chunks);
        let mut token_stream = Box::pin(handler.from_byte_stream(byte_stream));
        
        // Collect tokens
        let mut tokens = Vec::new();
        while let Some(result) = token_stream.next().await {
            if let Ok(token) = result {
                tokens.push(token);
            }
        }
        
        assert!(tokens.len() >= 1);
        assert!(tokens.iter().any(|t| t.is_done()));
    }
    
    #[test]
    fn test_parse_done_event() {
        let event = SseEvent::new(b"[DONE]");
        let token = HttpStreamHandler::parse_token_from_event(event);
        
        assert!(matches!(token, Some(StreamToken::Done)));
    }
}
