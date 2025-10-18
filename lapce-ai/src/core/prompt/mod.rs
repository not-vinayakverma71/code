//! Prompt System - 1:1 Translation from Codex TypeScript to Rust
//!
//! This module provides the complete system prompt generation for AI interactions.
//! It translates the Codex VS Code extension's prompt system to Rust for Lapce's
//! native AI integration via shared-memory IPC.
//!
//! # Architecture
//!
//! ```text
//! SYSTEM_PROMPT()
//!   ↓
//! Check .kilocode/system-prompt-{mode}
//!   ├─ EXISTS → Use file + custom instructions
//!   └─ NOT EXISTS → generate_prompt()
//!       ↓
//! Assemble 11+ sections:
//!   1. roleDefinition (from mode)
//!   2. markdownFormattingSection
//!   3. sharedToolUseSection
//!   4. getToolDescriptionsForMode() ← dynamically filtered
//!   5. toolUseGuidelinesSection
//!   6. mcpServersSection [if enabled]
//!   7. capabilitiesSection
//!   8. modesSection
//!   9. rulesSection
//!   10. systemInfoSection
//!   11. objectiveSection
//!   12. addCustomInstructions() ← 5 layers
//! ```
//!
//! # Reference
//!
//! Codex source: `/home/verma/lapce/Codex/src/core/prompts/system.ts`
//! Documentation: `lapce-ai/docs/CHUNK-01-PROMPTS-SYSTEM.md`

pub mod builder;
pub mod errors;
pub mod modes;
pub mod settings;
pub mod tokenizer;

// Subdirectories matching Codex structure
pub mod instructions;
pub mod sections;
pub mod tools;

#[cfg(test)]
mod tests;

// Re-exports for convenience
pub use builder::{PromptBuilder, PromptVariables};
pub use errors::PromptError;
pub use modes::{Mode, ModeConfig, get_mode_by_slug, get_mode_selection};
pub use settings::SystemPromptSettings;
pub use tokenizer::count_tokens;

/// Main entry point for generating system prompts.
///
/// This is the Rust equivalent of `SYSTEM_PROMPT()` in system.ts.
///
/// # Arguments
///
/// * `workspace_path` - Current working directory (workspace root)
/// * `mode` - AI mode slug (code, architect, ask, debug, orchestrator)
/// * `settings` - System prompt settings
/// * `custom_instructions` - User's global custom instructions
///
/// # Returns
///
/// Complete system prompt as a String (15K-25K tokens typically)
pub async fn system_prompt(
    workspace_path: std::path::PathBuf,
    mode: &str,
    settings: &SystemPromptSettings,
    custom_instructions: Option<String>,
) -> Result<String, PromptError> {
    let mode_config = get_mode_by_slug(mode)?;
    
    let builder = PromptBuilder::new(
        workspace_path,
        mode_config,
        settings.clone(),
        custom_instructions,
    );
    
    builder.build().await
}
