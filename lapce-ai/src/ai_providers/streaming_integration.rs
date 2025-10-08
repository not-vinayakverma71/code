/// Streaming Integration - Connect StreamingPipeline to all AI Providers
/// This module provides helpers to integrate the streaming pipeline with providers

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use anyhow::Result;
use reqwest::Response;
use futures::stream::{Stream, StreamExt, BoxStream};

use crate::streaming_pipeline::{
    StreamingPipeline, StreamPipelineBuilder, StreamToken,
    ContentFilter, TokenAccumulator, StreamTransformer,
};
use crate::ai_providers::sse_decoder::{SseDecoder, SseEvent};

/// Provider type for pipeline configuration
#[derive(Debug, Clone)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Gemini,
    Bedrock,
    Azure,
    XAI,
    VertexAI,
}

impl ProviderType {
    /// Get the model name for token decoder
    pub fn default_model(&self) -> &str {
        match self {
            Self::OpenAI | Self::Azure | Self::XAI => "gpt-4",
            Self::Anthropic => "claude-3",
            Self::Gemini | Self::VertexAI => "gemini-pro",
            Self::Bedrock => "claude-v2",
        }
    }
    
    /// Check if provider uses event-based SSE
    pub fn uses_event_sse(&self) -> bool {
        matches!(self, Self::Anthropic | Self::Bedrock)
    }
}

/// Create a configured streaming pipeline for a provider
pub fn create_provider_pipeline(provider: ProviderType) -> Result<Arc<Mutex<StreamingPipeline>>> {
    let pipeline = StreamPipelineBuilder::new()
        .with_model(provider.default_model())
        .with_permits(100) // Initial permits
        .enable_metrics()
        .build()?;
    
    Ok(Arc::new(Mutex::new(pipeline)))
}

/// Process HTTP response through streaming pipeline
pub async fn process_response_with_pipeline(
    response: Response,
    provider: ProviderType,
) -> Result<BoxStream<'static, Result<StreamToken>>> {
    // Create pipeline for this provider
    let pipeline = create_provider_pipeline(provider.clone())?;
    
    // Convert response to bytes stream
    let bytes_stream = response
        .bytes_stream()
        .map(|result| result.map_err(|e| anyhow::anyhow!(e)));
    
    // Process through pipeline
    let token_stream = StreamingPipeline::process_stream_static(
        pipeline,
        bytes_stream,
    ).await;
    
    Ok(token_stream)
}

/// Process SSE response with provider-specific parsing
pub async fn process_sse_response(
    response: Response,
    provider: ProviderType,
    parse_fn: impl Fn(&SseEvent) -> Option<StreamToken> + Send + Sync + 'static,
) -> Result<BoxStream<'static, Result<StreamToken>>> {
    // Create SSE decoder
    let mut decoder = SseDecoder::new();
    
    // Process response stream
    let stream = response
        .bytes_stream()
        .map(move |result| result.map_err(|e| anyhow::anyhow!(e)))
        .flat_map(move |chunk_result| {
            match chunk_result {
                Ok(chunk) => {
                    // Decode SSE events
                    let events = decoder.process_chunk(&chunk);
                    
                    // Parse events to tokens
                    let tokens: Vec<Result<StreamToken>> = events
                        .into_iter()
                        .filter_map(|event| parse_fn(&event))
                        .map(Ok)
                        .collect();
                    
                    futures::stream::iter(tokens)
                }
                Err(e) => futures::stream::iter(vec![Err(e)]),
            }
        });
    
    Ok(Box::pin(stream))
}

/// Advanced streaming with pipeline and transformers
pub async fn create_advanced_stream(
    response: Response,
    provider: ProviderType,
    parse_fn: impl Fn(&SseEvent) -> Option<StreamToken> + Send + Sync + 'static,
    enable_filtering: bool,
    enable_accumulation: bool,
) -> Result<BoxStream<'static, Result<StreamToken>>> {
    // Create pipeline with transformers
    let mut pipeline_builder = StreamPipelineBuilder::new()
        .with_model(provider.default_model())
        .with_permits(100)
        .enable_metrics();
    
    // Add content filter if enabled
    if enable_filtering {
        // Skip content filter for now - implementation issue
        // TODO: Fix StreamTransformer trait implementation
    }
    
    // Add token accumulator if enabled  
    if enable_accumulation {
        pipeline_builder = pipeline_builder.add_transformer(
            TokenAccumulator::new(10, 100)  // Min 10, max 100 chars
        );
    }
    
    let pipeline = Arc::new(Mutex::new(pipeline_builder.build()?));
    
    // Create bytes stream from response
    let bytes_stream = response
        .bytes_stream()
        .map(|result| result.map_err(|e| anyhow::anyhow!(e)));
    
    // Process through pipeline
    let token_stream = StreamingPipeline::process_stream_static(
        pipeline,
        bytes_stream,
    ).await;
    
    Ok(token_stream)
}

/// Helper to create a stream from SSE events directly
pub fn create_token_stream(
    events: Vec<SseEvent>,
    parse_fn: impl Fn(&SseEvent) -> Option<StreamToken>,
) -> BoxStream<'static, Result<StreamToken>> {
    let tokens: Vec<Result<StreamToken>> = events
        .into_iter()
        .filter_map(|event| parse_fn(&event))
        .map(Ok)
        .collect();
    
    Box::pin(futures::stream::iter(tokens))
}

/// Concurrent streaming helper for multiple providers
pub async fn concurrent_provider_streams(
    responses: Vec<(Response, ProviderType)>,
) -> Result<Vec<BoxStream<'static, Result<StreamToken>>>> {
    let mut streams = Vec::new();
    
    for (response, provider) in responses {
        let stream = process_response_with_pipeline(response, provider).await?;
        streams.push(stream);
    }
    
    Ok(streams)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_provider_pipeline_creation() {
        let pipeline = create_provider_pipeline(ProviderType::OpenAI);
        assert!(pipeline.is_ok());
        
        let pipeline = create_provider_pipeline(ProviderType::Anthropic);
        assert!(pipeline.is_ok());
    }
    
    #[tokio::test]
    async fn test_provider_types() {
        assert_eq!(ProviderType::OpenAI.default_model(), "gpt-4");
        assert_eq!(ProviderType::Anthropic.default_model(), "claude-3");
        assert!(ProviderType::Anthropic.uses_event_sse());
        assert!(!ProviderType::OpenAI.uses_event_sse());
    }
}
