/// Kilocode Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/kilocode.ts
use serde::{Deserialize, Serialize};

/// GhostServiceSettings - Direct translation from TypeScript
/// Lines 3-14 from kilocode.ts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GhostServiceSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_auto_trigger: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_trigger_delay: Option<u32>, // min 1, max 30, default 3
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_quick_inline_task_keybinding: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_smart_inline_task_keybinding: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_custom_provider: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_config_id: Option<String>,
}

/// CommitRange - Direct translation from TypeScript
/// Lines 16-21 from kilocode.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRange {
    pub from: String,
    pub to: String,
}

/// KiloCodeMetaData - Direct translation from TypeScript
/// Lines 23-27 from kilocode.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiloCodeMetaData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_range: Option<CommitRange>,
}

impl GhostServiceSettings {
    /// Validate auto_trigger_delay is between 1 and 30
    pub fn validate_delay(&self) -> bool {
        if let Some(delay) = self.auto_trigger_delay {
            delay >= 1 && delay <= 30
        } else {
            true
        }
    }
}
