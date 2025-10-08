/// Provider Settings Types
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettingsEntry {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl Default for ProviderSettingsEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            enabled: false,
            api_key: None,
            base_url: None,
            model: None,
            max_tokens: None,
            temperature: None,
        }
    }
}
