/// Streaming Pipeline - Core orchestrator for stream processing
/// Phase 2, Task 8: StreamingPipeline orchestrator
/// Based on docs/08-STREAMING-PIPELINE.md lines 69-696

use std::sync::Arc;
use std::time::Instant;
use bytes::{Bytes, BytesMut};
use futures::stream::{Stream, StreamExt, BoxStream};
use tokio::sync::{mpsc, Mutex};
use anyhow::Result;
use async_stream::stream;

use crate::streaming_pipeline::sse_parser::SseParser;
use crate::streaming_pipeline::sse_event::SseEvent;
use crate::streaming_pipeline::stream_token::StreamToken;
use crate::streaming_pipeline::token_decoder::TokenDecoder;
use crate::streaming_pipeline::stream_backpressure::StreamBackpressureController;
use crate::streaming_pipeline::transformer::{StreamTransformer, TransformResult};
use crate::streaming_pipeline::metrics::StreamMetrics;

/// Main streaming pipeline for processing token streams
pub struct StreamingPipeline {
    /// SSE parser for event processing
    sse_parser: SseParser,
    
    /// Token decoder for BPE decoding
    token_decoder: TokenDecoder,
    
    /// Backpressure controller
    backpressure: StreamBackpressureController,
    
    /// Stream transformers
    transformers: Vec<Box<dyn StreamTransformer>>,
    
    /// Metrics collector
    metrics: Arc<StreamMetrics>,
}

impl StreamingPipeline {
    /// Create new streaming pipeline
    pub fn new() -> Result<Self> {
        Ok(Self {
            sse_parser: SseParser::new(),
            token_decoder: TokenDecoder::default()?,
            backpressure: StreamBackpressureController::default(),
            transformers: Vec::new(),
            metrics: Arc::new(StreamMetrics::new()),
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(
        model: &str,
        initial_permits: usize,
        enable_metrics: bool,
    ) -> Result<Self> {
        Ok(Self {
            sse_parser: SseParser::new(),
            token_decoder: TokenDecoder::new(model)?,
            backpressure: StreamBackpressureController::new(initial_permits),
            transformers: Vec::new(),
            metrics: if enable_metrics {
                Arc::new(StreamMetrics::new())
            } else {
                Arc::new(StreamMetrics::noop())
            },
        })
    }
    
    /// Add a transformer to the pipeline
    pub fn add_transformer(&mut self, transformer: Box<dyn StreamTransformer>) {
        self.transformers.push(transformer);
    }
    
    /// Process a stream of bytes into stream tokens
    pub async fn process_stream_static<S>(
        pipeline: Arc<Mutex<Self>>,
        stream: S,
    ) -> BoxStream<'static, Result<StreamToken>>
    where
        S: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(100);
        
        // Spawn processing task
        tokio::spawn(async move {
            let mut stream = Box::pin(stream);
            
            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        // Acquire backpressure permit
                        let backpressure = {
                            let p = pipeline.lock().await;
                            p.backpressure.clone()
                        };
                        let permit = match backpressure.acquire().await {
                            Ok(permit) => permit,
                            Err(e) => {
                                let _ = tx.send(Err(e)).await;
                                break;
                            }
                        };
                        
                        // Process chunk
                        let start = Instant::now();
                        let tokens = {
                            let mut p = pipeline.lock().await;
                            p.process_chunk(chunk)
                        };
                        let duration = start.elapsed();
                        
                        // Send tokens
                        for token in tokens {
                            if tx.send(Ok(token)).await.is_err() {
                                break;
                            }
                        }
                        
                        // Update backpressure
                        {
                            let p = pipeline.lock().await;
                            let backpressure = p.backpressure.clone();
                            drop(p);
                            backpressure.adapt_capacity(duration).await;
                        }
                        
                        drop(permit);
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        break;
                    }
                }
            }
        });
        
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }
    
    /// Process a single chunk
    fn process_chunk(&mut self, chunk: Bytes) -> Vec<StreamToken> {
        // Parse SSE events
        let events = self.sse_parser.parse_chunk(&chunk);
        let mut tokens = Vec::new();
        
        for event in events {
            if let Some(mut token) = self.parse_token(event) {
                // Apply transformers
                for transformer in &mut self.transformers {
                    match transformer.transform(&mut token) {
                        TransformResult::Pass => continue,
                        TransformResult::Skip => break,
                        TransformResult::Replace(new_token) => {
                            token = new_token;
                        }
                        TransformResult::Error(e) => {
                            tokens.push(StreamToken::Error(e.to_string()));
                            break;
                        }
                    }
                }
                
                tokens.push(token);
            }
        }
        
        // Update metrics
        self.metrics.record_chunk(chunk.len(), tokens.len());
        
        tokens
    }
    
    /// Parse token from SSE event
    fn parse_token(&mut self, event: SseEvent) -> Option<StreamToken> {
        // Check for [DONE] marker
        if event.is_done() {
            return Some(StreamToken::Done);
        }
        
        // Try to parse as JSON
        if let Ok(data) = event.parse_json::<serde_json::Value>() {
            // Extract content from various formats
            if let Some(content) = self.extract_content(&data) {
                return Some(StreamToken::Text(content));
            }
        }
        
        // Fallback to raw text
        if let Ok(text) = std::str::from_utf8(&event.data) {
            if !text.trim().is_empty() {
                return Some(StreamToken::Text(text.to_string()));
            }
        }
        
        None
    }
    
    /// Extract content from JSON data
    fn extract_content(&self, data: &serde_json::Value) -> Option<String> {
        // OpenAI format
        if let Some(choices) = data.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(delta) = choice.get("delta") {
                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                        return Some(content.to_string());
                    }
                }
            }
        }
        
        // Anthropic format
        if let Some(delta) = data.get("delta") {
            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                return Some(text.to_string());
            }
        }
        
        // Direct text field
        if let Some(text) = data.get("text").and_then(|t| t.as_str()) {
            return Some(text.to_string());
        }
        
        None
    }
    
    /// Get pipeline metrics
    pub fn metrics(&self) -> Arc<StreamMetrics> {
        Arc::clone(&self.metrics)
    }
    
    /// Reset pipeline state
    pub async fn reset(&mut self) {
        self.sse_parser.clear();
        self.token_decoder.clear();
        self.backpressure.reset().await;
        self.metrics.reset();
    }
}

