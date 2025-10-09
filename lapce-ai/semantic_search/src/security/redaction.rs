// PII Redaction Utility - SEM-012-A
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // AWS credentials patterns
    static ref AWS_ACCESS_KEY: Regex = Regex::new(r"AKIA[0-9A-Z]{16}").unwrap();
    static ref AWS_SECRET_KEY: Regex = Regex::new(r"[A-Za-z0-9/+=]{40}").unwrap();
    
    // API keys and tokens
    static ref API_KEY: Regex = Regex::new(r#"(?i)(api[_-]?key|apikey|api_token|access[_-]?token)['"]?\s*[:=]\s*['"]?([A-Za-z0-9_\-]{20,})"#).unwrap();
    static ref BEARER_TOKEN: Regex = Regex::new(r"(?i)bearer\s+[A-Za-z0-9_\-\.]+").unwrap();
    
    // Email addresses
    static ref EMAIL: Regex = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
    
    // Credit card numbers
    static ref CREDIT_CARD: Regex = Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap();
    
    // SSH keys
    static ref SSH_KEY: Regex = Regex::new(r"-----BEGIN [A-Z]+ PRIVATE KEY-----[\s\S]+?-----END [A-Z]+ PRIVATE KEY-----").unwrap();
    
    // JWT tokens
    static ref JWT: Regex = Regex::new(r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+").unwrap();
    
    // Generic secrets
    static ref SECRET: Regex = Regex::new(r#"(?i)(password|passwd|pwd|secret|token)['"]?\s*[:=]\s*['"]?([^\s'",]+)"#).unwrap();
}

/// Redact sensitive information from text
pub fn redact_pii(text: &str) -> String {
    let mut result = text.to_string();
    
    // Redact AWS credentials
    result = AWS_ACCESS_KEY.replace_all(&result, "[REDACTED_AWS_KEY]").to_string();
    result = AWS_SECRET_KEY.replace_all(&result, "[REDACTED_SECRET]").to_string();
    
    // Redact API keys and tokens
    result = API_KEY.replace_all(&result, "$1=[REDACTED]").to_string();
    result = BEARER_TOKEN.replace_all(&result, "Bearer [REDACTED]").to_string();
    
    // Redact emails
    result = EMAIL.replace_all(&result, "[REDACTED_EMAIL]").to_string();
    
    // Redact credit cards
    result = CREDIT_CARD.replace_all(&result, "[REDACTED_CC]").to_string();
    
    // Redact SSH keys
    result = SSH_KEY.replace_all(&result, "[REDACTED_SSH_KEY]").to_string();
    
    // Redact JWT tokens
    result = JWT.replace_all(&result, "[REDACTED_JWT]").to_string();
    
    // Redact generic secrets
    result = SECRET.replace_all(&result, "$1=[REDACTED]").to_string();
    
    result
}

/// Check if text contains PII
pub fn contains_pii(text: &str) -> bool {
    AWS_ACCESS_KEY.is_match(text) ||
    AWS_SECRET_KEY.is_match(text) ||
    API_KEY.is_match(text) ||
    BEARER_TOKEN.is_match(text) ||
    EMAIL.is_match(text) ||
    CREDIT_CARD.is_match(text) ||
    SSH_KEY.is_match(text) ||
    JWT.is_match(text) ||
    SECRET.is_match(text)
}

/// Redact structured fields in logs
pub fn redact_log_fields(fields: &mut std::collections::HashMap<String, String>) {
    for (key, value) in fields.iter_mut() {
        // Redact values for sensitive keys
        let key_lower = key.to_lowercase();
        if key_lower.contains("password") ||
           key_lower.contains("secret") ||
           key_lower.contains("token") ||
           key_lower.contains("key") ||
           key_lower.contains("credential") {
            *value = "[REDACTED]".to_string();
        } else {
            // Redact PII in values
            *value = redact_pii(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_redact_aws_credentials() {
        let text = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let redacted = redact_pii(text);
        assert_eq!(redacted, "AWS_ACCESS_KEY_ID=[REDACTED_AWS_KEY]");
    }
    
    #[test]
    fn test_redact_email() {
        let text = "Contact us at admin@example.com for support";
        let redacted = redact_pii(text);
        assert_eq!(redacted, "Contact us at [REDACTED_EMAIL] for support");
    }
    
    #[test]
    fn test_redact_api_key() {
        let text = "api_key=sk_test_4eC39HqLyjWDarjtT1zdp7dc";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[REDACTED]"));
    }
    
    #[test]
    fn test_redact_password() {
        let text = "password: mysecretpassword123";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[REDACTED]"));
    }
    
    #[test]
    fn test_contains_pii() {
        assert!(contains_pii("AKIAIOSFODNN7EXAMPLE"));
        assert!(contains_pii("admin@example.com"));
        assert!(contains_pii("password=secret"));
        assert!(!contains_pii("This is safe text"));
    }
    
    #[test]
    fn test_redact_log_fields() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("username".to_string(), "john".to_string());
        fields.insert("password".to_string(), "secret123".to_string());
        fields.insert("api_key".to_string(), "sk_test_key".to_string());
        fields.insert("email".to_string(), "john@example.com".to_string());
        
        redact_log_fields(&mut fields);
        
        assert_eq!(fields.get("username").unwrap(), "john");
        assert_eq!(fields.get("password").unwrap(), "[REDACTED]");
        assert_eq!(fields.get("api_key").unwrap(), "[REDACTED]");
        assert_eq!(fields.get("email").unwrap(), "[REDACTED_EMAIL]");
    }
}
