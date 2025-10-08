/// Security module for IPC implementation
/// Provides authentication, audit logging, and security enforcement

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use dashmap::DashMap;
use anyhow::{Result, bail};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error};

/// Security configuration for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication for handshake
    pub auth_enabled: bool,
    
    /// Optional shared secret for authentication
    pub auth_token: Option<String>,
    
    /// Enable audit logging
    pub audit_enabled: bool,
    
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    
    /// File permissions for shared memory (octal)
    pub shm_permissions: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auth_enabled: false,
            auth_token: None,
            audit_enabled: true,
            rate_limit: RateLimitConfig::default(),
            shm_permissions: 0o600, // Owner read/write only
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Max requests per second per connection
    pub max_rps: u32,
    
    /// Max burst size
    pub burst_size: u32,
    
    /// Enable rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_rps: 1000,
            burst_size: 100,
            enabled: true,
        }
    }
}

/// Handshake authentication
#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeAuth {
    /// Client identifier
    pub client_id: String,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// HMAC signature
    pub signature: String,
    
    /// Nonce for replay protection
    pub nonce: u64,
}

impl HandshakeAuth {
    pub fn new(client_id: String, secret: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let nonce = rand::random::<u64>();
        
        let signature = Self::compute_signature(&client_id, timestamp, nonce, secret);
        
        Self {
            client_id,
            timestamp,
            signature,
            nonce,
        }
    }
    
    fn compute_signature(client_id: &str, timestamp: u64, nonce: u64, secret: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(client_id.as_bytes());
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&nonce.to_le_bytes());
        hasher.update(secret.as_bytes());
        
        format!("{:x}", hasher.finalize())
    }
    
    pub fn verify(&self, secret: &str, max_age_secs: u64) -> Result<()> {
        // Check timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now > self.timestamp + max_age_secs {
            bail!("Authentication expired");
        }
        
        // Verify signature
        let expected = Self::compute_signature(&self.client_id, self.timestamp, self.nonce, secret);
        
        if self.signature != expected {
            bail!("Invalid signature");
        }
        
        Ok(())
    }
}

/// Audit log entry
#[derive(Debug, Serialize)]
pub struct AuditLogEntry {
    pub timestamp: u64,
    pub event_type: AuditEventType,
    pub connection_id: u64,
    pub client_id: Option<String>,
    pub details: String,
    pub success: bool,
}

#[derive(Debug, Serialize, Clone)]
pub enum AuditEventType {
    ConnectionEstablished,
    ConnectionClosed,
    AuthenticationAttempt,
    AuthenticationSuccess,
    AuthenticationFailure,
    RateLimitExceeded,
    MessageReceived,
    MessageSent,
    ErrorOccurred,
}

/// Security manager for IPC connections
pub struct SecurityManager {
    config: SecurityConfig,
    
    /// Nonce tracker for replay protection
    nonces: Arc<DashMap<u64, u64>>,
    
    /// Rate limiter state
    rate_limiters: Arc<DashMap<u64, RateLimiter>>,
    
    /// Audit log sink (could be file, syslog, etc.)
    audit_sink: Arc<dyn AuditSink>,
}

