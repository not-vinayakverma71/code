//! Prompt Builder
//!
//! 1:1 Translation from Codex `src/core/prompts/system.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/system.ts (lines 1-246)

use std::path::PathBuf;

use crate::core::prompt::{
    errors::{PromptError, PromptResult},
    modes::ModeConfig,
    settings::SystemPromptSettings,
    sections::{custom_system_prompt, custom_instructions},
    tokenizer::count_tokens,
};

pub use custom_system_prompt::PromptVariables;

/// Maximum prompt size in characters (approximately 100K tokens)
const MAX_PROMPT_SIZE: usize = 400_000;

/// Prompt Builder - orchestrates system prompt generation
///
/// Translation of SYSTEM_PROMPT() and generatePrompt() from system.ts
pub struct PromptBuilder {
    workspace: PathBuf,
    mode: ModeConfig,
    settings: SystemPromptSettings,
    custom_instructions: Option<String>,
}

impl PromptBuilder {
    pub fn new(
        workspace: PathBuf,
        mode: ModeConfig,
        settings: SystemPromptSettings,
        custom_instructions: Option<String>,
    ) -> Self {
        Self {
            workspace,
            mode,
            settings,
            custom_instructions,
        }
    }
    
    /// Build complete system prompt
    ///
    /// Translation of SYSTEM_PROMPT() from system.ts (lines 147-244)
    pub async fn build(&self) -> PromptResult<String> {
        // Create variables for interpolation
        let variables = PromptVariables::from_workspace(&self.workspace, &self.mode.slug);
        
        // Step 1: Try to load custom system prompt file
        let file_custom_prompt = custom_system_prompt::load_system_prompt_file(
            &self.workspace,
            &self.mode.slug,
            &variables,
        ).await?;
        
        // If file-based custom prompt exists, use it with custom instructions only
        if !file_custom_prompt.is_empty() {
            let (role_definition, base_instructions) = crate::core::prompt::modes::get_mode_selection(&self.mode);
            
            let custom_instr = custom_instructions::add_custom_instructions(
                &base_instructions,
                self.custom_instructions.as_deref().unwrap_or(""),
                &self.workspace,
                &self.mode.slug,
                Some("en"), // TODO: Make configurable
                None, // TODO: Add rooignore instructions
                &self.settings,
            ).await?;
            
            return Ok(format!(
                "{}\n\n{}\n\n{}",
                role_definition,
                file_custom_prompt,
                custom_instr
            ));
        }
        
        // Step 2: Generate structured prompt
        self.generate_prompt().await
    }
    
    /// Generate structured prompt with all sections
    ///
    /// Translation of generatePrompt() from system.ts (lines 52-145)
    async fn generate_prompt(&self) -> PromptResult<String> {
        let start = std::time::Instant::now();
        use crate::core::prompt::sections::*;
        use crate::core::prompt::tools::{get_tool_descriptions_for_mode, ToolDescriptionContext};
        
        let (role_definition, base_instructions) = crate::core::prompt::modes::get_mode_selection(&self.mode);
        
        // Feature gates (all disabled pre-IPC per memories)
        let codebase_search_available = false; // TODO: Wire after IPC
        let supports_browser = false; // TODO: Wire after IPC
        let has_mcp = false; // TODO: Wire after IPC
        let diff_strategy = None; // TODO: Wire after IPC
        let fast_apply_available = false; // TODO: Wire after IPC (Morph)
        
        // Generate tool descriptions for this mode
        let tool_context = ToolDescriptionContext {
            workspace: &self.workspace,
            supports_browser,
            codebase_search_available,
            fast_apply_available,
            max_concurrent_file_reads: self.settings.max_concurrent_file_reads as usize,
            partial_reads_enabled: true,
            todo_list_enabled: self.settings.todo_list_enabled,
            image_generation_enabled: false, // TODO: Wire after IPC
            run_slash_command_enabled: false, // TODO: Wire after IPC
            browser_viewport_size: self.settings.browser_viewport_size.clone().unwrap_or_else(|| "900x600".to_string()),
            new_task_require_todos: self.settings.new_task_require_todos,
        };
        let tool_descriptions = get_tool_descriptions_for_mode(&self.mode, &tool_context);
        
        // Assemble all sections in order (matching Codex system.ts)
        let mut sections = vec![
            role_definition,
            markdown_formatting_section(),
            shared_tool_use_section(),
            tool_descriptions,
            tool_use_guidelines_section(codebase_search_available),
            // TODO: mcp_servers_section() [if enabled] - later
            capabilities_section(
                &self.workspace,
                supports_browser,
                has_mcp,
                diff_strategy,
                codebase_search_available,
                fast_apply_available,
            ),
            // TODO: modes_section() - requires async mode loading
            // TODO: rules_section() - P7 remaining
            system_info_section(&self.workspace),
            objective_section(codebase_search_available),
        ];
        
        // Add custom instructions last
        let custom_instr = custom_instructions::add_custom_instructions(
            &base_instructions,
            self.custom_instructions.as_deref().unwrap_or(""),
            &self.workspace,
            &self.mode.slug,
            Some("en"),
            None,
            &self.settings,
        ).await?;
        
        if !custom_instr.is_empty() {
            sections.push(custom_instr);
        }
        
        let prompt = sections
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        
        // Observability: log prompt build metrics
        let duration = start.elapsed();
        let char_count = prompt.len();
        let token_estimate = char_count / 4; // Approximate token count
        
        tracing::info!(
            mode = %self.mode.slug,
            duration_ms = duration.as_millis(),
            char_count = char_count,
            token_estimate = token_estimate,
            has_custom_instructions = self.custom_instructions.is_some(),
            "Prompt build completed"
        );
        
        Ok(prompt)
    }
    
