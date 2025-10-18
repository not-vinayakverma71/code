/// Security Hardening (LSP-018)
/// Rate limiting, payload size caps, PII redaction, permission gating

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use regex::Regex;
use lazy_static::lazy_static;

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum request rate per client (requests per second)
    pub max_requests_per_second: u32,
    /// Maximum payload size in bytes (10MB default)
    pub max_payload_size: usize,
    /// Maximum URI length
    pub max_uri_length: usize,
    /// Enable PII redaction in logs
    pub enable_pii_redaction: bool,
    /// Enable cross-workspace permission checks
    pub enable_workspace_gating: bool,
    /// Allowed workspace paths (if gating enabled)
    pub allowed_workspaces: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_requests_per_second: 100,
            max_payload_size: 10 * 1024 * 1024, // 10MB
            max_uri_length: 2048,
            enable_pii_redaction: true,
            enable_workspace_gating: false,
            allowed_workspaces: Vec::new(),
        }
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    tokens: Arc<parking_lot::Mutex<HashMap<String, TokenBucket>>>,
    max_tokens: u32,
    refill_rate: u32, // tokens per second
}

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(max_requests_per_second: u32) -> Self {
        Self {
            tokens: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            max_tokens: max_requests_per_second,
            refill_rate: max_requests_per_second,
        }
    }
    
    /// Check if request is allowed for client
    pub fn check_rate_limit(&self, client_id: &str) -> bool {
        let mut tokens = self.tokens.lock();
        
        let bucket = tokens.entry(client_id.to_string()).or_insert(TokenBucket {
            tokens: self.max_tokens as f64,
            last_refill: Instant::now(),
        });
        
        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate as f64;
        bucket.tokens = (bucket.tokens + new_tokens).min(self.max_tokens as f64);
        bucket.last_refill = now;
        
        // Check if we have tokens
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
    
    /// Reset rate limit for client (useful for testing)
    pub fn reset(&self, client_id: &str) {
        self.tokens.lock().remove(client_id);
    }
}

/// PII redaction patterns
lazy_static! {
    static ref PII_PATTERNS: Vec<(Regex, &'static str)> = vec![
        // API keys and tokens
        (Regex::new(r"(?i)(api[_-]?key|token|secret|password|passwd)\s*[:=]\s*[a-zA-Z0-9_\-\.]+").unwrap(), "$1: [REDACTED]"),
        // AWS credentials
        (Regex::new(r"(?i)(aws[_-]?access[_-]?key[_-]?id|aws[_-]?secret[_-]?access[_-]?key)\s*[:=]\s*[A-Z0-9]+").unwrap(), "$1: [REDACTED]"),
        // Email addresses
        (Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(), "[EMAIL_REDACTED]"),
        // IP addresses (private)
        (Regex::new(r"\b(10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(1[6-9]|2[0-9]|3[01])\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3})\b").unwrap(), "[IP_REDACTED]"),
        // SSH keys
        (Regex::new(r"(?i)(ssh-rsa|ssh-dss|ecdsa-sha2-nistp256)\s+[A-Za-z0-9+/=]+").unwrap(), "$1 [KEY_REDACTED]"),
        // JWT tokens
        (Regex::new(r"eyJ[A-Za-z0-9_-]*\.eyJ[A-Za-z0-9_-]*\.[A-Za-z0-9_-]*").unwrap(), "[JWT_REDACTED]"),
        // Credit card patterns (basic)
        (Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b").unwrap(), "[CARD_REDACTED]"),
    ];
}

/// Redact PII from string
pub fn redact_pii(text: &str) -> String {
    let mut result = text.to_string();
    for (pattern, replacement) in PII_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).to_string();
    }
    result
}

/// Security validator
pub struct SecurityValidator {
    config: SecurityConfig,
    rate_limiter: RateLimiter,
}

impl SecurityValidator {
    pub fn new(config: SecurityConfig) -> Self {
        let rate_limiter = RateLimiter::new(config.max_requests_per_second);
        Self {
            config,
            rate_limiter,
        }
    }
    
    /// Validate request
    pub fn validate_request(
        &self,
        client_id: &str,
        payload_size: usize,
        uri: &str,
    ) -> Result<()> {
        // Check rate limit
        if !self.rate_limiter.check_rate_limit(client_id) {
            return Err(anyhow!("Rate limit exceeded for client: {}", client_id));
        }
        
        // Check payload size
        if payload_size > self.config.max_payload_size {
            return Err(anyhow!(
                "Payload size {} exceeds maximum {}",
                payload_size,
                self.config.max_payload_size
            ));
        }
        
        // Check URI length
        if uri.len() > self.config.max_uri_length {
            return Err(anyhow!(
                "URI length {} exceeds maximum {}",
                uri.len(),
                self.config.max_uri_length
            ));
        }
        
        // Check workspace permissions
        if self.config.enable_workspace_gating {
            self.validate_workspace_access(uri)?;
        }
        
        Ok(())
    }
    
