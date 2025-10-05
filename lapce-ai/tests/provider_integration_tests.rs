/// Comprehensive integration tests for all AI providers
use lapce_ai_rust::ai_providers::{
    openai_exact::OpenAIExactClient,
    anthropic_exact::AnthropicExactClient,  
    gemini_exact::GeminiExactClient,
    azure_exact::AzureExactClient,
    vertex_ai_exact::VertexAIExactClient,
    openrouter_exact::OpenRouterExactClient,
    bedrock_exact::BedrockExactClient,
    provider_manager::ProviderManager,
    traits::{AIProvider, ChatMessage, ChatRequest, ChatResponse, StreamToken},
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use anyhow::Result;
use futures::StreamExt;
use std::env;

/// Test configuration for each provider
#[derive(Debug, Clone)]
struct ProviderTestConfig {
    name: String,
    model: String,
    api_key_env: String,
    base_url: Option<String>,
    max_tokens: u32,
    temperature: f32,
    supports_streaming: bool,
    supports_function_calling: bool,
}

/// Test results for a provider
#[derive(Debug)]
struct TestResult {
    provider: String,
    test_name: String,
    success: bool,
    duration: Duration,
    error: Option<String>,
    response_tokens: Option<usize>,
}

/// Create test configurations for all providers
fn get_provider_configs() -> Vec<ProviderTestConfig> {
    vec![
        ProviderTestConfig {
            name: "OpenAI".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            api_key_env: "OPENAI_API_KEY".to_string(),
            base_url: None,
            max_tokens: 1000,
            temperature: 0.7,
            supports_streaming: true,
            supports_function_calling: true,
        },
        ProviderTestConfig {
            name: "Anthropic".to_string(),
            model: "claude-3-haiku-20240307".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            base_url: None,
            max_tokens: 1000,
            temperature: 0.7,
            supports_streaming: true,
            supports_function_calling: false,
        },
        ProviderTestConfig {
            name: "Gemini".to_string(),
            model: "gemini-pro".to_string(),
            api_key_env: "GEMINI_API_KEY".to_string(),
            base_url: None,
            max_tokens: 1000,
            temperature: 0.7,
            supports_streaming: true,
            supports_function_calling: true,
        },
        ProviderTestConfig {
            name: "Azure".to_string(),
            model: "gpt-35-turbo".to_string(),
            api_key_env: "AZURE_API_KEY".to_string(),
            base_url: Some(env::var("AZURE_ENDPOINT").unwrap_or_default()),
            max_tokens: 1000,
            temperature: 0.7,
            supports_streaming: true,
            supports_function_calling: true,
        },
        ProviderTestConfig {
            name: "VertexAI".to_string(),
            model: "gemini-pro".to_string(),
            api_key_env: "VERTEX_AI_PROJECT".to_string(),
            base_url: None,
            max_tokens: 1000,
            temperature: 0.7,
            supports_streaming: true,
            supports_function_calling: true,
        },
        ProviderTestConfig {
            name: "OpenRouter".to_string(),
            model: "openai/gpt-3.5-turbo".to_string(),
            api_key_env: "OPENROUTER_API_KEY".to_string(),
            base_url: None,
            max_tokens: 1000,
            temperature: 0.7,
            supports_streaming: true,
            supports_function_calling: false,
        },
        ProviderTestConfig {
            name: "Bedrock".to_string(),
            model: "anthropic.claude-instant-v1".to_string(),
            api_key_env: "AWS_ACCESS_KEY_ID".to_string(),
            base_url: None,
            max_tokens: 1000,
            temperature: 0.7,
            supports_streaming: true,
            supports_function_calling: false,
        },
    ]
}

/// Create a provider instance from config
async fn create_provider(config: &ProviderTestConfig) -> Result<Box<dyn AIProvider>> {
    let api_key = env::var(&config.api_key_env)
        .unwrap_or_else(|_| format!("test-{}-key", config.name.to_lowercase()));
    
    let provider: Box<dyn AIProvider> = match config.name.as_str() {
        "OpenAI" => Box::new(OpenAIExactClient::new(
            api_key,
            config.base_url.clone(),
            None,
        )),
        "Anthropic" => Box::new(AnthropicExactClient::new(
            api_key,
            config.base_url.clone(),
        )),
        "Gemini" => Box::new(GeminiExactClient::new(
            api_key,
        )),
        "Azure" => Box::new(AzureExactClient::new(
            api_key,
            config.base_url.clone().unwrap_or_default(),
            "2024-02-01".to_string(),
        )),
        "VertexAI" => Box::new(VertexAIExactClient::new(
            api_key.clone(),
            api_key,
            "us-central1".to_string(),
        )?),
        "OpenRouter" => Box::new(OpenRouterExactClient::new(
            api_key,
        )),
        "Bedrock" => Box::new(BedrockExactClient::new(
            api_key,
            env::var("AWS_SECRET_ACCESS_KEY").unwrap_or_default(),
            "us-east-1".to_string(),
        )),
        _ => return Err(anyhow::anyhow!("Unknown provider: {}", config.name)),
    };
    
    Ok(provider)
}

/// Test basic completion
async fn test_basic_completion(
    provider: &dyn AIProvider,
    config: &ProviderTestConfig,
) -> TestResult {
    let start = Instant::now();
    let test_name = "basic_completion".to_string();
    
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "What is 2+2? Reply with just the number.".to_string(),
        },
    ];
    
    let request = ChatRequest {
        model: config.model.clone(),
        messages,
        temperature: Some(config.temperature),
        max_tokens: Some(config.max_tokens),
        stream: false,
    };
    
    let result = timeout(Duration::from_secs(30), provider.chat(request)).await;
    
    match result {
        Ok(Ok(response)) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: response.content.contains("4"),
            duration: start.elapsed(),
            error: None,
            response_tokens: Some(response.content.len()),
        },
        Ok(Err(e)) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: false,
            duration: start.elapsed(),
            error: Some(e.to_string()),
            response_tokens: None,
        },
        Err(_) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: false,
            duration: start.elapsed(),
            error: Some("Timeout after 30 seconds".to_string()),
            response_tokens: None,
        },
    }
}

