/// Security Integration Tests
/// Tests per-boot SHM namespace, auth token validation, and rate limiting

use lapce_ai_rust::ipc::security::{SecurityManager, SecurityConfig, HandshakeAuth, AuditEventType};
use lapce_ai_rust::ipc::shm_namespace::{
    get_boot_suffix, create_namespaced_path, extract_base_path, 
    is_current_boot_path, cleanup_stale_shm_segments
};
use std::time::Duration;
use std::thread;

#[test]
fn test_per_boot_shm_namespace() {
    // Boot suffix should be consistent within same process
    let suffix1 = get_boot_suffix();
    let suffix2 = get_boot_suffix();
    assert_eq!(suffix1, suffix2, "Boot suffix should be consistent");
    
    // Should be 8 hex characters
    assert_eq!(suffix1.len(), 8, "Boot suffix should be 8 characters");
    assert!(u32::from_str_radix(suffix1, 16).is_ok(), "Boot suffix should be valid hex");
    
    println!("Boot suffix: {}", suffix1);
}

#[test]
fn test_namespaced_path_creation() {
    let base_path = "/test_ipc";
    let namespaced = create_namespaced_path(base_path);
    
    // Should contain base path and suffix
    assert!(namespaced.starts_with("/test_ipc-"), "Should start with base path");
    assert_eq!(namespaced.len(), "/test_ipc-".len() + 8, "Should have 8-char suffix");
    
    // Test path without leading slash
    let namespaced2 = create_namespaced_path("test_ipc");
    assert!(namespaced2.starts_with("/test_ipc-"), "Should add leading slash");
}

#[test]
fn test_base_path_extraction() {
    let namespaced = "/test_ipc-12ab34cd";
    let extracted = extract_base_path(namespaced);
    assert_eq!(extracted, "/test_ipc", "Should extract original base path");
    
    // Test malformed path
    let malformed = "/test_ipc";
    let extracted2 = extract_base_path(malformed);
    assert_eq!(extracted2, "/test_ipc", "Should handle path without suffix");
}

#[test]
fn test_current_boot_path_detection() {
    let suffix = get_boot_suffix();
    let current_path = format!("/test-{}", suffix);
    let old_path = "/test-deadbeef";
    
    assert!(is_current_boot_path(&current_path), "Should recognize current boot path");
    assert!(!is_current_boot_path(old_path), "Should reject old boot path");
}

#[test]
fn test_stale_segment_cleanup() {
    let base_paths = vec!["/test_cleanup"];
    
    // This should not panic even if /dev/shm doesn't exist or is inaccessible
    let result = cleanup_stale_shm_segments(&base_paths);
    
    // Either succeeds or fails gracefully
    match result {
        Ok(()) => println!("Cleanup completed successfully"),
        Err(e) => println!("Cleanup failed (expected in test environment): {}", e),
    }
}

#[test]
fn test_handshake_authentication() {
    let secret = "test_secret_key_123";
    let client_id = "test_client_001";
    
    // Create authentication
    let auth = HandshakeAuth::new(client_id.to_string(), secret);
    
    // Should verify with correct secret
    assert!(auth.verify(secret, 60).is_ok(), "Should verify with correct secret");
    
    // Should fail with wrong secret
    assert!(auth.verify("wrong_secret", 60).is_err(), "Should fail with wrong secret");
    
    // Test timestamp expiry (create old auth)
    let mut old_auth = HandshakeAuth::new(client_id.to_string(), secret);
    old_auth.timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() - 120; // 2 minutes ago
    
    assert!(old_auth.verify(secret, 60).is_err(), "Should fail when expired");
}

#[test]
fn test_rate_limiting() {
    let mut config = SecurityConfig::default();
    config.rate_limit.enabled = true;
    config.rate_limit.max_rps = 5;
    config.rate_limit.burst_size = 3;
    
    let security = SecurityManager::new(config);
    let connection_id = 12345;
    
    // Should allow burst
    for i in 0..3 {
        assert!(security.check_rate_limit(connection_id).is_ok(), 
                "Should allow burst request {}", i);
    }
    
    // Should be rate limited
    assert!(security.check_rate_limit(connection_id).is_err(), 
            "Should be rate limited after burst");
}