    /// Validate workspace access
    fn validate_workspace_access(&self, uri: &str) -> Result<()> {
        if !uri.starts_with("file://") {
            return Ok(()); // Only check file URIs
        }
        
        let path = &uri[7..]; // Remove "file://"
        
        // Check if path is in allowed workspaces
        let allowed = self.config.allowed_workspaces.iter().any(|workspace| {
            path.starts_with(workspace)
        });
        
        if !allowed {
            return Err(anyhow!("Access denied: URI not in allowed workspaces"));
        }
        
        Ok(())
    }
    
    /// Validate JSON input
    pub fn validate_json(&self, json: &str) -> Result<serde_json::Value> {
        // Check size before parsing
        if json.len() > self.config.max_payload_size {
            return Err(anyhow!("JSON size exceeds maximum"));
        }
        
        // Parse and validate
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| anyhow!("Invalid JSON: {}", e))?;
        
        // Additional validation: check depth to prevent stack overflow
        self.validate_json_depth(&value, 0)?;
        
        Ok(value)
    }
    
    fn validate_json_depth(&self, value: &serde_json::Value, depth: usize) -> Result<()> {
        const MAX_DEPTH: usize = 100;
        
        if depth > MAX_DEPTH {
            return Err(anyhow!("JSON depth exceeds maximum of {}", MAX_DEPTH));
        }
        
        match value {
            serde_json::Value::Object(map) => {
                for (_, v) in map {
                    self.validate_json_depth(v, depth + 1)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    self.validate_json_depth(v, depth + 1)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Reset rate limit (for testing)
    pub fn reset_rate_limit(&self, client_id: &str) {
        self.rate_limiter.reset(client_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(10); // 10 requests per second
        
        // First 10 requests should succeed
        for _ in 0..10 {
            assert!(limiter.check_rate_limit("client1"));
        }
        
        // 11th request should fail
        assert!(!limiter.check_rate_limit("client1"));
        
        // Different client should succeed
        assert!(limiter.check_rate_limit("client2"));
    }
    
    #[test]
    fn test_pii_redaction() {
        let text = "API_KEY=abc123 and email is user@example.com";
        let redacted = redact_pii(text);
        
        assert!(redacted.contains("[REDACTED]"));
        assert!(redacted.contains("[EMAIL_REDACTED]"));
        assert!(!redacted.contains("abc123"));
        assert!(!redacted.contains("user@example.com"));
    }
    
    #[test]
    fn test_pii_redaction_aws() {
        let text = "aws_access_key_id=AKIAIOSFODNN7EXAMPLE";
        let redacted = redact_pii(text);
        
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
    }
    
    #[test]
    fn test_pii_redaction_jwt() {
        let text = "token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let redacted = redact_pii(text);
        
        assert!(redacted.contains("[JWT_REDACTED]"));
        assert!(!redacted.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }
    
    #[test]
    fn test_security_validator_payload_size() {
        let config = SecurityConfig {
            max_payload_size: 100,
            ..Default::default()
        };
        let validator = SecurityValidator::new(config);
        
        // Small payload should succeed
        assert!(validator.validate_request("client1", 50, "file:///test.rs").is_ok());
        
        // Large payload should fail
        assert!(validator.validate_request("client1", 200, "file:///test.rs").is_err());
    }
    
    #[test]
    fn test_security_validator_uri_length() {
        let config = SecurityConfig {
            max_uri_length: 50,
            ..Default::default()
        };
        let validator = SecurityValidator::new(config);
        
        // Short URI should succeed
        assert!(validator.validate_request("client1", 10, "file:///test.rs").is_ok());
        
        // Long URI should fail
        let long_uri = format!("file:///{}", "a".repeat(100));
        assert!(validator.validate_request("client1", 10, &long_uri).is_err());
    }
    
    #[test]
    fn test_workspace_gating() {
        let config = SecurityConfig {
            enable_workspace_gating: true,
            allowed_workspaces: vec!["/home/user/project".to_string()],
            ..Default::default()
        };
        let validator = SecurityValidator::new(config);
        
        // Allowed workspace should succeed
        assert!(validator.validate_request("client1", 10, "file:///home/user/project/file.rs").is_ok());
        
        // Disallowed workspace should fail
        assert!(validator.validate_request("client1", 10, "file:///tmp/file.rs").is_err());
    }
    
    #[test]
    fn test_json_validation_depth() {
        let config = SecurityConfig::default();
        let validator = SecurityValidator::new(config);
        
        // Simple JSON should succeed
        let simple = r#"{"key": "value"}"#;
        assert!(validator.validate_json(simple).is_ok());
        
        // Deeply nested JSON should fail
        let mut deep = String::from("{");
        for _ in 0..150 {
            deep.push_str(r#""a":{"#);
        }
        deep.push_str(r#""b":"c""#);
        for _ in 0..150 {
            deep.push('}');
        }
        
        assert!(validator.validate_json(&deep).is_err());
    }
}