/// Test streaming completion
async fn test_streaming_completion(
    provider: &dyn AIProvider,
    config: &ProviderTestConfig,
) -> TestResult {
    if !config.supports_streaming {
        return TestResult {
            provider: config.name.clone(),
            test_name: "streaming_completion".to_string(),
            success: true,
            duration: Duration::from_secs(0),
            error: Some("Streaming not supported".to_string()),
            response_tokens: None,
        };
    }
    
    let start = Instant::now();
    let test_name = "streaming_completion".to_string();
    
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "Count from 1 to 5.".to_string(),
        },
    ];
    
    let request = ChatRequest {
        model: config.model.clone(),
        messages,
        temperature: Some(config.temperature),
        max_tokens: Some(config.max_tokens),
        stream: true,
    };
    
    let result = timeout(Duration::from_secs(30), async {
        let mut stream = provider.chat_stream(request).await?;
        let mut full_response = String::new();
        let mut token_count = 0;
        
        while let Some(token_result) = stream.next().await {
            match token_result {
                Ok(token) => {
                    full_response.push_str(&token.content);
                    token_count += 1;
                }
                Err(e) => return Err(e),
            }
        }
        
        Ok((full_response, token_count))
    })
    .await;
    
    match result {
        Ok(Ok((response, tokens))) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: response.contains("1") && response.contains("5"),
            duration: start.elapsed(),
            error: None,
            response_tokens: Some(tokens),
        },
        Ok(Err(e)) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: false,
            duration: start.elapsed(),
            error: Some(e.to_string()),
            response_tokens: None,
        },
        Err(_) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: false,
            duration: start.elapsed(),
            error: Some("Timeout after 30 seconds".to_string()),
            response_tokens: None,
        },
    }
}

