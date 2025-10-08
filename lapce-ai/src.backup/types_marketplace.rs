/// Marketplace Types - EXACT 1:1 Translation from TypeScript
/// Source: Codex/packages/types/src/marketplace.ts
use serde::{Deserialize, Serialize};

/// McpParameter - Direct translation from TypeScript
/// Lines 6-13 from marketplace.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpParameter {
    pub name: String,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

/// McpInstallationMethod - Direct translation from TypeScript
/// Lines 18-25 from marketplace.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInstallationMethod {
    pub name: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<McpParameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisites: Option<Vec<String>>,
}

/// MarketplaceItemType - Direct translation from TypeScript
/// Lines 30-32 from marketplace.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketplaceItemType {
    Mode,
    Mcp,
}

/// Base marketplace item fields - Lines 37-45
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseMarketplaceItem {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisites: Option<Vec<String>>,
}

/// ModeMarketplaceItem - Lines 50-54
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeMarketplaceItem {
    #[serde(flatten)]
    pub base: BaseMarketplaceItem,
    pub content: String, // YAML content for modes
}

/// McpContent - for the union type in content field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpContent {
    Single(String),
    Methods(Vec<McpInstallationMethod>),
}

/// McpMarketplaceItem - Lines 56-62
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpMarketplaceItem {
    #[serde(flatten)]
    pub base: BaseMarketplaceItem,
    pub url: String, // Required url field
    pub content: McpContent, // Single config or array of methods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<McpParameter>>,
}

/// MarketplaceItem - Discriminated union Lines 67-78
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum MarketplaceItem {
    Mode {
        #[serde(flatten)]
        item: ModeMarketplaceItem,
    },
    Mcp {
        #[serde(flatten)]
        item: McpMarketplaceItem,
    },
}
