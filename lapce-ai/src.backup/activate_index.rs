/// Exact 1:1 Translation of TypeScript activate/index.ts from codex-reference/activate/index.ts
/// DAY 9 H1-2: Translate activate/index.ts

// Export modules - exact translation lines 1-5
pub mod handle_uri;
pub mod register_commands;
pub mod register_code_actions;
pub mod register_terminal_actions;
pub mod code_action_provider;

// Re-export public interfaces
pub use handle_uri::handle_uri;
pub use crate::mock_types::RegisterCommandOptions;
pub use register_commands::register_commands;
pub use register_code_actions::register_code_actions;
pub use register_terminal_actions::register_terminal_actions;
pub use crate::code_action_provider::CodeActionProvider;