/// Test error handling with invalid model
async fn test_error_handling(
    provider: &dyn AIProvider,
    config: &ProviderTestConfig,
) -> TestResult {
    let start = Instant::now();
    let test_name = "error_handling".to_string();
    
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "Test".to_string(),
        },
    ];
    
    let request = ChatRequest {
        model: "invalid-model-xyz-123".to_string(),
        messages,
        temperature: Some(config.temperature),
        max_tokens: Some(config.max_tokens),
        stream: false,
    };
    
    let result = timeout(Duration::from_secs(10), provider.chat(request)).await;
    
    match result {
        Ok(Ok(_)) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: false,
            duration: start.elapsed(),
            error: Some("Expected error but got success".to_string()),
            response_tokens: None,
        },
        Ok(Err(e)) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: true,
            duration: start.elapsed(),
            error: Some(format!("Got expected error: {}", e)),
            response_tokens: None,
        },
        Err(_) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: false,
            duration: start.elapsed(),
            error: Some("Timeout".to_string()),
            response_tokens: None,
        },
    }
}

/// Test rate limiting behavior
async fn test_rate_limiting(
    provider: &dyn AIProvider,
    config: &ProviderTestConfig,
) -> TestResult {
    let start = Instant::now();
    let test_name = "rate_limiting".to_string();
    
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "Hi".to_string(),
        },
    ];
    
    let mut tasks = vec![];
    for _ in 0..5 {
        let request = ChatRequest {
            model: config.model.clone(),
            messages: messages.clone(),
            temperature: Some(config.temperature),
            max_tokens: Some(10),
            stream: false,
        };
        
        let provider_clone = provider.clone();
        tasks.push(tokio::spawn(async move {
            timeout(Duration::from_secs(30), provider_clone.chat(request)).await
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    let successful = results.iter().filter(|r| {
        matches!(r, Ok(Ok(Ok(_))))
    }).count();
    
    TestResult {
        provider: config.name.clone(),
        test_name,
        success: successful >= 3,
        duration: start.elapsed(),
        error: Some(format!("{}/5 requests successful", successful)),
        response_tokens: None,
    }
}

/// Test multi-turn conversation
async fn test_conversation(
    provider: &dyn AIProvider,
    config: &ProviderTestConfig,
) -> TestResult {
    let start = Instant::now();
    let test_name = "multi_turn_conversation".to_string();
    
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "My name is Alice.".to_string(),
        },
        ChatMessage {
            role: "assistant".to_string(),
            content: "Nice to meet you, Alice!".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "What's my name?".to_string(),
        },
    ];
    
    let request = ChatRequest {
        model: config.model.clone(),
        messages,
        temperature: Some(config.temperature),
        max_tokens: Some(config.max_tokens),
        stream: false,
    };
    
    let result = timeout(Duration::from_secs(30), provider.chat(request)).await;
    
    match result {
        Ok(Ok(response)) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: response.content.to_lowercase().contains("alice"),
            duration: start.elapsed(),
            error: None,
            response_tokens: Some(response.content.len()),
        },
        Ok(Err(e)) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: false,
            duration: start.elapsed(),
            error: Some(e.to_string()),
            response_tokens: None,
        },
        Err(_) => TestResult {
            provider: config.name.clone(),
            test_name,
            success: false,
            duration: start.elapsed(),
            error: Some("Timeout".to_string()),
            response_tokens: None,
        },
    }
}

