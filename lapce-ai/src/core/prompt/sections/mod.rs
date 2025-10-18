//! Prompt Sections
//!
//! 1:1 Translation from Codex `src/core/prompts/sections/`
//!
//! Each section generates a specific part of the system prompt

pub mod custom_system_prompt;
pub mod custom_instructions;
pub mod markdown_formatting;
pub mod tool_use;
pub mod tool_use_guidelines;
pub mod capabilities;
pub mod system_info;
pub mod objective;
// pub mod modes_section; // TODO: Requires async mode loading
// pub mod rules; // TODO: Complex editing instructions
// pub mod mcp_servers; // TODO: MCP integration

pub use custom_system_prompt::{load_system_prompt_file, PromptVariables};
pub use custom_instructions::add_custom_instructions;
pub use markdown_formatting::markdown_formatting_section;
pub use tool_use::shared_tool_use_section;
pub use tool_use_guidelines::tool_use_guidelines_section;
pub use capabilities::{capabilities_section, DiffStrategy};
pub use system_info::system_info_section;
pub use objective::objective_section;
