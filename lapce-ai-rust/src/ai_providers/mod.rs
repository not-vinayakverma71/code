/// AI Providers Module - 7 PROVIDERS ONLY
/// OpenAI, Anthropic, Gemini, AWS Bedrock, Azure, xAI

// Core components (EXACT from spec)
pub mod core_trait;
pub mod provider_manager;
pub mod sse_decoder;

// 7 Provider implementations (EXACT ports from TypeScript)
pub mod openai_exact;     // 1. OpenAI
pub mod anthropic_exact;   // 2. Anthropic (Claude)
pub mod gemini_exact;      // 3. Google Gemini
pub mod bedrock_exact;     // 4. AWS Bedrock
pub mod azure_exact;       // 5. Azure OpenAI
pub mod xai_exact;         // 6. xAI (Grok)
pub mod vertex_ai_exact;   // 7. GCP Vertex AI (for Gemini on GCP)

// Legacy implementations (removed)
pub mod traits;

// Re-export core types
pub use core_trait::{
    AiProvider, CompletionRequest, CompletionResponse, ChatRequest, ChatResponse,
    StreamToken, HealthStatus, Model, ProviderCapabilities, Usage
};
pub use provider_manager::{ProviderManager, ProvidersConfig, ProviderMetrics};
pub use sse_decoder::{SseDecoder, SseEvent};
