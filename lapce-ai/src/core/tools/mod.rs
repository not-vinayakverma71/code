// Core tools module - Production-grade tool execution system for Lapce AI
// Implements P0 comprehensive TODO for tool translation from TypeScript

pub mod adapters;
pub mod permissions;
pub mod util;
pub mod fs;
pub mod execute_command;
pub mod diff_engine;
pub mod diff_tool;

mod traits;
mod registry;
mod xml_util;

pub use traits::{Tool, ToolContext, ToolResult, ToolError, ApprovalRequired};
pub use registry::{ToolRegistry, ToolMetadata};

// Re-export adapters for convenient access
pub use adapters::{
    ipc::IpcAdapter,
    lapce_diff::DiffAdapter,
    lapce_terminal::TerminalAdapter,
};

// Re-export permissions
pub use permissions::rooignore::RooIgnore;

// Re-export utilities
pub use util::xml::{parse_tool_xml, generate_tool_xml, XmlToolArgs};

#[cfg(test)]
mod tests;
