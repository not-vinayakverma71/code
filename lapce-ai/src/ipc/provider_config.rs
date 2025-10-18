// Provider configuration from environment variables
// Maps environment variables to ProviderInitConfig for AI provider initialization

use crate::ai_providers::provider_registry::ProviderInitConfig;
use anyhow::{Result, bail};
use std::env;

/// Load provider configuration from environment variables
/// Returns a map of provider name -> ProviderInitConfig
pub fn load_provider_configs_from_env() -> Result<Vec<(String, ProviderInitConfig)>> {
    let mut configs = Vec::new();
    
    // OpenAI
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() {
            configs.push((
                "openai".to_string(),
                ProviderInitConfig {
                    provider_type: "openai".to_string(),
                    api_key: Some(api_key),
                    base_url: env::var("OPENAI_BASE_URL").ok(),
                    region: None,
                    project_id: None,
                    location: None,
                    deployment_name: None,
                    api_version: None,
                },
            ));
        }
    }
    
    // Anthropic
    if let Ok(api_key) = env::var("ANTHROPIC_API_KEY") {
        if !api_key.is_empty() {
            configs.push((
                "anthropic".to_string(),
                ProviderInitConfig {
                    provider_type: "anthropic".to_string(),
                    api_key: Some(api_key),
                    base_url: env::var("ANTHROPIC_BASE_URL").ok(),
                    region: None,
                    project_id: None,
                    location: None,
                    deployment_name: None,
                    api_version: None,
                },
            ));
        }
    }
    
    // Gemini
    if let Ok(api_key) = env::var("GEMINI_API_KEY") {
        if !api_key.is_empty() {
            configs.push((
                "gemini".to_string(),
                ProviderInitConfig {
                    provider_type: "gemini".to_string(),
                    api_key: Some(api_key),
                    base_url: env::var("GEMINI_BASE_URL").ok(),
                    region: None,
                    project_id: None,
                    location: None,
                    deployment_name: None,
                    api_version: None,
                },
            ));
        }
    }
    
    // Azure OpenAI
    if let Ok(api_key) = env::var("AZURE_OPENAI_API_KEY") {
        if !api_key.is_empty() {
            let deployment_name = env::var("AZURE_OPENAI_DEPLOYMENT_NAME").ok();
            let api_version = env::var("AZURE_OPENAI_API_VERSION").ok();
            
            configs.push((
                "azure".to_string(),
                ProviderInitConfig {
                    provider_type: "azure".to_string(),
                    api_key: Some(api_key),
                    base_url: env::var("AZURE_OPENAI_ENDPOINT").ok(),
                    region: None,
                    project_id: None,
                    location: None,
                    deployment_name,
                    api_version,
                },
            ));
        }
    }
    
    // AWS Bedrock
    // Bedrock uses AWS credentials from environment or ~/.aws/credentials
    if env::var("AWS_ACCESS_KEY_ID").is_ok() || env::var("AWS_PROFILE").is_ok() {
        configs.push((
            "bedrock".to_string(),
            ProviderInitConfig {
                provider_type: "bedrock".to_string(),
                api_key: None, // Uses AWS credentials
                base_url: None,
                region: env::var("AWS_REGION").ok().or_else(|| Some("us-east-1".to_string())),
                project_id: None,
                location: None,
                deployment_name: None,
                api_version: None,
            },
        ));
    }
    
    // xAI (Grok)
    if let Ok(api_key) = env::var("XAI_API_KEY") {
        if !api_key.is_empty() {
            configs.push((
                "xai".to_string(),
                ProviderInitConfig {
                    provider_type: "xai".to_string(),
                    api_key: Some(api_key),
                    base_url: env::var("XAI_BASE_URL").ok(),
                    region: None,
                    project_id: None,
                    location: None,
                    deployment_name: None,
                    api_version: None,
                },
            ));
        }
    }
    
    // Vertex AI (GCP)
    if let Ok(project_id) = env::var("VERTEX_PROJECT_ID") {
        if !project_id.is_empty() {
            configs.push((
                "vertex_ai".to_string(),
                ProviderInitConfig {
                    provider_type: "vertex_ai".to_string(),
                    api_key: env::var("GOOGLE_APPLICATION_CREDENTIALS").ok(),
                    base_url: None,
                    region: None,
                    project_id: Some(project_id),
                    location: env::var("VERTEX_LOCATION").ok().or_else(|| Some("us-central1".to_string())),
                    deployment_name: None,
                    api_version: None,
                },
            ));
        }
    }
    
    // OpenRouter
    if let Ok(api_key) = env::var("OPENROUTER_API_KEY") {
        if !api_key.is_empty() {
            configs.push((
                "openrouter".to_string(),
                ProviderInitConfig {
                    provider_type: "openrouter".to_string(),
                    api_key: Some(api_key),
                    base_url: Some("https://openrouter.ai/api/v1".to_string()),
                    region: None,
                    project_id: None,
                    location: None,
                    deployment_name: None,
                    api_version: None,
                },
            ));
        }
    }
    
    Ok(configs)
}

/// Validate that at least one provider is configured
pub fn validate_provider_configs() -> Result<()> {
    let configs = load_provider_configs_from_env()?;
    
    if configs.is_empty() {
        bail!(
            "No AI providers configured. Please set at least one of: \n\
             - OPENAI_API_KEY\n\
             - ANTHROPIC_API_KEY\n\
             - GEMINI_API_KEY\n\
             - AZURE_OPENAI_API_KEY + AZURE_OPENAI_ENDPOINT\n\
             - AWS_ACCESS_KEY_ID (for Bedrock)\n\
             - XAI_API_KEY\n\
             - VERTEX_PROJECT_ID (for Vertex AI)\n\
             - OPENROUTER_API_KEY"
        );
    }
    
    eprintln!("✓ Loaded {} AI provider(s) from environment", configs.len());
    for (name, _) in &configs {
        eprintln!("  - {}", name);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_load_openai_config() {
        env::set_var("OPENAI_API_KEY", "test-key");
        
        let configs = load_provider_configs_from_env().unwrap();
        let openai = configs.iter().find(|(name, _)| name == "openai");
        
        assert!(openai.is_some());
        let (_, config) = openai.unwrap();
        assert_eq!(config.provider_type, "openai");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        
        env::remove_var("OPENAI_API_KEY");
    }
    
    #[test]
    fn test_empty_config_fails_validation() {
        // Clear all env vars
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("GEMINI_API_KEY");
        env::remove_var("AZURE_OPENAI_API_KEY");
        env::remove_var("AWS_ACCESS_KEY_ID");
        env::remove_var("AWS_PROFILE");
        env::remove_var("XAI_API_KEY");
        env::remove_var("VERTEX_PROJECT_ID");
        env::remove_var("OPENROUTER_API_KEY");
        
        let result = validate_provider_configs();
        assert!(result.is_err());
    }
}
