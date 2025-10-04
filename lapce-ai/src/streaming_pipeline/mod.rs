//! Streaming Pipeline Infrastructure
//! 
//! PHASE 1-2 Implementation (Weeks 1-4)
//! 
//! This module contains all streaming-related functionality:
//! - SSE (Server-Sent Events) parsing
//! - Stream token processing  
//! - HTTP stream handling
//! - Backpressure control
//! - Stream transformers
//! - Pipeline orchestration

// PHASE 1 - Core Types ✅ COMPLETE
pub mod sse_event;
pub mod stream_token;
pub mod sse_parser;

// PHASE 2 - Streaming Infrastructure ✅ COMPLETE
pub mod token_decoder;
pub mod http_handler;
pub mod stream_backpressure;
pub mod pipeline;
pub mod transformer;
pub mod builder;
pub mod metrics;

// Integration tests
#[cfg(test)]
pub mod integration_test;

// Legacy components (to be refactored)
pub mod streaming_response;
pub mod stream_transform;
pub mod xml_matcher;
pub mod backpressure_handling;
pub mod types;

// Re-exports for convenience
pub use sse_event::SseEvent;
pub use stream_token::{StreamToken, TextDelta, FunctionCall, ToolCall};
pub use sse_parser::SseParser;
pub use token_decoder::TokenDecoder;
pub use http_handler::{HttpStreamHandler, ResponseExt};
pub use stream_backpressure::{StreamBackpressureController, BackpressureConfig};
pub use pipeline::StreamingPipeline;
pub use transformer::{StreamTransformer, TransformResult, ContentFilter, TokenAccumulator};
pub use builder::StreamPipelineBuilder;
pub use metrics::{StreamMetrics, MetricsSummary};

// Legacy re-exports
pub use streaming_response::*;
pub use stream_transform::*;
pub use backpressure_handling::*;
