// AWS Configuration Validator with Actionable Error Messages
use crate::error::{Error, Result};
use std::env;
use tracing::{info, warn, error};

/// AWS configuration requirements
#[derive(Debug, Clone)]
pub struct AwsConfigRequirements {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    pub session_token: Option<String>,
    pub model_id: String,
    pub max_batch_size: usize,
    pub requests_per_second: f64,
}

impl Default for AwsConfigRequirements {
    fn default() -> Self {
        Self {
            access_key_id: String::new(),
            secret_access_key: String::new(),
            region: "us-east-1".to_string(),
            session_token: None,
            model_id: "amazon.titan-embed-text-v1".to_string(),
            max_batch_size: 25,
            requests_per_second: 10.0,
        }
    }
}

/// Validate AWS configuration with detailed error messages
pub fn validate_aws_config() -> Result<AwsConfigRequirements> {
    let mut config = AwsConfigRequirements::default();
    let mut errors = Vec::new();
    
    // Check AWS credentials
    match env::var("AWS_ACCESS_KEY_ID") {
        Ok(key) if !key.is_empty() => {
            config.access_key_id = key;
            info!("AWS_ACCESS_KEY_ID found");
        }
        _ => {
            errors.push("AWS_ACCESS_KEY_ID not set or empty");
        }
    }
    
    match env::var("AWS_SECRET_ACCESS_KEY") {
        Ok(key) if !key.is_empty() => {
            config.secret_access_key = key;
            info!("AWS_SECRET_ACCESS_KEY found");
        }
        _ => {
            errors.push("AWS_SECRET_ACCESS_KEY not set or empty");
        }
    }
    
    // Check optional session token
    if let Ok(token) = env::var("AWS_SESSION_TOKEN") {
        if !token.is_empty() {
            config.session_token = Some(token);
            info!("AWS_SESSION_TOKEN found");
        }
    }
    
    // Check region
    match env::var("AWS_REGION") {
        Ok(region) if !region.is_empty() => {
            config.region = region.clone();
            info!("AWS_REGION set to: {}", region);
        }
        _ => {
            match env::var("AWS_DEFAULT_REGION") {
                Ok(region) if !region.is_empty() => {
                    config.region = region.clone();
                    info!("AWS_DEFAULT_REGION set to: {}", region);
                }
                _ => {
                    warn!("No AWS region specified, using default: us-east-1");
                }
            }
        }
    }
    
    // Check model configuration
    if let Ok(model) = env::var("TITAN_MODEL_ID") {
        if !model.is_empty() {
            config.model_id = model.clone();
            info!("TITAN_MODEL_ID set to: {}", model);
        }
    }
    
    if let Ok(batch_size) = env::var("TITAN_MAX_BATCH_SIZE") {
        if let Ok(size) = batch_size.parse::<usize>() {
            config.max_batch_size = size;
            info!("TITAN_MAX_BATCH_SIZE set to: {}", size);
        }
    }
    
    if let Ok(rps) = env::var("TITAN_REQUESTS_PER_SECOND") {
        if let Ok(rate) = rps.parse::<f64>() {
            config.requests_per_second = rate;
            info!("TITAN_REQUESTS_PER_SECOND set to: {}", rate);
        }
    }
    
    // If there are errors, provide actionable guidance
    if !errors.is_empty() {
        error!("AWS configuration validation failed");
        
        let mut message = String::from("AWS configuration is incomplete. Please set the following environment variables:\n\n");
        
        for error in &errors {
            message.push_str(&format!("  ❌ {}\n", error));
        }
        
        message.push_str("\n📝 How to fix:\n\n");
        message.push_str("1. Export environment variables:\n");
        message.push_str("   export AWS_ACCESS_KEY_ID=your_access_key\n");
        message.push_str("   export AWS_SECRET_ACCESS_KEY=your_secret_key\n");
        message.push_str("   export AWS_REGION=us-east-1\n\n");
        
        message.push_str("2. Or create a .env file in the project root:\n");
        message.push_str("   AWS_ACCESS_KEY_ID=your_access_key\n");
        message.push_str("   AWS_SECRET_ACCESS_KEY=your_secret_key\n");
        message.push_str("   AWS_REGION=us-east-1\n\n");
        
        message.push_str("3. Or configure AWS CLI:\n");
        message.push_str("   aws configure\n\n");
        
        message.push_str("📚 Documentation:\n");
        message.push_str("   https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-envvars.html\n");
        message.push_str("   https://docs.aws.amazon.com/bedrock/latest/userguide/titan-embedding-models.html\n");
        
        return Err(Error::Runtime { message });
    }
    
    // Validate region supports Bedrock
    let supported_regions = vec![
        "us-east-1", "us-west-2", "eu-west-1", "eu-central-1", 
        "ap-southeast-1", "ap-northeast-1"
    ];
    
    if !supported_regions.contains(&config.region.as_str()) {
        warn!(
            "Region '{}' may not support AWS Bedrock. Supported regions: {:?}",
            config.region, supported_regions
        );
    }
    
    info!("✅ AWS configuration validated successfully");
    Ok(config)
}

/// Check if AWS credentials are available (for test skipping)
pub fn has_aws_credentials() -> bool {
    env::var("AWS_ACCESS_KEY_ID").is_ok() && 
    env::var("AWS_SECRET_ACCESS_KEY").is_ok()
}

/// Load configuration from .env file if it exists
pub fn load_env_file() {
    if let Ok(contents) = std::fs::read_to_string(".env") {
        for line in contents.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                
                if env::var(key).is_err() {
                    env::set_var(key, value);
                }
            }
        }
        info!("Loaded configuration from .env file");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_has_credentials_detection() {
        // This test will pass or fail based on environment
        let has_creds = has_aws_credentials();
        if has_creds {
            println!("AWS credentials detected");
        } else {
            println!("No AWS credentials found");
        }
    }
    
    #[test]
    fn test_validation_with_missing_credentials() {
        // Temporarily clear credentials
        let old_key = env::var("AWS_ACCESS_KEY_ID").ok();
        let old_secret = env::var("AWS_SECRET_ACCESS_KEY").ok();
        
        env::remove_var("AWS_ACCESS_KEY_ID");
        env::remove_var("AWS_SECRET_ACCESS_KEY");
        
        let result = validate_aws_config();
        assert!(result.is_err());
        
        if let Err(Error::Runtime { message }) = result {
            assert!(message.contains("AWS configuration is incomplete"));
            assert!(message.contains("How to fix"));
        }
        
        // Restore credentials if they existed
        if let Some(key) = old_key {
            env::set_var("AWS_ACCESS_KEY_ID", key);
        }
        if let Some(secret) = old_secret {
            env::set_var("AWS_SECRET_ACCESS_KEY", secret);
        }
    }
}