    /// Build with retry and fallback strategies
    ///
    /// Translation of build_with_retry pattern from CHUNK-01 Part 5
    pub async fn build_with_retry(&self) -> PromptResult<String> {
        let start = std::time::Instant::now();
        let mut retry_count = 0;
        let mut used_fallback = false;
        
        let result = match self.build().await {
            Ok(prompt) if prompt.len() > MAX_PROMPT_SIZE => {
                tracing::warn!(
                    mode = %self.mode.slug,
                    char_count = prompt.len(),
                    max_size = MAX_PROMPT_SIZE,
                    "Prompt too large, attempting condensed build"
                );
                used_fallback = true;
                retry_count += 1;
                self.build_condensed().await
            }
            Ok(prompt) => Ok(prompt),
            Err(PromptError::RuleLoadError(e)) => {
                tracing::warn!(
                    mode = %self.mode.slug,
                    error = %e,
                    "Rules failed to load, continuing without rules"
                );
                used_fallback = true;
                retry_count += 1;
                self.build_without_rules().await
            }
            Err(e) => Err(e),
        };
        
        // Log retry metrics
        if used_fallback {
            let duration = start.elapsed();
            tracing::info!(
                mode = %self.mode.slug,
                retry_count = retry_count,
                used_fallback = used_fallback,
                total_duration_ms = duration.as_millis(),
                "Prompt build with retry completed"
            );
        }
        
        result
    }
    
    /// Build condensed version (reduce custom instructions)
    async fn build_condensed(&self) -> PromptResult<String> {
        tracing::debug!(
            mode = %self.mode.slug,
            "Building condensed prompt"
        );
        // TODO: Implement condensing strategy
        // For now, just skip custom instructions
        let (role_definition, _) = crate::core::prompt::modes::get_mode_selection(&self.mode);
        
        let sections = vec![
            role_definition,
            "# Tools\n(Condensed mode - custom instructions omitted)".to_string(),
        ];
        
        Ok(sections.join("\n\n"))
    }
    
    /// Build without rules (fallback for rule load errors)
    async fn build_without_rules(&self) -> PromptResult<String> {
        tracing::debug!(
            mode = %self.mode.slug,
            "Building prompt without rules"
        );
        // Build normally but skip rule loading
        let (role_definition, base_instructions) = crate::core::prompt::modes::get_mode_selection(&self.mode);
        
        // Add only global instructions, skip all rule files
        let custom_instr = if let Some(ref global) = self.custom_instructions {
            format!("\n====\n\nUSER'S CUSTOM INSTRUCTIONS\n\nGlobal Instructions:\n{}", global)
        } else {
            String::new()
        };
        
        Ok(format!("{}\n\n{}\n\n{}", role_definition, base_instructions, custom_instr))
    }
    
    /// Get token count for the built prompt
    pub async fn build_and_count(&self) -> PromptResult<(String, u32)> {
        let prompt = self.build().await?;
        let tokens = count_tokens(&prompt, None)?;
        Ok((prompt, tokens))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::core::prompt::modes::get_mode_by_slug;
    
    #[tokio::test]
    async fn test_builder_basic() {
        let temp_dir = TempDir::new().unwrap();
        let mode = get_mode_by_slug("code").unwrap();
        let settings = SystemPromptSettings::default();
        
        let builder = PromptBuilder::new(
            temp_dir.path().to_path_buf(),
            mode,
            settings,
            None,
        );
        
        let prompt = builder.build().await.unwrap();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Kilo Code"));
    }
    
    #[tokio::test]
    async fn test_builder_with_custom_instructions() {
        let temp_dir = TempDir::new().unwrap();
        let mode = get_mode_by_slug("code").unwrap();
        let settings = SystemPromptSettings::default();
        
        let builder = PromptBuilder::new(
            temp_dir.path().to_path_buf(),
            mode,
            settings,
            Some("Always use Rust for examples".to_string()),
        );
        
        let prompt = builder.build().await.unwrap();
        assert!(prompt.contains("Global Instructions"));
        assert!(prompt.contains("Always use Rust"));
    }
    
    #[tokio::test]
    async fn test_builder_token_count() {
        let temp_dir = TempDir::new().unwrap();
        let mode = get_mode_by_slug("architect").unwrap();
        let settings = SystemPromptSettings::default();
        
        let builder = PromptBuilder::new(
            temp_dir.path().to_path_buf(),
            mode,
            settings,
            None,
        );
        
        let (prompt, tokens) = builder.build_and_count().await.unwrap();
        assert!(!prompt.is_empty());
        assert!(tokens > 0);
    }
    
    #[tokio::test]
    async fn test_build_without_rules() {
        let temp_dir = TempDir::new().unwrap();
        let mode = get_mode_by_slug("code").unwrap();
        let settings = SystemPromptSettings::default();
        
        let builder = PromptBuilder::new(
            temp_dir.path().to_path_buf(),
            mode,
            settings,
            Some("Test instructions".to_string()),
        );
        
        let prompt = builder.build_without_rules().await.unwrap();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Test instructions"));
    }
}
