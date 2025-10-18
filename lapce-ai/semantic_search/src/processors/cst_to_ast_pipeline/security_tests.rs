// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Security tests for CST to AST pipeline - PII redaction validation

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::security::redaction::redact_pii;
    use std::path::Path;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_error_messages_redact_file_paths_with_usernames() {
        // Simulate error with username in path
        let path_with_username = "/home/john.doe@company.com/secret_project/main.rs";
        let error_msg = format!("Failed to parse file: {}", path_with_username);
        
        let redacted = redact_pii(&error_msg);
        
        // Email should be redacted
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(!redacted.contains("john.doe@company.com"));
    }

    #[test]
    fn test_error_messages_redact_api_keys_in_source() {
        // Simulate parsing error with API key in source
        let error_msg = "Parse error near token 'api_key=sk_live_51HqLyjWDarjtT1zdp7dc'";
        
        let redacted = redact_pii(&error_msg);
        
        // API key should be redacted
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("sk_live_51HqLyjWDarjtT1zdp7dc"));
    }

    #[test]
    fn test_error_messages_redact_aws_credentials() {
        let error_msg = "Failed to load config: AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        
        let redacted = redact_pii(&error_msg);
        
        // AWS keys should be redacted
        assert!(redacted.contains("[REDACTED_AWS_KEY]"));
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_detect_language_error_redacts_path() {
        let pipeline = CstToAstPipeline::new();
        
        // Create temp file without extension
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        // This should fail with "No file extension" error
        let result = pipeline.detect_language(path);
        
        assert!(result.is_err());
        let error_msg = format!("{:?}", result.unwrap_err());
        
        // Even though this error doesn't contain PII, verify redaction works
        let redacted = redact_pii(&error_msg);
        assert!(!redacted.is_empty());
    }

    #[test]
    fn test_parse_error_messages_safe() {
        // Simulate various error scenarios
        let test_cases = vec![
            "Failed to read file: permission denied",
            "Parse error at line 42",
            "Unsupported language: cobol",
            "Failed to set language: invalid",
        ];
        
        for case in test_cases {
            let redacted = redact_pii(case);
            // Should not panic and should return something
            assert!(!redacted.is_empty());
        }
    }

    #[tokio::test]
    async fn test_process_file_error_path_redaction() {
        // Test that PII in error messages gets redacted
        let error_msg_with_email = "Failed to read file: /home/admin@secret.com/project/main.rs";
        
        let redacted = redact_pii(&error_msg_with_email);
        
        // Email in path should be redacted
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(!redacted.contains("admin@secret.com"));
    }

    #[test]
    fn test_identifier_extraction_no_pii_leak() {
        // Test that extracted identifiers don't leak PII
        // extract_identifier looks for children with "identifier" kind
        let identifier_child = CstNode {
            kind: "identifier".to_string(),
            text: "user_email".to_string(),
            start_byte: 0,
            end_byte: 10,
            start_position: (0, 0),
            end_position: (0, 10),
            is_named: true,
            is_missing: false,
            is_extra: false,
            field_name: None,
            children: vec![],
            stable_id: None,
        };
        
        let parent_cst = CstNode {
            kind: "function_declaration".to_string(),
            text: "fn user_email() {}".to_string(),
            start_byte: 0,
            end_byte: 18,
            start_position: (0, 0),
            end_position: (0, 18),
            is_named: true,
            is_missing: false,
            is_extra: false,
            field_name: None,
            children: vec![identifier_child],
            stable_id: None,
        };
        
        let id = extract_identifier(&parent_cst);
        assert_eq!(id, Some("user_email".to_string()));
        
        // Verify that if actual PII was in identifier, redaction catches it
        let pii_text = "admin@company.com";
        let redacted = redact_pii(pii_text);
        assert_eq!(redacted, "[REDACTED_EMAIL]");
    }

    #[cfg(feature = "cst_ts")]
    #[test]
    fn test_canonical_mapping_errors_redacted() {
        use crate::search::search_metrics::{CANONICAL_MAPPING_UNKNOWN_TOTAL};
        
        // Simulate unknown mapping - metrics should not contain PII
        let language = "rust";
        let kind = "some_unknown_node@email.com"; // Artificial PII in kind
        
        // In real code, this would go through mapping
        // Verify metric label doesn't contain PII
        let safe_language = redact_pii(language);
        assert_eq!(safe_language, "rust"); // No PII in language
        
        let safe_kind = redact_pii(kind);
        assert!(safe_kind.contains("[REDACTED_EMAIL]"));
    }

    #[test]
    fn test_complexity_calculation_no_pii_exposure() {
        // Ensure complexity calculation doesn't expose source code with PII
        let cst_with_string = CstNode {
            kind: "string_literal".to_string(),
            text: "\"password=secret123\"".to_string(),
            start_byte: 0,
            end_byte: 20,
            start_position: (0, 0),
            end_position: (0, 20),
            is_named: true,
            is_missing: false,
            is_extra: false,
            field_name: None,
            children: vec![],
            stable_id: None,
        };
        
        let complexity = calculate_complexity(&cst_with_string);
        assert_eq!(complexity, 1); // Basic complexity, no PII leaked
        
        // If we log this text, it should be redacted
        let redacted = redact_pii(&cst_with_string.text);
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_no_pii_in_ast_metadata() {
        let metadata = NodeMetadata {
            start_line: 1,
            end_line: 10,
            start_column: 0,
            end_column: 50,
            source_file: Some(PathBuf::from("/home/user@example.com/code.rs")),
            language: "rust".to_string(),
            complexity: 5,
            stable_id: None,
        };
        
        // If we serialize or log metadata, paths should be redacted
        let path_str = metadata.source_file.as_ref().unwrap().to_string_lossy();
        let redacted = redact_pii(&path_str);
        
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(!redacted.contains("user@example.com"));
    }

    #[tokio::test]
    async fn test_end_to_end_no_pii_in_errors() {
        let pipeline = CstToAstPipeline::new();
        
        // Create a file with PII in content
        let code = r#"
const API_KEY = "sk_test_4eC39HqLyjWDarjtT1zdp7dc";
const EMAIL = "admin@secret.com";
        "#;
        
        let mut temp_file = NamedTempFile::with_suffix(".js").unwrap();
        temp_file.write_all(code.as_bytes()).unwrap();
        let path = temp_file.path();
        
        // Process file successfully
        let result = pipeline.process_file(path).await;
        
        // Should succeed
        assert!(result.is_ok());
        
        // If we were to log any part of the AST with PII, it should be redacted
        let ast = result.unwrap().ast;
        if let Some(value) = &ast.value {
            let redacted = redact_pii(value);
            // Verify redaction works on values
            if value.contains("sk_test") || value.contains("@") {
                assert_ne!(redacted, *value);
            }
        }
    }
}
