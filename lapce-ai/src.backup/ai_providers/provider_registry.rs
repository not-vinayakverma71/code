/// Provider Registry - Core infrastructure for managing all providers
/// EXACT implementation as specified in requirements

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;

use crate::ai_providers::core_trait::AiProvider;
use crate::ai_providers::{
    openai_exact::{OpenAiHandler, OpenAiHandlerOptions},
    anthropic_exact::{AnthropicProvider, AnthropicConfig},
    gemini_exact::{GeminiProvider, GeminiConfig},
    bedrock_exact::{BedrockProvider, BedrockConfig},
    azure_exact::{AzureOpenAiProvider, AzureOpenAiConfig},
    xai_exact::{XaiProvider, XaiConfig},
    vertex_ai_exact::{VertexAiProvider, VertexAiConfig},
    openrouter_exact::{OpenRouterProvider, OpenRouterConfig},
};

/// Provider configuration
#[derive(Debug, Clone)]
pub struct ProviderInitConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub region: Option<String>,
    pub project_id: Option<String>,
    pub location: Option<String>,
    pub deployment_name: Option<String>,
    pub api_version: Option<String>,
}

/// Provider Registry - manages all AI provider instances
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn AiProvider + Send + Sync + 'static>>,
}

impl ProviderRegistry {
    /// Create new registry and register all providers
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }
    
    /// Register a provider instance
    pub fn register(&mut self, name: String, provider: Arc<dyn AiProvider + Send + Sync + 'static>) {
        self.providers.insert(name, provider);
    }
    
    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn AiProvider + Send + Sync + 'static>> {
        self.providers.get(name).cloned()
    }
    
    /// List all registered providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
    
    /// Create and register a provider from configuration
    pub async fn create_and_register(&mut self, config: ProviderInitConfig) -> Result<()> {
        let provider = Self::create_provider(config.clone()).await?;
        let name = config.provider_type.clone();
        self.register(name, provider);
        Ok(())
    }
    
    /// Create a provider instance from configuration
    pub async fn create_provider(config: ProviderInitConfig) -> Result<Arc<dyn AiProvider + Send + Sync + 'static>> {
        match config.provider_type.as_str() {
            "openai" => {
                let options = OpenAiHandlerOptions {
                    openai_api_key: config.api_key.unwrap_or_default(),
                    openai_base_url: config.base_url,
                    openai_model_id: None,
                    openai_headers: None,
                    openai_use_azure: false,
                    azure_api_version: None,
                    openai_r1_format_enabled: false,
                    openai_legacy_format: false,
                    timeout_ms: Some(30000),
                };
                let handler = OpenAiHandler::new(options).await?;
                Ok(Arc::new(handler) as Arc<dyn AiProvider + Send + Sync + 'static>)
            }
            
            "anthropic" => {
                let anthropic_config = AnthropicConfig {
                    api_key: config.api_key.unwrap_or_default(),
                    base_url: config.base_url,
                    version: "2023-06-01".to_string(),
                    beta_features: vec!["prompt-caching-2024-07-31".to_string()],
                    default_model: Some("claude-3-opus-20240229".to_string()),
                    cache_enabled: true,
                    timeout_ms: Some(30000),
                };
                Ok(Arc::new(AnthropicProvider::new(anthropic_config).await?) as Arc<dyn AiProvider + Send + Sync + 'static>)
            }
            
            "gemini" => {
                let gemini_config = GeminiConfig {
                    api_key: config.api_key.unwrap_or_default(),
                    base_url: config.base_url,
                    default_model: Some("gemini-pro".to_string()),
                    api_version: config.api_version.or(Some("v1beta".to_string())),
                    timeout_ms: Some(30000),
                    project_id: config.project_id,
                    location: config.location,
                };
                Ok(Arc::new(GeminiProvider::new(gemini_config).await?) as Arc<dyn AiProvider + Send + Sync + 'static>)
            }
            
            "bedrock" => {
                let bedrock_config = BedrockConfig {
                    region: config.region.unwrap_or_else(|| "us-east-1".to_string()),
                    access_key_id: config.api_key.clone().unwrap_or_default(),
                    secret_access_key: config.api_key.unwrap_or_default(),
                    session_token: None,
                    base_url: config.base_url,
                    default_model: Some("anthropic.claude-3-opus-20240229-v1:0".to_string()),
                    timeout_ms: Some(30000),
                };
                Ok(Arc::new(BedrockProvider::new(bedrock_config).await?) as Arc<dyn AiProvider + Send + Sync + 'static>)
            }
            
            "azure" => {
                let azure_config = AzureOpenAiConfig {
                    endpoint: config.base_url.unwrap_or_default(),
                    api_key: config.api_key.unwrap_or_default(),
                    deployment_name: config.deployment_name.unwrap_or_default(),
                    api_version: config.api_version.unwrap_or_else(|| "2024-02-01".to_string()),
                    use_entra_id: false,
                    timeout_ms: Some(30000),
                };
                Ok(Arc::new(AzureOpenAiProvider::new(azure_config).await?) as Arc<dyn AiProvider + Send + Sync + 'static>)
            }
            
            "xai" => {
                let xai_config = XaiConfig {
                    api_key: config.api_key.unwrap_or_default(),
                    base_url: config.base_url.or(Some("https://api.x.ai/v1".to_string())),
                };
                Ok(Arc::new(XaiProvider::new(xai_config).await?) as Arc<dyn AiProvider + Send + Sync + 'static>)
            }
            
            "vertex-ai" => {
                let vertex_config = VertexAiConfig {
                    project_id: config.project_id.unwrap_or_default(),
                    location: config.location.unwrap_or_else(|| "us-central1".to_string()),
                    access_token: config.api_key.unwrap_or_default(),
                    default_model: Some("gemini-pro".to_string()),
                    timeout_ms: Some(30000),
                };
                Ok(Arc::new(VertexAiProvider::new(vertex_config).await?) as Arc<dyn AiProvider + Send + Sync + 'static>)
            }
            
            "openrouter" => {
                let openrouter_config = OpenRouterConfig {
                    api_key: config.api_key.unwrap_or_default(),
                    base_url: config.base_url.unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string()),
                    default_model: config.deployment_name.unwrap_or_else(|| "openrouter/auto".to_string()),
                    specific_provider: None,
                    allow_fallbacks: true,
                    data_collection: None,
                    provider_sort: None,
                    use_middle_out_transform: true,
                    timeout_ms: 30000,
                    referer: std::env::var("OPENROUTER_HTTP_REFERER").unwrap_or_else(|_| "https://lapce.dev".to_string()),
                    app_title: std::env::var("OPENROUTER_APP_TITLE").unwrap_or_else(|_| "Lapce IDE".to_string()),
                };
                Ok(Arc::new(OpenRouterProvider::new(openrouter_config).await?) as Arc<dyn AiProvider + Send + Sync + 'static>)
            }
            
            _ => {
                anyhow::bail!("Unknown provider type: {}", config.provider_type)
            }
        }
    }
    
    /// Initialize with default set of providers
    pub async fn initialize_defaults(&mut self) -> Result<()> {
        // This would be called with actual API keys from environment or config
        // For now, just returns empty registry
        Ok(())
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_registry_creation() {
        let registry = ProviderRegistry::new();
        assert_eq!(registry.list_providers().len(), 0);
    }
    
    #[tokio::test]
    async fn test_provider_registration() {
        let mut registry = ProviderRegistry::new();
        
        // Create a test config
        let config = ProviderInitConfig {
            provider_type: "openai".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: None,
            region: None,
            project_id: None,
            location: None,
            deployment_name: None,
            api_version: None,
        };
        
        // Register provider
        registry.create_and_register(config).await.unwrap();
        
        // Verify it's registered
        assert_eq!(registry.list_providers().len(), 1);
        assert!(registry.get("openai").is_some());
    }
}
