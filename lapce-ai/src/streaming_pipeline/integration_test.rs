/// Integration test for streaming pipeline
/// Verifies all Phase 1 & 2 components work together

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures::stream;
    use futures::StreamExt;
    use crate::streaming_pipeline::*;
    
    #[tokio::test]
    async fn test_complete_streaming_pipeline() {
        // Phase 1: Test SSE Parser
        let mut parser = SseParser::new();
        
        let openai_data = b"data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n";
        let events = parser.parse_chunk(openai_data);
        assert_eq!(events.len(), 1);
        
        // Phase 1: Test StreamToken
        let token = StreamToken::Text("Hello".to_string());
        assert_eq!(token.as_text(), Some("Hello"));
        
        // Phase 2: Test Pipeline Builder
        let pipeline = StreamPipelineBuilder::new()
            .with_model("gpt-3.5-turbo")
            .enable_metrics()
            .add_transformer(TokenAccumulator::new(5, 100))
            .build()
            .expect("Failed to build pipeline");
        
        // Create test stream
        let chunks = vec![
            Ok(Bytes::from("data: {\"choices\":[{\"delta\":{\"content\":\"Hello \"}}]}\n\n")),
            Ok(Bytes::from("data: {\"choices\":[{\"delta\":{\"content\":\"streaming \"}}]}\n\n")),
            Ok(Bytes::from("data: {\"choices\":[{\"delta\":{\"content\":\"world!\"}}]}\n\n")),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];
        
        let input_stream = stream::iter(chunks);
        let mut output_stream = Box::pin(pipeline.process_simple(input_stream));
        
        // Collect results
        let mut results = Vec::new();
        while let Some(result) = output_stream.next().await {
            if let Ok(token) = result {
                results.push(token);
            }
        }
        
        // Verify we got tokens
        assert!(!results.is_empty());
        
        // Verify we got the done token
        assert!(results.iter().any(|t| t.is_done()));
        
        // Verify we got text content
        let text_tokens: Vec<_> = results
            .iter()
            .filter_map(|t| t.as_text())
            .collect();
        assert!(!text_tokens.is_empty());
        
        println!("✅ Integration test passed!");
        println!("  - SSE Parser: Working");
        println!("  - StreamToken: Working");
        println!("  - Pipeline: Working");
        println!("  - Transformers: Working");
        println!("  - Collected {} tokens", results.len());
    }
    
    #[tokio::test]
    async fn test_http_handler() {
        let handler = HttpStreamHandler::new();
        
        // Mock SSE chunks
        let chunks = vec![
            Ok(Bytes::from("event: message_start\ndata: {\"type\":\"start\"}\n\n")),
            Ok(Bytes::from("event: content_block_delta\ndata: {\"delta\":{\"text\":\"Hi\"}}\n\n")),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];
        
        let byte_stream = stream::iter(chunks);
        let mut token_stream = Box::pin(handler.from_byte_stream(byte_stream));
        
        let mut tokens = Vec::new();
        while let Some(result) = token_stream.next().await {
            if let Ok(token) = result {
                tokens.push(token);
            }
        }
        
        assert!(!tokens.is_empty());
        assert!(tokens.iter().any(|t| t.is_done()));
        
        println!("✅ HTTP Handler test passed!");
    }
    
    #[test]
    fn test_backpressure_controller() {
        let controller = StreamBackpressureController::new(10);
        assert_eq!(controller.available_permits(), 10);
        assert!(controller.buffer_size() > 0);
        
        println!("✅ Backpressure controller test passed!");
    }
    
    #[test]
    fn test_metrics() {
        let metrics = StreamMetrics::new();
        
        metrics.record_chunk(100, 10);
        metrics.record_chunk(200, 20);
        
        assert_eq!(metrics.chunks_processed(), 2);
        assert_eq!(metrics.tokens_generated(), 30);
        
        let summary = metrics.summary();
        println!("✅ Metrics test passed!");
        println!("{}", summary);
    }
}
