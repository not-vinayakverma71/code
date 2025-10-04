// EXACT 1:1 TRANSLATION FROM TYPESCRIPT MCP Marketplace
// Critical: NO CHANGES to message formats - must match TypeScript exactly

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use reqwest;

// EXACT TypeScript McpMarketplaceItem interface
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpMarketplaceItem {
    pub mcp_id: String,
    pub github_url: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub github_stars: u32,
    pub download_count: u32,
    pub created_at: String,
    pub updated_at: String,
    pub version: String,
    pub min_engine_version: String,
    pub max_engine_version: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub last_github_sync: String,
}

// EXACT TypeScript McpMarketplaceCatalog interface
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpMarketplaceCatalog {
    pub items: Vec<McpMarketplaceItem>,
}

// EXACT TypeScript McpDownloadResponse interface
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpDownloadResponse {
    pub mcp_id: String,
    pub github_url: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub version: String,
    pub download_url: String,
    pub requires_api_key: bool,
}

// EXACT TypeScript McpState interface
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpState {
    pub mcp_marketplace_catalog: Option<McpMarketplaceCatalog>,
}

// EXACT TypeScript McpMode type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpMode {
    Full,
    #[serde(rename = "server-use-only")]
    ServerUseOnly,
    Off,
}

pub const DEFAULT_MCP_TIMEOUT_SECONDS: u64 = 60;
pub const MIN_MCP_TIMEOUT_SECONDS: u64 = 1;

/// MCP Marketplace client - EXACT translation from TypeScript
pub struct McpMarketplace {
    client: reqwest::Client,
    base_url: String,
    cached_catalog: Option<McpMarketplaceCatalog>,
}

impl McpMarketplace {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.cline.bot/v1/mcp".to_string(),
            cached_catalog: None,
        }
    }

    /// Fetch MCP marketplace catalog from API - EXACT match to TypeScript
    pub async fn fetch_marketplace_from_api(&mut self, silent: bool) -> Result<McpMarketplaceCatalog> {
        let url = format!("{}/marketplace", self.base_url);
        
        let response = self.client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let error = format!("Failed to fetch MCP marketplace: {}", response.status());
            if !silent {
                return Err(anyhow::anyhow!(error));
            }
            eprintln!("Failed to fetch MCP marketplace: {}", error);
            return Ok(McpMarketplaceCatalog { items: vec![] });
        }

        let data: Vec<serde_json::Value> = response.json().await?;
        
        // Map items exactly as TypeScript does
        let items: Vec<McpMarketplaceItem> = data.iter()
            .filter_map(|item| {
                serde_json::from_value::<McpMarketplaceItem>(item.clone()).ok()
            })
            .map(|mut item| {
                // Ensure defaults match TypeScript
                if item.github_stars == 0 {
                    item.github_stars = 0;
                }
                if item.download_count == 0 {
                    item.download_count = 0;
                }
                if item.tags.is_empty() {
                    item.tags = vec![];
                }
                item
            })
            .collect();

        let catalog = McpMarketplaceCatalog { items };
        self.cached_catalog = Some(catalog.clone());
        
        Ok(catalog)
    }

    /// Silently refresh MCP marketplace
    pub async fn silently_refresh_marketplace(&mut self) -> Option<McpMarketplaceCatalog> {
        match self.fetch_marketplace_from_api(true).await {
            Ok(catalog) => Some(catalog),
            Err(e) => {
                eprintln!("Failed to silently refresh MCP marketplace: {}", e);
                None
            }
        }
    }

    /// Fetch MCP marketplace with caching support
    pub async fn fetch_marketplace(&mut self, force_refresh: bool) -> Result<McpMarketplaceCatalog> {
        if !force_refresh {
            if let Some(ref catalog) = self.cached_catalog {
                if !catalog.items.is_empty() {
                    return Ok(catalog.clone());
                }
            }
        }

        self.fetch_marketplace_from_api(false).await
    }

    /// Download MCP server details
    pub async fn download_mcp(&self, mcp_id: &str) -> Result<McpDownloadResponse> {
        let url = format!("{}/download/{}", self.base_url, mcp_id);
        
        let response = self.client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download MCP server details: {}",
                response.status()
            ));
        }

        let download_details: McpDownloadResponse = response.json().await?;
        Ok(download_details)
    }

    /// Fetch latest MCP servers from hub
    pub async fn fetch_latest_servers_from_hub(&self) -> Result<Vec<McpMarketplaceItem>> {
        let url = format!("{}/servers/latest", self.base_url);
        
        let response = self.client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch latest servers: {}",
                response.status()
            ));
        }

        let servers: Vec<McpMarketplaceItem> = response.json().await?;
        Ok(servers)
    }

    /// Search marketplace
    pub async fn search_marketplace(&self, query: &str) -> Result<Vec<McpMarketplaceItem>> {
        let url = format!("{}/marketplace/search", self.base_url);
        
        let mut params = HashMap::new();
        params.insert("q", query);
        
        let response = self.client
            .get(&url)
            .query(&params)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Search failed: {}",
                response.status()
            ));
        }

        let results: Vec<McpMarketplaceItem> = response.json().await?;
        Ok(results)
    }

    /// Get MCP server display name
    pub fn get_mcp_server_display_name(
        server_name: &str,
        catalog: &McpMarketplaceCatalog,
    ) -> String {
        catalog.items
            .iter()
            .find(|item| item.mcp_id == server_name)
            .map(|item| item.name.clone())
            .unwrap_or_else(|| server_name.to_string())
    }
}

