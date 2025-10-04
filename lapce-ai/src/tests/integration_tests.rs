/// Integration tests
/// DAY 8 H5-6: Port integration tests

#[cfg(test)]
mod integration_tests {
    use crate::ipc_server_exact_translation::*;
    use crate::ipc_client_exact_translation::*;
    use crate::task_exact_translation::*;
    use crate::metrics_collection::*;
    use crate::prometheus_export::*;
    use crate::auto_reconnection::*;
    use crate::backpressure_handling::*;
    use crate::rate_limiting::*;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_end_to_end_ipc_flow() {
        let socket_path = "/tmp/test_e2e_ipc.sock";
        let _ = std::fs::remove_file(socket_path);
        
        // Start server
        let server = IpcServer::new(socket_path.to_string());
        tokio::spawn(async move {
            server.listen().await;
        });
        
        sleep(Duration::from_millis(100)).await;
        
        // Create client
        let client = IpcClient::new(socket_path.to_string());
        sleep(Duration::from_millis(100)).await;
        
        // Send command
        let command = TaskCommand::StartNewTask {
            configuration: RooCodeSettings {
                api_key: Some("test-key".to_string()),
                model: Some("gpt-4".to_string()),
                max_tokens: Some(1000),
                temperature: Some(0.7),
            },
            text: "Test task".to_string(),
            images: None,
            new_tab: None,
        };
        
        client.send_command(command).await;
        
        // Verify connection
        assert!(client.is_connected().await);
        assert!(client.is_ready().await);
        
        // Cleanup
        client.disconnect().await;
        let _ = std::fs::remove_file(socket_path);
    }
    
    #[tokio::test]
    async fn test_task_with_metrics() {
        use crate::global_settings_exact_translation::*;
        
        let provider = Arc::new(ClineProvider {
            context: ExtensionContext {
                global_storage_uri: std::env::temp_dir(),
                workspace_state: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            },
            state: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });
        
        let options = TaskOptions {
            context: ExtensionContext {
                global_storage_uri: std::env::temp_dir(),
                workspace_state: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            },
            provider,
            api_configuration: ProviderSettings::default(),
            task: Some("Metrics test task".to_string()),
            ..Default::default()
        };
        
        let task = Task::new(options);
        
        // Create metrics collector
        let mut collector = MetricsCollector::new();
        
        // Simulate some API calls
        let usage = TokenUsage {
            total_tokens_in: 100,
            total_tokens_out: 50,
            total_cache_writes: Some(10),
            total_cache_reads: Some(5),
            total_cost: 0.01,
            context_tokens: 150,
        };
        
        collector.add_sample(usage.clone());
        
        // Export to Prometheus
        let exporter = PrometheusExporter::new("test");
        exporter.export_token_usage(&usage).await;
        
        let prometheus_output = exporter.export().await;
        
        assert!(prometheus_output.contains("test_tokens_in_total 100"));
        assert!(prometheus_output.contains("test_tokens_out_total 50"));
        assert!(prometheus_output.contains("test_cost_total 0.01"));
    }
    
    #[tokio::test]
    async fn test_reconnection_with_backpressure() {
        let reconnect_manager = Arc::new(AutoReconnectionManager::new(
            ReconnectionStrategy::Fixed { delay_ms: 10 },
            3,
            || Box::pin(async { Ok(()) })
        ));
        
        let backpressure = Arc::new(BackpressureController::new(
            BackpressureStrategy::DropOldest,
            10
        ));
        
        // Connect
        reconnect_manager.connect().await.unwrap();
        assert_eq!(reconnect_manager.get_state().await, ConnectionState::Connected);
        
        // Fill backpressure buffer
        for i in 0..12 {
            let msg = Message {
                id: format!("msg_{}", i),
                payload: vec![i as u8],
                timestamp: std::time::Instant::now(),
                priority: MessagePriority::Normal,
            };
            let _ = backpressure.submit(msg).await;
        }
        
        let metrics = backpressure.get_metrics().await;
        assert!(metrics.messages_dropped > 0);
        assert!(metrics.peak_buffer_size <= 10);
    }
    
    #[tokio::test]
    async fn test_rate_limiting_integration() {
        let token_bucket = Arc::new(TokenBucketRateLimiter::new(20.0, 10.0));
        let sliding_window = Arc::new(SlidingWindowRateLimiter::new(
            Duration::from_millis(100),
            10
        ));
        
        let mut token_allowed = 0;
        let mut window_allowed = 0;
        
        // Test both limiters concurrently
        let mut handles = vec![];
        
        for i in 0..30 {
            let tb = token_bucket.clone();
            let sw = sliding_window.clone();
            
            handles.push(tokio::spawn(async move {
                let token_ok = tb.try_consume(1.0).await;
                let window_ok = sw.check_and_consume().await;
                (token_ok, window_ok)
            }));
        }
        
        for handle in handles {
            let (token, window) = handle.await.unwrap();
            if token { token_allowed += 1; }
            if window { window_allowed += 1; }
        }
        
        // Token bucket should allow ~20
        assert!(token_allowed <= 20);
        assert!(token_allowed > 0);
        
        // Sliding window should allow 10
        assert!(window_allowed <= 10);
        assert!(window_allowed > 0);
    }
    
    #[tokio::test]
    async fn test_message_pipeline() {
        use crate::message_routing_dispatch::*;
        use crate::buffer_management::*;
        
        // Create stream buffer
        let mut buffer = StreamBuffer::new();
        
        // Simulate streaming response
        buffer.push(ApiStreamChunk::text("Processing ".to_string()));
        buffer.push(ApiStreamChunk::text("your ".to_string()));
        buffer.push(ApiStreamChunk::text("request".to_string()));
        
        assert_eq!(buffer.get_text(), "Processing your request");
        
        // Add reasoning
        buffer.push(ApiStreamChunk::reasoning("Analyzing the task".to_string()));
        assert_eq!(buffer.get_reasoning(), "Analyzing the task");
        
        // Add usage
        buffer.push(ApiStreamChunk::usage(50, 25));
        let (input, output) = buffer.get_usage();
        assert_eq!(input, 50);
        assert_eq!(output, 25);
    }
    