impl Default for StreamingPipeline {
    fn default() -> Self {
        Self::new().expect("Failed to create default pipeline")
    }
}

/// Simple stream processing without Arc<Mutex>
impl StreamingPipeline {
    /// Process stream directly (simpler API)
    pub fn process_simple<S>(
        mut self,
        stream: S,
    ) -> impl Stream<Item = Result<StreamToken>> + Send + 'static
    where
        S: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        stream! {
            let mut stream = Box::pin(stream);
            
            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        let tokens = self.process_chunk(chunk);
                        for token in tokens {
                            yield Ok(token);
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    
    #[tokio::test]
    async fn test_pipeline_processing() {
        let pipeline = StreamingPipeline::new().unwrap();
        
        // Create test stream
        let chunks = vec![
            Ok(Bytes::from("data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n")),
            Ok(Bytes::from("data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n")),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];
        
        let input_stream = stream::iter(chunks);
        let mut output_stream = Box::pin(pipeline.process_simple(input_stream));
        
        // Collect tokens
        let mut tokens = Vec::new();
        while let Some(result) = output_stream.next().await {
            if let Ok(token) = result {
                tokens.push(token);
            }
        }
        
        // Should have tokens including Done
        assert!(!tokens.is_empty());
        assert!(tokens.iter().any(|t| t.is_done()));
    }
    
    #[test]
    fn test_extract_content() {
        let pipeline = StreamingPipeline::new().unwrap();
        
        // Test OpenAI format
        let openai_data = serde_json::json!({
            "choices": [{
                "delta": {
                    "content": "test content"
                }
            }]
        });
        
        let content = pipeline.extract_content(&openai_data);
        assert_eq!(content, Some("test content".to_string()));
        
        // Test Anthropic format
        let anthropic_data = serde_json::json!({
            "delta": {
                "text": "anthropic content"
            }
        });
        
        let content = pipeline.extract_content(&anthropic_data);
        assert_eq!(content, Some("anthropic content".to_string()));
    }
}
