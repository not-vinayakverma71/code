/// Mode Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/mode.ts
use serde::{Deserialize, Serialize};
use crate::types_tool::ToolGroup;

/// GroupOptions - Direct translation from TypeScript
/// Lines 9-31 from mode.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_regex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// GroupEntry - Direct translation from TypeScript
/// Lines 37-39 from mode.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GroupEntry {
    Simple(ToolGroup),
    WithOptions(ToolGroup, GroupOptions),
}

/// ModeConfig - Direct translation from TypeScript
/// Lines 64-76 from mode.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeConfig {
    pub slug: String,
    pub name: String,
    pub role_definition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
    pub groups: Vec<GroupEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ModeSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_name: Option<String>, // kilocode_change
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModeSource {
    Global,
    Project,
}

/// CustomModesSettings - Direct translation from TypeScript
/// Lines 82-100 from mode.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModesSettings {
    pub custom_modes: Vec<ModeConfig>,
}

/// Validation functions matching TypeScript
impl GroupOptions {
    pub fn validate_regex(&self) -> bool {
        if let Some(pattern) = &self.file_regex {
            regex::Regex::new(pattern).is_ok()
        } else {
            true // Optional, so empty is valid
        }
    }
}

impl ModeConfig {
    pub fn validate_slug(&self) -> bool {
        self.slug.chars().all(|c| c.is_alphanumeric() || c == '-')
    }
}

impl CustomModesSettings {
    pub fn validate_unique_slugs(&self) -> bool {
        let mut slugs = std::collections::HashSet::new();
        self.custom_modes.iter().all(|mode| slugs.insert(&mode.slug))
    }
}
