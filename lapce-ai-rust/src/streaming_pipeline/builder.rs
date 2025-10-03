/// Stream Pipeline Builder - Builder pattern for pipeline construction
/// Phase 2, Task 10: StreamPipelineBuilder
/// Based on docs/08-STREAMING-PIPELINE.md lines 562-608

use anyhow::Result;
use crate::streaming_pipeline::pipeline::StreamingPipeline;
use crate::streaming_pipeline::transformer::StreamTransformer;
use crate::streaming_pipeline::stream_backpressure::{StreamBackpressureController, BackpressureConfig};
use crate::streaming_pipeline::metrics::StreamMetrics;
use std::sync::Arc;

/// Builder for constructing streaming pipelines
pub struct StreamPipelineBuilder {
    /// Stream transformers to add
    transformers: Vec<Box<dyn StreamTransformer>>,
    
    /// Backpressure configuration
    backpressure_config: BackpressureConfig,
    
    /// Enable metrics collection
    metrics_enabled: bool,
    
    /// Model for token decoder
    model: String,
}

impl StreamPipelineBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            transformers: Vec::new(),
            backpressure_config: BackpressureConfig::default(),
            metrics_enabled: false,
            model: "gpt-4".to_string(),
        }
    }
    
    /// Add a transformer to the pipeline
    pub fn add_transformer<T: StreamTransformer + 'static>(mut self, transformer: T) -> Self {
        self.transformers.push(Box::new(transformer));
        self
    }
    
    /// Configure backpressure
    pub fn with_backpressure(mut self, config: BackpressureConfig) -> Self {
        self.backpressure_config = config;
        self
    }
    
    /// Set initial permits for backpressure
    pub fn with_permits(mut self, permits: usize) -> Self {
        self.backpressure_config.initial_permits = permits;
        self
    }
    
    /// Set buffer size limits
    pub fn with_buffer_limits(mut self, min: usize, max: usize) -> Self {
        self.backpressure_config.min_buffer = min;
        self.backpressure_config.max_buffer = max;
        self
    }
    
    /// Enable metrics collection
    pub fn enable_metrics(mut self) -> Self {
        self.metrics_enabled = true;
        self
    }
    
    /// Set the model for token decoding
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
    
    /// Build the streaming pipeline
    pub fn build(self) -> Result<StreamingPipeline> {
        let mut pipeline = StreamingPipeline::with_config(
            &self.model,
            self.backpressure_config.initial_permits,
            self.metrics_enabled,
        )?;
        
        // Add all transformers
        for transformer in self.transformers {
            pipeline.add_transformer(transformer);
        }
        
        Ok(pipeline)
    }
}

impl Default for StreamPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Preset configurations
impl StreamPipelineBuilder {
    /// Create builder for low latency streaming
    pub fn low_latency() -> Self {
        Self::new()
            .with_permits(200)
            .with_buffer_limits(512, 8192)
    }
    
    /// Create builder for high throughput
    pub fn high_throughput() -> Self {
        Self::new()
            .with_permits(500)
            .with_buffer_limits(4096, 131072)
    }
    
    /// Create builder for memory constrained environments
    pub fn memory_constrained() -> Self {
        Self::new()
            .with_permits(50)
            .with_buffer_limits(256, 2048)
    }
    
    /// Create builder with debug configuration
    pub fn debug() -> Self {
        Self::new()
            .enable_metrics()
            .with_permits(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming_pipeline::transformer::{ContentFilter, TokenAccumulator};
    
    #[test]
    fn test_builder_basic() {
        let pipeline = StreamPipelineBuilder::new()
            .with_model("gpt-3.5-turbo")
            .enable_metrics()
            .build();
        
        assert!(pipeline.is_ok());
    }
    
    #[test]
    fn test_builder_with_transformers() {
        let filter = ContentFilter::new(vec![], String::new()).unwrap();
        let accumulator = TokenAccumulator::default();
        
        let pipeline = StreamPipelineBuilder::new()
            .add_transformer(filter)
            .add_transformer(accumulator)
            .build();
        
        assert!(pipeline.is_ok());
    }
    
    #[test]
    fn test_preset_configurations() {
        let low_latency = StreamPipelineBuilder::low_latency().build();
        assert!(low_latency.is_ok());
        
        let high_throughput = StreamPipelineBuilder::high_throughput().build();
        assert!(high_throughput.is_ok());
        
        let memory_constrained = StreamPipelineBuilder::memory_constrained().build();
        assert!(memory_constrained.is_ok());
        
        let debug = StreamPipelineBuilder::debug().build();
        assert!(debug.is_ok());
    }
}
