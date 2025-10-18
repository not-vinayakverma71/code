/// Integration layer - Connects dispatcher to existing AI components
/// 
/// This bridges the new IPC/dispatcher architecture with existing:
/// - MCP tools
/// - AI providers (Claude, OpenAI)
/// - Semantic search
/// - Tree-sitter integration

pub mod tool_bridge;
pub mod provider_bridge;

pub use tool_bridge::ToolBridge;
pub use provider_bridge::ProviderBridge;