    #[tokio::test]
    async fn test_sliding_window_truncation() {
        use crate::sliding_window_logic::*;
        
        let messages = vec![
            ApiMessage {
                role: "system".to_string(),
                content: serde_json::json!("System prompt"),
                ts: None,
                is_summary: None,
            },
            ApiMessage {
                role: "user".to_string(),
                content: serde_json::json!("First message"),
                ts: None,
                is_summary: None,
            },
            ApiMessage {
                role: "assistant".to_string(),
                content: serde_json::json!("First response"),
                ts: None,
                is_summary: None,
            },
            ApiMessage {
                role: "user".to_string(),
                content: serde_json::json!("Second message"),
                ts: None,
                is_summary: None,
            },
            ApiMessage {
                role: "assistant".to_string(),
                content: serde_json::json!("Second response"),
                ts: None,
                is_summary: None,
            },
        ];
        
        let truncated = truncate_conversation(messages.clone(), 0.4, "test-task");
        
        // Should keep first message and remove ~40% of others
        assert!(truncated.len() < messages.len());
        assert_eq!(truncated[0].role, "system");
    }
    
    #[tokio::test]
    async fn test_xml_parsing_integration() {
        use crate::xml_parsing_utils::*;
        
        let mut matcher = SimpleXmlMatcher::new("thinking");
        
        // Stream XML content
        let chunks = vec![
            "<think",
            "ing>",
            "I need to analyze this problem",
            " step by step",
            "</thinking>",
        ];
        
        for chunk in chunks {
            matcher.push(chunk);
        }
        
        let results = matcher.finish();
        assert!(!results.is_empty());
        
        // Should have captured the thinking content
        let matched_content = results.iter()
            .filter(|r| r.matched)
            .map(|r| r.data.clone())
            .collect::<String>();
        
        assert!(matched_content.contains("analyze this problem"));
    }
    
    #[tokio::test]
    async fn test_token_counting_integration() {
        use crate::token_counting::*;
        
        let content = vec![
            ContentBlockParam::Text {
                block_type: "text".to_string(),
                text: "This is a test message with some content".to_string(),
            },
            ContentBlockParam::Image {
                block_type: "image".to_string(),
                source: ImageSource {
                    data: "a".repeat(100 * 1024), // 100KB image
                    media_type: "image/png".to_string(),
                },
            },
        ];
        
        // Count with caching
        let counter = CachedTokenCounter::new();
        
        let count1 = counter.count_tokens(&content).await.unwrap();
        let count2 = counter.count_tokens(&content).await.unwrap();
        
        // Should get same result (cached)
        assert_eq!(count1, count2);
        
        // Should have reasonable token count
        assert!(count1 > 85); // At least image tokens
        assert!(count1 < 200); // Not too high
    }
    
    #[tokio::test]
    async fn test_configuration_injection() {
        use crate::configuration_management::*;
        
        std::env::set_var("TEST_API_KEY", "secret-key-123");
        std::env::set_var("TEST_BASE_URL", "https://api.example.com");
        
        let config = serde_json::json!({
            "api_key": "${env:TEST_API_KEY}",
            "base_url": "${env:TEST_BASE_URL}",
            "model": "gpt-4"
        });
        
        let injected: serde_json::Value = inject_env(config, None).await.unwrap();
        
        assert_eq!(injected["api_key"], "secret-key-123");
        assert_eq!(injected["base_url"], "https://api.example.com");
        assert_eq!(injected["model"], "gpt-4");
        
        // Cleanup
        std::env::remove_var("TEST_API_KEY");
        std::env::remove_var("TEST_BASE_URL");
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[tokio::test]
    async fn test_concurrent_message_processing() {
        use crate::backpressure_handling::*;
        
        let controller = Arc::new(BackpressureController::new(
            BackpressureStrategy::ExponentialBackoff {
                initial_delay_ms: 1,
                max_delay_ms: 10,
                multiplier: 2.0,
            },
            100
        ));
        
        let processed = Arc::new(AtomicU32::new(0));
        let mut handles = vec![];
        
        // Spawn multiple workers
        for worker_id in 0..10 {
            let controller = controller.clone();
            let processed = processed.clone();
            
            handles.push(tokio::spawn(async move {
                for i in 0..50 {
                    let msg = Message {
                        id: format!("worker_{}_msg_{}", worker_id, i),
                        payload: vec![worker_id as u8, i as u8],
                        timestamp: std::time::Instant::now(),
                        priority: if i % 3 == 0 {
                            MessagePriority::High
                        } else {
                            MessagePriority::Normal
                        },
                    };
                    
                    if controller.submit(msg).await.is_ok() {
                        processed.fetch_add(1, Ordering::Relaxed);
                    }
                    
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }
            }));
        }
        
        // Wait for all workers
        for handle in handles {
            let _ = handle.await;
        }
        
        let total = processed.load(Ordering::Relaxed);
        let metrics = controller.get_metrics().await;
        
        println!("Stress test results:");
        println!("  Total submitted: 500");
        println!("  Total processed: {}", total);
        println!("  Messages dropped: {}", metrics.messages_dropped);
        println!("  Messages delayed: {}", metrics.messages_delayed);
        println!("  Peak buffer size: {}", metrics.peak_buffer_size);
        
        // Should have processed many messages
        assert!(total > 0);
        assert!(metrics.peak_buffer_size <= 100);
    }
}
