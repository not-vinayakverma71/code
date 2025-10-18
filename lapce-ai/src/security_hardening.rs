/// Security Hardening (Days 45-46)
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;
use anyhow::Result;
use sha2::{Sha256, Digest};

/// Rate limiter implementation
pub struct RateLimiter {
    limits: Arc<RwLock<HashMap<String, TokenBucket>>>,
    max_requests_per_second: u32,
    burst_size: u32,
}

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    rate: f64,
    capacity: f64,
}

impl RateLimiter {
    pub fn new(max_requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            max_requests_per_second,
            burst_size,
        }
    }
    
    pub async fn check_rate_limit(&self, client_id: &str) -> Result<bool> {
        let mut limits = self.limits.write().await;
        let now = Instant::now();
        
        let bucket = limits.entry(client_id.to_string()).or_insert_with(|| {
            TokenBucket {
                tokens: self.burst_size as f64,
                last_refill: now,
                rate: self.max_requests_per_second as f64,
                capacity: self.burst_size as f64,
            }
        });
        
        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * bucket.rate).min(bucket.capacity);
        bucket.last_refill = now;
        
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Input validation
pub struct InputValidator {
    max_message_size: usize,
    allowed_methods: Vec<String>,
}

impl InputValidator {
    pub fn new() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10MB
            allowed_methods: vec![
                "get".to_string(),
                "set".to_string(),
                "delete".to_string(),
                "search".to_string(),
                "embed".to_string(),
            ],
        }
    }
    
    pub fn validate_message(&self, data: &[u8]) -> Result<()> {
        if data.len() > self.max_message_size {
            return Err(anyhow::anyhow!("Message too large"));
        }
        
        // Check for null bytes
        if data.contains(&0) {
            return Err(anyhow::anyhow!("Invalid null byte in message"));
        }
        
        Ok(())
    }
    
    pub fn validate_method(&self, method: &str) -> Result<()> {
        if !self.allowed_methods.contains(&method.to_string()) {
            return Err(anyhow::anyhow!("Method not allowed: {}", method));
        }
        Ok(())
    }
    
    pub fn sanitize_path(&self, path: &str) -> Result<String> {
        // Prevent path traversal
        if path.contains("..") || path.contains("//") {
            return Err(anyhow::anyhow!("Invalid path"));
        }
        
        // Remove leading/trailing slashes
        let clean = path.trim_matches('/');
        
        // Validate characters
        if !clean.chars().all(|c| c.is_alphanumeric() || c == '/' || c == '_' || c == '-' || c == '.') {
            return Err(anyhow::anyhow!("Invalid characters in path"));
        }
        
        Ok(clean.to_string())
    }
}

/// Authentication system
pub struct Authenticator {
    api_keys: Arc<RwLock<HashMap<String, ApiKey>>>,
}

#[derive(Clone)]
struct ApiKey {
    key_hash: String,
    permissions: Vec<String>,
    rate_limit_override: Option<u32>,
    created_at: Instant,
}

impl Authenticator {
    pub fn new() -> Self {
        Self {
            api_keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn add_api_key(&self, key: &str, permissions: Vec<String>) -> Result<()> {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());
        
        let mut keys = self.api_keys.write().await;
        keys.insert(key.to_string(), ApiKey {
            key_hash,
            permissions,
            rate_limit_override: None,
            created_at: Instant::now(),
        });
        
        Ok(())
    }
    
    pub async fn authenticate(&self, api_key: &str) -> Result<Vec<String>> {
        let keys = self.api_keys.read().await;
        
        if let Some(key_info) = keys.get(api_key) {
            Ok(key_info.permissions.clone())
        } else {
            Err(anyhow::anyhow!("Invalid API key"))
        }
    }
    
    pub async fn has_permission(&self, api_key: &str, permission: &str) -> bool {
        if let Ok(permissions) = self.authenticate(api_key).await {
            permissions.contains(&permission.to_string()) || permissions.contains(&"*".to_string())
        } else {
            false
        }
    }
}

/// Security middleware
pub struct SecurityMiddleware {
    rate_limiter: RateLimiter,
    validator: InputValidator,
    authenticator: Authenticator,
}

impl SecurityMiddleware {
    pub fn new() -> Self {
        Self {
            rate_limiter: RateLimiter::new(1000, 100),
            validator: InputValidator::new(),
            authenticator: Authenticator::new(),
        }
    }
    
    pub async fn process_request(&self, client_id: &str, api_key: &str, method: &str, data: &[u8]) -> Result<()> {
        // Rate limiting
        if !self.rate_limiter.check_rate_limit(client_id).await? {
            return Err(anyhow::anyhow!("Rate limit exceeded"));
        }
        
        // Authentication
        let permissions = self.authenticator.authenticate(api_key).await?;
        if !permissions.contains(&method.to_string()) && !permissions.contains(&"*".to_string()) {
            return Err(anyhow::anyhow!("Permission denied"));
        }
        
        // Input validation
        self.validator.validate_message(data)?;
        self.validator.validate_method(method)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10, 5);
        
        // Should allow burst
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("client1").await.unwrap());
        }
        
        // Should be rate limited
        assert!(!limiter.check_rate_limit("client1").await.unwrap());
    }
    
    #[test]
    fn test_input_validation() {
        let validator = InputValidator::new();
        
        assert!(validator.validate_message(b"valid data").is_ok());
        assert!(validator.validate_message(&vec![0xFF; 11 * 1024 * 1024]).is_err());
        
        assert!(validator.validate_method("get").is_ok());
        assert!(validator.validate_method("hack").is_err());
        
        assert!(validator.sanitize_path("/valid/path").is_ok());
        assert!(validator.sanitize_path("../etc/passwd").is_err());
    }
    
    #[tokio::test]
    async fn test_authentication() {
        let auth = Authenticator::new();
        
        auth.add_api_key("test_key", vec!["read".to_string(), "write".to_string()]).await.unwrap();
        
        assert!(auth.has_permission("test_key", "read").await);
        assert!(auth.has_permission("test_key", "write").await);
        assert!(!auth.has_permission("test_key", "delete").await);
        assert!(!auth.has_permission("invalid_key", "read").await);
    }
}
