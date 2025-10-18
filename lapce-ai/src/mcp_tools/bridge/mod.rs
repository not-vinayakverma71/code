// Bridge module: Wires core/tools production tool suite into MCP
// Enables full production tools (no mocks) with approvals/safety/rooignore/streaming

pub mod result;
pub mod context;
pub mod core_tool_adapter;

#[cfg(test)]
mod tests;

pub use result::core_output_to_mcp;
pub use context::{to_core_context, to_core_context_with_adapters, ContextConversionOptions};
pub use core_tool_adapter::CoreToolAsMcp;