/// MCP UI Messages - EXACT translation from TypeScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum McpWebviewMessage {
    #[serde(rename = "fetchMcpMarketplace")]
    FetchMcpMarketplace {
        #[serde(rename = "bool")]
        force_refresh: bool,
    },
    
    #[serde(rename = "silentlyRefreshMcpMarketplace")]
    SilentlyRefreshMcpMarketplace,
    
    #[serde(rename = "fetchLatestMcpServersFromHub")]
    FetchLatestMcpServersFromHub,
    
    #[serde(rename = "downloadMcp")]
    DownloadMcp {
        #[serde(rename = "mcpId")]
        mcp_id: String,
    },
    
    #[serde(rename = "mcpEnabled")]
    McpEnabled {
        enabled: bool,
    },
    
    #[serde(rename = "mcpMarketplaceCatalog")]
    McpMarketplaceCatalog {
        #[serde(skip_serializing_if = "Option::is_none")]
        mcp_marketplace_catalog: Option<McpMarketplaceCatalog>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    
    #[serde(rename = "mcpDownloadDetails")]
    McpDownloadDetails {
        #[serde(skip_serializing_if = "Option::is_none")]
        mcp_download_details: Option<McpDownloadResponse>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}

/// MCP Extension Messages - EXACT translation from TypeScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum McpExtensionMessage {
    #[serde(rename = "askUseMcpServer")]
    AskUseMcpServer {
        server_name: String,
        #[serde(rename = "use_type")]
        use_type: McpUseType,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        resource_uri: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpUseType {
    UseMcpTool,
    AccessMcpResource,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_mode_serialization() {
        // Verify exact JSON format matches TypeScript
        let full = serde_json::to_string(&McpMode::Full).unwrap();
        assert_eq!(full, r#""full""#);
        
        let server_only = serde_json::to_string(&McpMode::ServerUseOnly).unwrap();
        assert_eq!(server_only, r#""server-use-only""#);
        
        let off = serde_json::to_string(&McpMode::Off).unwrap();
        assert_eq!(off, r#""off""#);
    }

    #[test]
    fn test_message_format_exact_match() {
        // Verify message formats match TypeScript exactly
        let msg = McpWebviewMessage::FetchMcpMarketplace { 
            force_refresh: true 
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"fetchMcpMarketplace""#));
        assert!(json.contains(r#""bool":true"#));
    }
}
