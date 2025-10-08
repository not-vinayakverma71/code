// Environment configuration for AWS Titan embeddings
use std::env;
use serde::{Deserialize, Serialize};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitanConfig {
    pub aws_region: String,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub model: String,
    pub dimensions: usize,
    pub max_concurrent_requests: usize,
    pub requests_per_second: usize,
}

impl TitanConfig {
    /// Load configuration from environment variables
    /// Precedence: Environment variables > .env file
    pub fn from_env() -> Result<Self> {
        // Load .env file if exists (won't override existing env vars)
        let _ = dotenv::dotenv();
        
        // Required fields
        let aws_region = env::var("AWS_REGION")
            .unwrap_or_else(|_| "us-east-1".to_string());
        
        let model = env::var("TITAN_MODEL")
            .unwrap_or_else(|_| "amazon.titan-embed-text-v2:0".to_string());
        
        let dimensions = env::var("TITAN_DIMENSIONS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1024);
        
        // Optional AWS credentials (may use IAM role or AWS CLI config)
        let aws_access_key_id = env::var("AWS_ACCESS_KEY_ID").ok();
        let aws_secret_access_key = env::var("AWS_SECRET_ACCESS_KEY").ok();
        
        // Rate limiting
        let max_concurrent_requests = env::var("TITAN_MAX_CONCURRENT_REQUESTS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10);
        
        let requests_per_second = env::var("TITAN_REQUESTS_PER_SECOND")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(5);
        
        // Log configuration
        log::info!("Titan config loaded:");
        log::info!("  Model: {}", model);
        log::info!("  Dimensions: {}", dimensions);
        log::info!("  Region: {}", aws_region);
        log::info!("  Max concurrent: {}", max_concurrent_requests);
        log::info!("  RPS limit: {}", requests_per_second);
        
        Ok(Self {
            aws_region,
            aws_access_key_id,
            aws_secret_access_key,
            model,
            dimensions,
            max_concurrent_requests,
            requests_per_second,
        })
    }
    
    /// Validate dimension alignment with model
    pub fn validate_dimensions(&self) -> Result<()> {
        let expected_dim = match self.model.as_str() {
            "amazon.titan-embed-text-v1" => 1536,
            "amazon.titan-embed-text-v2:0" => 1024,
            _ => {
                log::warn!("Unknown Titan model: {}, assuming {} dimensions", 
                    self.model, self.dimensions);
                self.dimensions
            }
        };
        
        if self.dimensions != expected_dim {
            return Err(Error::Runtime {
                message: format!(
                    "Dimension mismatch: config specifies {} but model '{}' expects {}",
                    self.dimensions, self.model, expected_dim
                )
            });
        }
        
        Ok(())
    }
}
