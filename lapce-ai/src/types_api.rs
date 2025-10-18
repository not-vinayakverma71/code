/// API Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/api.ts
use async_trait::async_trait;

// RooCodeEvents type removed - not needed for MCP tools
use crate::types_global_settings::RooCodeSettings;
use crate::types_provider_settings::ProviderSettingsEntry;

/// RooCodeAPI - Direct translation from TypeScript interface
/// Lines 11-145 from api.ts
#[async_trait]
pub trait RooCodeAPI: Send + Sync {
    /// Starts a new task with an optional initial message and images.
    /// Lines 18-28
    async fn start_new_task(
        &self,
        configuration: Option<RooCodeSettings>,
        text: Option<String>,
        images: Option<Vec<String>>,
        new_tab: Option<bool>,
    ) -> Result<String, Box<dyn std::error::Error>>;

    /// Resumes a task with the given ID.
    /// Lines 34
    async fn resume_task(&self, task_id: &str) -> Result<(), Box<dyn std::error::Error>>;

    /// Checks if a task with the given ID is in the task history.
    /// Lines 40
    async fn is_task_in_history(&self, task_id: &str) -> Result<bool, Box<dyn std::error::Error>>;

    /// Returns the current task stack.
    /// Lines 45
    fn get_current_task_stack(&self) -> Vec<String>;

    /// Clears the current task.
    /// Lines 49
    async fn clear_current_task(&self, last_message: Option<String>) -> Result<(), Box<dyn std::error::Error>>;

    /// Cancels the current task.
    /// Lines 53
    async fn cancel_current_task(&self) -> Result<(), Box<dyn std::error::Error>>;

    /// Sends a message to the current task.
    /// Lines 59
    async fn send_message(&self, message: Option<String>, images: Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>>;

    /// Simulates pressing the primary button in the chat interface.
    /// Lines 63
    async fn press_primary_button(&self) -> Result<(), Box<dyn std::error::Error>>;

    /// Simulates pressing the secondary button in the chat interface.
    /// Lines 67
    async fn press_secondary_button(&self) -> Result<(), Box<dyn std::error::Error>>;

    /// Returns true if the API is ready to use.
    /// Lines 71
    fn is_ready(&self) -> bool;

    /// Returns the current configuration.
    /// Lines 76
    fn get_configuration(&self) -> RooCodeSettings;

    /// Sets the configuration for the current task.
    /// Lines 81
    async fn set_configuration(&self, values: RooCodeSettings) -> Result<(), Box<dyn std::error::Error>>;

    /// Returns a list of all configured profile names
    /// Lines 86
    fn get_profiles(&self) -> Vec<String>;

    /// Returns the profile entry for a given name
    /// Lines 92
    fn get_profile_entry(&self, name: &str) -> Option<ProviderSettingsEntry>;

    /// Creates a new API configuration profile
    /// Lines 100-111
    async fn create_profile(
        &self,
        name: &str,
        profile: Option<ProviderSettingsEntry>,
        activate: Option<bool>,
    ) -> Result<String, Box<dyn std::error::Error>>;

    /// Updates an existing API configuration profile
    /// Lines 112-118
    async fn update_profile(
        &self,
        name: &str,
        profile: ProviderSettingsEntry,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Deletes an API configuration profile
    /// Lines 124
    async fn delete_profile(&self, name: &str) -> Result<(), Box<dyn std::error::Error>>;

    /// Activates an API configuration profile
    /// Lines 130
    async fn activate_profile(&self, name: &str) -> Result<(), Box<dyn std::error::Error>>;

    /// Deactivates the current API configuration profile
    /// Lines 135
    async fn deactivate_profile(&self) -> Result<(), Box<dyn std::error::Error>>;

    /// Shows the settings view
    /// Lines 140
    async fn show_settings(&self) -> Result<(), Box<dyn std::error::Error>>;

    /// Disposes the API
    /// Lines 145
    fn dispose(&self);
}
