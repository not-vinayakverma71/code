/// AI Providers Module - 7 PROVIDERS ONLY
/// OpenAI, Anthropic, Gemini, AWS Bedrock, Azure, xAI, Vertex AI

// Core components
pub mod core_trait;
pub mod provider_manager;
pub mod provider_registry;  // Core infrastructure for managing providers
pub mod sse_decoder;
pub mod message_converters;
pub mod traits;
pub mod streaming_integration;  // NEW: Connect StreamingPipeline to providers

// 7 Provider implementations (EXACT ports from TypeScript)
pub mod openai_exact;       // 1. OpenAI - FIXED: Real SSE streaming
pub mod anthropic_exact;    // 2. Anthropic (Claude) - Complete
pub mod gemini_exact;        // 3. Google Gemini - Complete
pub mod gemini_optimized;    // 3. Google Gemini - Optimized
pub mod gemini_ultra_optimized; // 3. Google Gemini - Ultra Optimized < 8MB
pub mod bedrock_exact;       // 4. AWS Bedrock - FIXED: Event-stream parsing
pub mod azure_exact;         // 5. Azure OpenAI - Complete
pub mod xai_exact;           // 6. xAI (Grok) - FIXED: Added completion methods
pub mod vertex_ai_exact;     // 7. GCP Vertex AI - FIXED: Real streaming

// Stub implementations (commented out - not in requirements)
pub mod openrouter_exact;    // Stub only - kept for compatibility
// pub mod perplexity_exact;    // Not implemented - mentioned but not required
// pub mod groq_exact;          // Not implemented - mentioned but not required

// Re-export core types
pub use core_trait::{
    AiProvider, CompletionRequest, CompletionResponse, ChatRequest, ChatResponse,
    StreamToken, HealthStatus, Model, ProviderCapabilities, Usage
};
pub use provider_manager::{ProviderManager, ProvidersConfig, ProviderMetrics};
pub use sse_decoder::{SseDecoder, SseEvent};
