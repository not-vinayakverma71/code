/// Load Test - 1K Concurrent Requests
/// Tests all 7 AI providers under heavy concurrent load

use lapce_ai_rust::ai_providers::{
    core_trait::{AiProvider, ChatRequest, ChatMessage, HealthStatus},
    provider_manager::{ProviderManager, ProvidersConfig, ProviderConfig, ProviderMetrics},
    // openai_exact::OpenAIProvider, // Module not available
    // anthropic_exact::AnthropicProvider, // Module not available
    // gemini_exact::GeminiProvider, // Module not available
    // azure_exact::AzureProvider, // Module not available
    // xai_exact::XAiProvider, // Module not available
    // xai_exact::XAiProvider, // Module not available
    // vertex_ai_exact::VertexAiProvider, // Module not available
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use futures::future::join_all;
use std::collections::HashMap;

#[tokio::test]
async fn test_1k_concurrent_requests() {
    println!("🚀 Starting 1K Concurrent Requests Load Test");
    
    // Create all 7 providers
    let providers = create_test_providers().await;
    
    // Test parameters
    const TOTAL_REQUESTS: usize = 1000;
    const MAX_CONCURRENT: usize = 100;
    const TIMEOUT: Duration = Duration::from_secs(30);
    
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let start = Instant::now();
    
    // Create test requests
    let mut tasks = Vec::new();
    
    for i in 0..TOTAL_REQUESTS {
        let provider = providers[i % providers.len()].clone();
        let sem = semaphore.clone();
        let request_id = i;
        
        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let request_start = Instant::now();
            
            let request = create_test_request(request_id);
            
            // Execute request with timeout
            let result = tokio::time::timeout(
                TIMEOUT,
                provider.chat(request)
            ).await;
            
            let latency = request_start.elapsed();
            
            match result {
                Ok(Ok(_response)) => {
                    println!("✅ Request {} completed in {:?}", request_id, latency);
                    (true, latency)
                }
                Ok(Err(e)) => {
                    println!("❌ Request {} failed: {}", request_id, e);
                    (false, latency)
                }
                Err(_) => {
                    println!("⏱️ Request {} timed out", request_id);
                    (false, TIMEOUT)
                }
            }
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks
    let results = join_all(tasks).await;
    
    // Analyze results
    let total_time = start.elapsed();
    let mut successful = 0;
    let mut failed = 0;
    let mut total_latency = Duration::ZERO;
    let mut max_latency = Duration::ZERO;
    let mut min_latency = Duration::from_secs(1000);
    
    for result in results {
        if let Ok((success, latency)) = result {
            if success {
                successful += 1;
                total_latency += latency;
                max_latency = max_latency.max(latency);
                min_latency = min_latency.min(latency);
            } else {
                failed += 1;
            }
        }
    }
    
    // Print results
    println!("\n📊 Load Test Results");
    println!("====================");
    println!("Total Requests: {}", TOTAL_REQUESTS);
    println!("Successful: {} ({:.1}%)", successful, (successful as f64 / TOTAL_REQUESTS as f64) * 100.0);
    println!("Failed: {} ({:.1}%)", failed, (failed as f64 / TOTAL_REQUESTS as f64) * 100.0);
    println!("Total Time: {:?}", total_time);
    println!("Throughput: {:.1} req/sec", TOTAL_REQUESTS as f64 / total_time.as_secs_f64());
    
    if successful > 0 {
        println!("Average Latency: {:?}", total_latency / successful as u32);
        println!("Min Latency: {:?}", min_latency);
        println!("Max Latency: {:?}", max_latency);
    }
    
    // Assert success criteria
    assert!(successful as f64 / TOTAL_REQUESTS as f64 >= 0.95, "At least 95% success rate required");
    assert!(total_time.as_secs() < 60, "Should complete within 60 seconds");
}

#[tokio::test]
async fn test_provider_rate_limiting() {
    println!("🚦 Testing Rate Limiting");
    
    let config = ProvidersConfig {
        providers: HashMap::from([
            ("openai".to_string(), ProviderConfig {
                name: "openai".to_string(),
                api_key: "test-key".to_string(),
                base_url: None,
                max_retries: 3,
                timeout: Duration::from_secs(30),
                rate_limit: Some(60), // 60 requests per minute
            }),
        ]),
        default_provider: "openai".to_string(),
        health_check_interval: Duration::from_secs(30),
        circuit_breaker_threshold: 5,
        circuit_breaker_timeout: Duration::from_secs(60),
    };
    
    let manager = ProviderManager::new(config).await.unwrap();
    
    // Try to send 100 requests rapidly
    let mut tasks = Vec::new();
    let start = Instant::now();
    
    for i in 0..100 {
        let mgr = manager.clone();
        tasks.push(tokio::spawn(async move {
            let request = create_test_request(i);
            mgr.chat(request).await
        }));
    }
    
    let results = join_all(tasks).await;
    let elapsed = start.elapsed();
    
    // Count rate limit errors
    let mut rate_limited = 0;
    for result in results {
        if let Ok(Err(e)) = result {
            if e.to_string().contains("Rate limit") {
                rate_limited += 1;
            }
        }
    }
    
    println!("Rate limited requests: {}/100", rate_limited);
    println!("Time elapsed: {:?}", elapsed);
    
    // Should have rate limiting in effect
    assert!(rate_limited > 0, "Rate limiting should be active");
}

#[tokio::test]
async fn test_circuit_breaker() {
    println!("⚡ Testing Circuit Breaker");
    
    // Create a provider that always fails
    let failing_provider = create_failing_provider();
    
    // Send requests until circuit breaker opens
    let mut consecutive_failures = 0;
    let mut circuit_opened = false;
    
    for i in 0..20 {
        let request = create_test_request(i);
        match failing_provider.chat(request).await {
            Err(e) => {
                consecutive_failures += 1;
                if e.to_string().contains("Circuit breaker open") {
                    circuit_opened = true;
                    println!("Circuit breaker opened after {} failures", consecutive_failures);
                    break;
                }
            }
            Ok(_) => {
                consecutive_failures = 0;
            }
        }
    }
    
    assert!(circuit_opened, "Circuit breaker should open after consecutive failures");
}

// Helper functions
async fn create_test_providers() -> Vec<Arc<dyn AiProvider + Send + Sync>> {
    vec![
        Arc::new(OpenAIProvider::new("test-key".to_string(), None)),
        Arc::new(AnthropicProvider::new("test-key".to_string(), None)),
        Arc::new(GeminiProvider::new("test-key".to_string(), None)),
        Arc::new(BedrockProvider::new("us-east-1".to_string())),
        Arc::new(AzureProvider::new(
            "https://test.openai.azure.com".to_string(),
            "test-key".to_string(),
            "2023-05-15".to_string()
        )),
        Arc::new(XAiProvider::new("test-key".to_string())),
        Arc::new(VertexAiProvider::new(
            "test-project".to_string(),
            "us-central1".to_string(),
            None
        )),
    ]
}

fn create_test_request(id: usize) -> ChatRequest {
    ChatRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some("You are a helpful assistant.".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(format!("Test request #{}", id)),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: None,
        stream: Some(false),
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        response_format: None,
        seed: None,
        tools: None,
        tool_choice: None,
        user: None,
    }
}

fn create_failing_provider() -> Arc<dyn AiProvider + Send + Sync> {
    // Mock provider that always fails
    struct FailingProvider;
    
    #[async_trait::async_trait]
    impl AiProvider for FailingProvider {
        fn name(&self) -> &'static str { "failing" }
        
        async fn health_check(&self) -> anyhow::Result<HealthStatus> {
            Err(anyhow::anyhow!("Always fails"))
        }
        
        async fn chat(&self, _request: ChatRequest) -> anyhow::Result<ChatResponse> {
            Err(anyhow::anyhow!("Provider failure"))
        }
        
        async fn chat_stream(&self, _request: ChatRequest) -> anyhow::Result<BoxStream<'static, anyhow::Result<StreamToken>>> {
            Err(anyhow::anyhow!("Provider failure"))
        }
        
        async fn complete(&self, _request: CompletionRequest) -> anyhow::Result<CompletionResponse> {
            Err(anyhow::anyhow!("Provider failure"))
        }
        
        async fn complete_stream(&self, _request: CompletionRequest) -> anyhow::Result<BoxStream<'static, anyhow::Result<StreamToken>>> {
            Err(anyhow::anyhow!("Provider failure"))
        }
        
        fn get_capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
        
        async fn list_models(&self) -> anyhow::Result<Vec<Model>> {
            Ok(vec![])
        }
    }
    
    Arc::new(FailingProvider)
}