impl SecurityManager {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            nonces: Arc::new(DashMap::new()),
            rate_limiters: Arc::new(DashMap::new()),
            audit_sink: Arc::new(LoggingAuditSink::new()),
        }
    }
    
    /// Authenticate a handshake request
    pub fn authenticate_handshake(&self, auth: &HandshakeAuth) -> Result<()> {
        if !self.config.auth_enabled {
            return Ok(());
        }
        
        let secret = self.config.auth_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Auth enabled but no token configured"))?;
        
        // Check for replay
        if self.nonces.contains_key(&auth.nonce) {
            bail!("Replay attack detected");
        }
        
        // Verify signature
        auth.verify(secret, 60)?;  // 60 second max age
        
        // Store nonce
        self.nonces.insert(auth.nonce, auth.timestamp);
        
        // Clean old nonces periodically (TODO: background task)
        if self.nonces.len() > 10000 {
            let cutoff = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - 3600;  // 1 hour
            
            self.nonces.retain(|_, &mut v| v > cutoff);
        }
        
        Ok(())
    }
    
    /// Check rate limit for a connection
    pub fn check_rate_limit(&self, connection_id: u64) -> Result<()> {
        if !self.config.rate_limit.enabled {
            return Ok(());
        }
        
        let mut entry = self.rate_limiters.entry(connection_id).or_insert_with(|| {
            RateLimiter::new(self.config.rate_limit.max_rps, self.config.rate_limit.burst_size)
        });
        
        if !entry.allow_request() {
            self.audit_log(AuditLogEntry {
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                event_type: AuditEventType::RateLimitExceeded,
                connection_id,
                client_id: None,
                details: format!("Rate limit exceeded: {} rps", self.config.rate_limit.max_rps),
                success: false,
            });
            
            bail!("Rate limit exceeded");
        }
        
        Ok(())
    }
    
    /// Log an audit event
    pub fn audit_log(&self, entry: AuditLogEntry) {
        if self.config.audit_enabled {
            self.audit_sink.log(entry);
        }
    }
    
    /// Get SHM permissions
    pub fn shm_permissions(&self) -> u32 {
        self.config.shm_permissions
    }
}

/// Token bucket rate limiter
struct RateLimiter {
    tokens: AtomicU64,
    max_tokens: u64,
    refill_rate: u64,
    last_refill: AtomicU64,
}

impl RateLimiter {
    fn new(max_rps: u32, burst_size: u32) -> Self {
        Self {
            tokens: AtomicU64::new(burst_size as u64),
            max_tokens: burst_size as u64,
            refill_rate: max_rps as u64,
            last_refill: AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64
            ),
        }
    }
    
    fn allow_request(&self) -> bool {
        // Refill tokens
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let last = self.last_refill.load(Ordering::Relaxed);
        let elapsed_ms = now.saturating_sub(last);
        
        if elapsed_ms >= 1000 {  // Refill every second
            let new_tokens = (elapsed_ms / 1000) * self.refill_rate;
            let current = self.tokens.load(Ordering::Relaxed);
            let updated = (current + new_tokens).min(self.max_tokens);
            
            self.tokens.store(updated, Ordering::Relaxed);
            self.last_refill.store(now, Ordering::Relaxed);
        }
        
        // Try to consume a token
        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current == 0 {
                return false;
            }
            
            if self.tokens.compare_exchange(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed
            ).is_ok() {
                return true;
            }
        }
    }
}

/// Audit sink trait
pub trait AuditSink: Send + Sync {
    fn log(&self, entry: AuditLogEntry);
}

/// Simple logging audit sink
struct LoggingAuditSink {
    // Could add file handle, syslog connection, etc.
}

impl LoggingAuditSink {
    fn new() -> Self {
        Self {}
    }
}

impl AuditSink for LoggingAuditSink {
    fn log(&self, entry: AuditLogEntry) {
        let level = if entry.success {
            tracing::Level::INFO
        } else {
            tracing::Level::WARN
        };
        
        match level {
            tracing::Level::INFO => {
                info!(
                    "[AUDIT] {} - {:?} - Connection {} - {}",
                    entry.timestamp,
                    entry.event_type,
                    entry.connection_id,
                    entry.details
                );
            }
            tracing::Level::WARN => {
                warn!(
                    "[AUDIT] {} - {:?} - Connection {} - {}",
                    entry.timestamp,
                    entry.event_type,
                    entry.connection_id,
                    entry.details
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_handshake_auth() {
        let secret = "test_secret";
        let auth = HandshakeAuth::new("client1".to_string(), secret);
        
        // Should verify successfully
        assert!(auth.verify(secret, 60).is_ok());
        
        // Should fail with wrong secret
        assert!(auth.verify("wrong_secret", 60).is_err());
    }
    
    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(10, 5);
        
        // Should allow burst
        for _ in 0..5 {
            assert!(limiter.allow_request());
        }
        
        // Should be rate limited
        assert!(!limiter.allow_request());
    }
    
    #[test]
    fn test_security_config_permissions() {
        let config = SecurityConfig::default();
        assert_eq!(config.shm_permissions, 0o600);
    }
}
