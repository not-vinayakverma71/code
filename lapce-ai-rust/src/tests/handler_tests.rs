/// Unit tests for handlers
/// DAY 8 H3-4: Translate unit tests for handlers

#[cfg(test)]
mod handler_tests {
    use crate::openai_provider_handler::*;
    use crate::streaming_response::*;
    use crate::timeout_retry_logic::*;
    use crate::handler_registration::*;
    use std::sync::Arc;
    use std::collections::HashMap;
    
    #[test]
    fn test_openai_handler_initialization() {
        let options = ApiHandlerOptions {
            openai_base_url: Some("https://api.openai.com/v1".to_string()),
            openai_api_key: Some("test-key".to_string()),
            openai_model_id: Some("gpt-4".to_string()),
            openai_streaming_enabled: Some(true),
            ..Default::default()
        };
        
        let handler = OpenAiHandler::new(options.clone());
        let (model_info, _) = handler.get_model();
        
        assert_eq!(model_info.id, "gpt-4");
        assert!(options.openai_streaming_enabled.unwrap_or(false));
    }
    
    #[test]
    fn test_xml_matcher_state_transitions() {
        let mut matcher = XmlMatcher::new();
        
        // Test opening tag detection
        let chunks = matcher.push("<thinking>".to_string());
        assert_eq!(chunks.len(), 0); // No output until closing tag
        
        // Test content accumulation
        let chunks = matcher.push("test content".to_string());
        assert_eq!(chunks.len(), 1);
        
        // Test closing tag
        let chunks = matcher.push("</thinking>".to_string());
        assert_eq!(chunks.len(), 1);
    }
    
    #[tokio::test]
    async fn test_retry_with_backoff() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            exponential_base: 2.0,
        };
        
        let mut attempts = 0;
        let result = retry_with_backoff(
            || {
                attempts += 1;
                Box::pin(async move {
                    if attempts < 2 {
                        Err("Simulated failure".to_string())
                    } else {
                        Ok("Success")
                    }
                })
            },
            config,
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        use std::time::Duration;
        
        let breaker = CircuitBreaker::new(2, Duration::from_millis(100));
        
        // First failure
        let _ = breaker.call(async { 
            Err::<(), &str>("error") 
        }).await;
        
        // Second failure - should open circuit
        let _ = breaker.call(async { 
            Err::<(), &str>("error") 
        }).await;
        
        // Circuit should be open
        let result = breaker.call(async { 
            Ok::<(), &str>(()) 
        }).await;
        
        assert!(result.is_err());
        
        // Reset circuit
        breaker.reset().await;
        
        // Should work now
        let result = breaker.call(async { 
            Ok::<(), &str>(()) 
        }).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_command_registry() {
        let registry = CommandRegistry::new();
        
        // Register a test command
        registry.register_command(
            CommandId::ActivationCompleted,
            Box::new(|| Ok(())),
        ).await;
        
        // Execute command
        let result = registry.execute_command(&CommandId::ActivationCompleted).await;
        assert!(result.is_ok());
        
        // Try non-existent command
        let result = registry.execute_command(&CommandId::AccountButtonClicked).await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_api_handler_options() {
        let mut options = ApiHandlerOptions::default();
        assert!(options.openai_base_url.is_none());
        
        options.openai_base_url = Some("https://custom.api.com".to_string());
        options.openai_use_azure = true;
        
        assert_eq!(options.openai_base_url, Some("https://custom.api.com".to_string()));
        assert!(options.openai_use_azure);
    }
    
    #[tokio::test]
    async fn test_rate_limiter() {
        use crate::timeout_retry_logic::RateLimiter;
        
        let limiter = RateLimiter::new(10.0, 5.0);
        
        // Should allow first request
        limiter.acquire(5.0).await;
        
        // Should allow second request
        limiter.acquire(5.0).await;
        
        // Third should wait (no tokens left)
        let start = std::time::Instant::now();
        limiter.acquire(1.0).await;
        let elapsed = start.elapsed();
        
        // Should have waited for refill
        assert!(elapsed.as_millis() > 0);
    }
}

#[cfg(test)]
mod provider_tests {
    use crate::openai_provider_handler::*;
    use crate::buffer_management::*;
    
    #[test]
    fn test_content_block_creation() {
        let text_block = ContentBlockParam::Text {
            block_type: "text".to_string(),
            text: "Hello, world!".to_string(),
        };
        
        let image_block = ContentBlockParam::Image {
            block_type: "image".to_string(),
            source: ImageSource {
                data: "base64data".to_string(),
                media_type: "image/png".to_string(),
            },
        };
        
        match text_block {
            ContentBlockParam::Text { text, .. } => assert_eq!(text, "Hello, world!"),
            _ => panic!("Wrong type"),
        }
        
        match image_block {
            ContentBlockParam::Image { source, .. } => {
                assert_eq!(source.media_type, "image/png");
            }
            _ => panic!("Wrong type"),
        }
    }
    
    #[test]
    fn test_stream_buffer() {
        let mut buffer = StreamBuffer::new();
        
        buffer.push(ApiStreamChunk::text("Hello ".to_string()));
        buffer.push(ApiStreamChunk::text("World".to_string()));
        buffer.push(ApiStreamChunk::reasoning("Thinking...".to_string()));
        
        assert_eq!(buffer.get_text(), "Hello World");
        assert_eq!(buffer.get_reasoning(), "Thinking...");
        
        buffer.push(ApiStreamChunk::usage(100, 50));
        let (input, output) = buffer.get_usage();
        assert_eq!(input, 100);
        assert_eq!(output, 50);
    }
    
    #[test]
    fn test_api_stream_chunk_types() {
        let text = ApiStreamChunk::text("test".to_string());
        assert!(text.is_text());
        assert_eq!(text.as_text(), Some("test"));
        
        let reasoning = ApiStreamChunk::reasoning("thinking".to_string());
        assert!(reasoning.is_reasoning());
        
        let usage = ApiStreamChunk::usage(10, 5);
        assert!(usage.is_usage());
        
        let error = ApiStreamChunk::error("ERR".to_string(), "Test error".to_string());
        assert!(error.is_error());
    }
}
