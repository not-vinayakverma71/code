// Core tools module - Production-grade tool execution system for Lapce AI
// Implements P0 comprehensive TODO for tool translation from TypeScript

pub mod adapters;
pub mod permissions;
pub mod logging;
pub mod config;
pub mod util;
pub mod fs;
pub mod execute_command;

pub mod diff_engine;
pub mod diff_tool;
pub mod streaming;
pub mod terminal;
pub mod expanded_tools;
pub mod expanded_tools_v2;
pub mod expanded_tools_registry;
pub mod error_recovery;
pub mod error_recovery_v2;
pub mod diff_view_streaming;
pub mod rooignore_enforcement;
pub mod rooignore_unified;
pub mod streaming_v2;
pub mod security_hardening;
pub mod approval_v2;
pub mod diff_engine_v2;
pub mod search;
pub mod list_files;
pub mod observability;

pub mod traits;
pub mod registry;
pub mod xml_util;

pub use traits::{Tool, ToolContext, ToolOutput, ToolResult, ToolError, ApprovalRequired};
pub use registry::{ToolRegistry, ToolMetadata};

// Re-export adapters for convenient access
pub use adapters::{
    ipc::IpcAdapter,
    lapce_diff::DiffAdapter,
    lapce_terminal::TerminalAdapter,
};

// Re-export permissions
pub use permissions::rooignore::RooIgnore;
pub use rooignore_unified::UnifiedRooIgnore;
pub use security_hardening::{validate_path_security, validate_command_security};
pub use error_recovery_v2::{ErrorRecoveryV2, ErrorCode};
pub use streaming_v2::{UnifiedStreamEmitter, StreamEvent};
pub use expanded_tools_registry::{ExpandedToolRegistry, TOOL_REGISTRY};

// Re-export utilities
pub use util::xml::{parse_tool_xml, generate_tool_xml, XmlToolArgs};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod security_tests;

// Re-export V2 tools for convenience
pub use expanded_tools_v2::{
    GitStatusToolV2, GitDiffToolV2,
    Base64ToolV2, JsonFormatToolV2,
    EnvironmentToolV2, ProcessListToolV2,
    FileSizeToolV2, CountLinesToolV2,
    ZipToolV2, CurlToolV2,
};