/// Run all tests for a provider
async fn test_provider(config: ProviderTestConfig) -> Vec<TestResult> {
    println!("\n🧪 Testing {}...", config.name);
    
    let provider_result = create_provider(&config).await;
    
    let provider = match provider_result {
        Ok(p) => p,
        Err(e) => {
            println!("  ❌ Failed to create provider: {}", e);
            return vec![TestResult {
                provider: config.name.clone(),
                test_name: "initialization".to_string(),
                success: false,
                duration: Duration::from_secs(0),
                error: Some(e.to_string()),
                response_tokens: None,
            }];
        }
    };
    
    let mut results = vec![];
    
    // Run tests
    println!("  📝 Testing basic completion...");
    results.push(test_basic_completion(&*provider, &config).await);
    
    println!("  📡 Testing streaming...");
    results.push(test_streaming_completion(&*provider, &config).await);
    
    println!("  ⚠️  Testing error handling...");
    results.push(test_error_handling(&*provider, &config).await);
    
    println!("  🔄 Testing rate limiting...");
    results.push(test_rate_limiting(&*provider, &config).await);
    
    println!("  💬 Testing conversation...");
    results.push(test_conversation(&*provider, &config).await);
    
    results
}

/// Generate test report
fn generate_report(all_results: Vec<TestResult>) {
    println!("\n" + "=".repeat(80).as_str());
    println!("📊 TEST REPORT");
    println!("=" + "=".repeat(80).as_str());
    
    // Group by provider
    let mut providers: std::collections::HashMap<String, Vec<&TestResult>> = 
        std::collections::HashMap::new();
    
    for result in &all_results {
        providers.entry(result.provider.clone())
            .or_insert_with(Vec::new)
            .push(result);
    }
    
    // Summary by provider
    for (provider, results) in providers.iter() {
        let total = results.len();
        let passed = results.iter().filter(|r| r.success).count();
        let percentage = (passed as f64 / total as f64) * 100.0;
        
        println!("\n{} Provider:", provider);
        println!("  Total Tests: {}", total);
        println!("  Passed: {} ({:.1}%)", passed, percentage);
        
        for result in results {
            let status = if result.success { "✅" } else { "❌" };
            println!("    {} {} - {:.2}s", 
                status, 
                result.test_name,
                result.duration.as_secs_f64()
            );
            if let Some(error) = &result.error {
                println!("       {}", error);
            }
        }
    }
    
    // Overall summary
    let total_tests = all_results.len();
    let total_passed = all_results.iter().filter(|r| r.success).count();
    let overall_percentage = (total_passed as f64 / total_tests as f64) * 100.0;
    
    println!("\n" + "=".repeat(80).as_str());
    println!("🎯 OVERALL RESULTS");
    println!("  Total Tests: {}", total_tests);
    println!("  Passed: {} ({:.1}%)", total_passed, overall_percentage);
    println!("=" + "=".repeat(80).as_str());
}

#[tokio::test]
async fn test_all_providers() {
    // Load environment variables from .env file if present
    dotenv::dotenv().ok();
    
    println!("\n🚀 Starting comprehensive AI provider tests...\n");
    
    let configs = get_provider_configs();
    let mut all_results = vec![];
    
    for config in configs {
        let results = test_provider(config).await;
        all_results.extend(results);
    }
    
    generate_report(all_results);
}

#[tokio::test]
async fn test_single_provider_openai() {
    dotenv::dotenv().ok();
    
    let config = ProviderTestConfig {
        name: "OpenAI".to_string(),
        model: "gpt-3.5-turbo".to_string(),
        api_key_env: "OPENAI_API_KEY".to_string(),
        base_url: None,
        max_tokens: 100,
        temperature: 0.5,
        supports_streaming: true,
        supports_function_calling: true,
    };
    
    let results = test_provider(config).await;
    generate_report(results);
}

#[tokio::test]
async fn test_provider_health_checks() {
    dotenv::dotenv().ok();
    
    println!("\n🏥 Testing provider health checks...\n");
    
    let configs = get_provider_configs();
    
    for config in configs {
        print!("  Checking {}... ", config.name);
        
        match create_provider(&config).await {
            Ok(provider) => {
                match provider.health_check().await {
                    Ok(status) => println!("✅ {}", status),
                    Err(e) => println!("⚠️  Unhealthy: {}", e),
                }
            }
            Err(e) => println!("❌ Failed to create: {}", e),
        }
    }
}
