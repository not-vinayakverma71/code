// SEM-012-B: Fuzz-style tests for PII redaction
use lancedb::security::redaction::redact_pii;
use lancedb::search::search_metrics::SearchMetrics;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

#[test]
fn test_redact_aws_credentials() {
    let test_cases = vec![
        ("AKIAIOSFODNN7EXAMPLE", "[REDACTED_AWS_KEY]"),
        ("aws_access_key_id = AKIAIOSFODNN7EXAMPLE", "aws_access_key_id = [REDACTED_AWS_KEY]"),
        ("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY", "[REDACTED_SECRET]"),
    ];
    
    for (input, expected_contains) in test_cases {
        let redacted = redact_pii(input);
        assert!(redacted.contains(expected_contains), 
            "Failed to redact: {} -> {}", input, redacted);
        assert!(!redacted.contains("AKIA"), "AWS key not fully redacted");
    }
}

#[test]
fn test_redact_api_keys_and_tokens() {
    let test_cases = vec![
        ("api_key=sk-1234567890abcdefghijklmnop", "api_key=[REDACTED]"),
        ("API_TOKEN: abcd1234efgh5678ijkl9012mnop3456", "API_TOKEN=[REDACTED]"),
        ("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9", "Bearer [REDACTED]"),
        ("apikey='super_secret_key_12345'", "apikey=[REDACTED]"),
    ];
    
    for (input, expected_contains) in test_cases {
        let redacted = redact_pii(input);
        assert!(redacted.contains(expected_contains), 
            "Failed to redact API key: {} -> {}", input, redacted);
    }
}

#[test]
fn test_redact_emails() {
    let emails = vec![
        "john.doe@example.com",
        "admin@company.org",
        "test.user+tag@subdomain.example.co.uk",
        "noreply@localhost",
    ];
    
    for email in emails {
        let text = format!("Contact me at {}", email);
        let redacted = redact_pii(&text);
        assert!(redacted.contains("[REDACTED_EMAIL]"), 
            "Failed to redact email: {}", email);
        assert!(!redacted.contains("@"), "Email not fully redacted: {}", redacted);
    }
}

#[test]
fn test_redact_credit_cards() {
    let cc_numbers = vec![
        "4532 1234 5678 9010",
        "4532-1234-5678-9010",
        "4532123456789010",
        "5105 1051 0510 5100",
    ];
    
    for cc in cc_numbers {
        let text = format!("Payment card: {}", cc);
        let redacted = redact_pii(&text);
        assert!(redacted.contains("[REDACTED_CC]"), 
            "Failed to redact credit card: {}", cc);
        assert!(!redacted.contains("4532") && !redacted.contains("5105"), 
            "Credit card not fully redacted: {}", redacted);
    }
}

#[test]
fn test_redact_ssh_keys() {
    let ssh_key = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA1234567890abcdefghijklmnop
qrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ
-----END RSA PRIVATE KEY-----"#;
    
    let redacted = redact_pii(ssh_key);
    assert!(redacted.contains("[REDACTED_SSH_KEY]"));
    assert!(!redacted.contains("BEGIN RSA"));
}

#[test]
fn test_redact_jwt_tokens() {
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    
    let text = format!("Authorization: Bearer {}", jwt);
    let redacted = redact_pii(&text);
    assert!(!redacted.contains("eyJ"), "JWT not redacted");
    assert!(redacted.contains("[REDACTED]"), "JWT should be redacted");
}

#[test]
fn test_redact_passwords_and_secrets() {
    let test_cases = vec![
        ("password: mysecretpass123", "password=[REDACTED]"),
        ("secret=\"topsecret\"", "secret=[REDACTED]"),
        ("pwd=admin123", "pwd=[REDACTED]"),
        ("token: abc123xyz789", "token=[REDACTED]"),
    ];
    
    for (input, expected_contains) in test_cases {
        let redacted = redact_pii(input);
        assert!(redacted.contains(expected_contains), 
            "Failed to redact secret: {} -> {}", input, redacted);
    }
}

#[test]
fn test_fuzz_random_strings_with_secrets() {
    let mut rng = thread_rng();
    
    // Generate 100 random test cases
    for _ in 0..100 {
        let prefix: String = (0..10)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();
        
        let suffix: String = (0..10)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();
        
        // Inject various types of secrets
        let secrets = vec![
            format!("{}AKIAIOSFODNN7EXAMPLE{}", prefix, suffix),
            format!("{}api_key=secret123456789{}", prefix, suffix),
            format!("{}test@example.com{}", prefix, suffix),
            format!("{}4532123456789010{}", prefix, suffix),
        ];
        
        for secret_text in secrets {
            let redacted = redact_pii(&secret_text);
            
            // Verify secrets are redacted
            assert!(!redacted.contains("AKIA"), "AWS key leaked in fuzz test");
            assert!(!redacted.contains("secret123456789"), "API key leaked in fuzz test");
            assert!(!redacted.contains("@example.com"), "Email leaked in fuzz test");
            assert!(!redacted.contains("4532123456789010"), "Credit card leaked in fuzz test");
        }
    }
}

#[test]
fn test_redaction_in_metrics() {
    let metrics = SearchMetrics::new();
    
    // Test that metrics properly redact sensitive data
    let sensitive_error = "Database error: password=admin123 failed";
    metrics.record_error(sensitive_error);
    
    let sensitive_operation = "api_key=AKIAIOSFODNN7EXAMPLE";
    metrics.record_aws_titan_request(
        std::time::Duration::from_millis(100),
        sensitive_operation
    );
    
    // Export metrics and verify no sensitive data appears
    let exported = lancedb::search::search_metrics::export_metrics();
    assert!(!exported.contains("admin123"), "Password leaked in metrics");
    assert!(!exported.contains("AKIA"), "AWS key leaked in metrics");
}

#[test]
fn test_complex_multi_secret_redaction() {
    let complex_text = r#"
    Configuration:
    AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
    AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
    API_KEY=sk-proj-1234567890abcdefghijklmnop
    DATABASE_URL=postgres://user:password123@localhost:5432/db
    ADMIN_EMAIL=admin@company.com
    SUPPORT_EMAIL=support@company.com
    JWT_TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U
    CREDIT_CARD=4532-1234-5678-9010
    "#;
    
    let redacted = redact_pii(complex_text);
    
    // Verify all sensitive data is redacted
    assert!(!redacted.contains("AKIAIOSFODNN7"), "AWS key not redacted");
    assert!(!redacted.contains("wJalrXUtnFEMI"), "AWS secret not redacted");
    assert!(!redacted.contains("sk-proj"), "API key not redacted");
    assert!(!redacted.contains("password123"), "Password not redacted");
    assert!(!redacted.contains("@company.com"), "Email not redacted");
    assert!(!redacted.contains("eyJ"), "JWT not redacted");
    assert!(!redacted.contains("4532"), "Credit card not redacted");
    
    // Verify redaction markers are present
    assert!(redacted.contains("[REDACTED"), "Should contain redaction markers");
}

#[test]
fn test_edge_cases() {
    // Empty string
    assert_eq!(redact_pii(""), "");
    
    // Very long string with secret in middle
    let long_str = "a".repeat(10000) + "AKIAIOSFODNN7EXAMPLE" + &"b".repeat(10000);
    let redacted = redact_pii(&long_str);
    assert!(!redacted.contains("AKIA"));
    
    // Multiple secrets on same line
    let multi = "key1=AKIAIOSFODNN7EXAMPLE key2=secret@example.com";
    let redacted = redact_pii(multi);
    assert!(!redacted.contains("AKIA"));
    assert!(!redacted.contains("@example.com"));
}