#[test]
fn test_security_config_defaults() {
    let config = SecurityConfig::default();
    
    assert!(!config.auth_enabled, "Auth should be disabled by default");
    assert!(config.audit_enabled, "Audit should be enabled by default");
    assert_eq!(config.shm_permissions, 0o600, "Should use secure permissions");
    assert!(config.rate_limit.enabled, "Rate limiting should be enabled");
    assert_eq!(config.rate_limit.max_rps, 1000, "Default RPS should be 1000");
    assert_eq!(config.rate_limit.burst_size, 100, "Default burst should be 100");
}

#[test]
fn test_authentication_with_security_manager() {
    let mut config = SecurityConfig::default();
    config.auth_enabled = true;
    config.auth_token = Some("test_token".to_string());
    
    let security = SecurityManager::new(config);
    
    // Valid authentication
    let auth = HandshakeAuth::new("client1".to_string(), "test_token");
    assert!(security.authenticate_handshake(&auth).is_ok(), 
            "Should authenticate with valid token");
    
    // Invalid token
    let bad_auth = HandshakeAuth::new("client2".to_string(), "wrong_token");
    assert!(security.authenticate_handshake(&bad_auth).is_err(), 
            "Should reject invalid token");
}

#[test]
fn test_replay_attack_prevention() {
    let mut config = SecurityConfig::default();
    config.auth_enabled = true;
    config.auth_token = Some("test_token".to_string());
    
    let security = SecurityManager::new(config);
    
    let auth = HandshakeAuth::new("client1".to_string(), "test_token");
    
    // First authentication should succeed
    assert!(security.authenticate_handshake(&auth).is_ok(), 
            "First auth should succeed");
    
    // Second authentication with same nonce should fail
    assert!(security.authenticate_handshake(&auth).is_err(), 
            "Replay should be rejected");
}

#[test]
fn test_audit_logging() {
    let config = SecurityConfig::default();
    let security = SecurityManager::new(config);
    
    // This should not panic
    security.audit_log(lapce_ai_rust::ipc::security::AuditLogEntry {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        event_type: AuditEventType::ConnectionEstablished,
        connection_id: 123,
        client_id: Some("test_client".to_string()),
        details: "Test audit log entry".to_string(),
        success: true,
    });
}

#[test]
fn test_disabled_authentication() {
    let mut config = SecurityConfig::default();
    config.auth_enabled = false; // Disabled
    
    let security = SecurityManager::new(config);
    
    // Should allow any authentication when disabled
    let auth = HandshakeAuth::new("client1".to_string(), "any_secret");
    assert!(security.authenticate_handshake(&auth).is_ok(), 
            "Should allow when auth disabled");
}

#[test]
fn test_concurrent_rate_limiting() {
    let mut config = SecurityConfig::default();
    config.rate_limit.enabled = true;
    config.rate_limit.max_rps = 10;
    config.rate_limit.burst_size = 5;
    
    let security = std::sync::Arc::new(SecurityManager::new(config));
    let mut handles = vec![];
    
    // Test multiple connections concurrently
    for conn_id in 0..3 {
        let security_clone = security.clone();
        let handle = thread::spawn(move || {
            let mut success_count = 0;
            let mut failure_count = 0;
            
            for _ in 0..10 {
                match security_clone.check_rate_limit(conn_id) {
                    Ok(()) => success_count += 1,
                    Err(_) => failure_count += 1,
                }
                thread::sleep(Duration::from_millis(10));
            }
            
            (success_count, failure_count)
        });
        handles.push(handle);
    }
    
    // Collect results
    let mut total_success = 0;
    let mut total_failures = 0;
    
    for handle in handles {
        let (success, failures) = handle.join().unwrap();
        total_success += success;
        total_failures += failures;
    }
    
    println!("Rate limiting test: {} successes, {} failures", 
             total_success, total_failures);
    
    // Should have some rate limiting (failures)
    assert!(total_failures > 0, "Should have some rate-limited requests");
}

#[test]
fn test_security_permissions() {
    let config = SecurityConfig::default();
    let security = SecurityManager::new(config);
    
    // Should return secure permissions
    assert_eq!(security.shm_permissions(), 0o600, 
               "Should return owner-only permissions");
}
