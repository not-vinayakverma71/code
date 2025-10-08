/// Test 1000 concurrent requests on multiple providers
/// Focus on AWS Bedrock Titan for 1M token validation

use anyhow::Result;
use colored::Colorize;
use futures::stream::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use lapce_ai_rust::ai_providers::{
    core_trait::{AiProvider, ChatRequest, ChatMessage, StreamToken},
    gemini_exact::{GeminiProvider, GeminiConfig},
    bedrock_exact::{BedrockProvider, BedrockConfig},
};

#[derive(Debug, Clone)]
struct LoadTestResult {
    provider: String,
    total_requests: usize,
    successful: usize,
    failed: usize,
    total_tokens: usize,
    duration_secs: f64,
    requests_per_second: f64,
    tokens_per_second: f64,
    avg_latency_ms: f64,
    p99_latency_ms: f64,
    errors: Vec<String>,
}

/// Test a single request
async fn test_single_request(
    provider: Arc<dyn AiProvider>,
    request_id: usize,
    semaphore: Arc<Semaphore>,
    token_counter: Arc<AtomicUsize>,
) -> Result<(usize, Duration)> {
    let _permit = semaphore.acquire().await?;
    let start = Instant::now();
    
    let request = ChatRequest {
        model: "amazon.titan-text-express-v1".to_string(), // Using Titan for AWS
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: Some(format!("Request #{}: Generate a short response.", request_id)),
                name: None,
                function_call: None,
                tool_calls: None,
            }
        ],
        temperature: Some(0.1),
        max_tokens: Some(50),
        stream: Some(true),
        top_p: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        user: None,
        functions: None,
        function_call: None,
        tools: None,
        tool_choice: None,
        response_format: None,
        seed: None,
        logprobs: None,
        top_logprobs: None,
    };
    
    let mut token_count = 0;
    let mut stream = provider.chat_stream(request).await?;
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(token) => {
                match token {
                    StreamToken::Text(text) => {
                        token_count += text.split_whitespace().count();
                    }
                    StreamToken::Delta { content } => {
                        token_count += content.split_whitespace().count();
                    }
                    StreamToken::Done => break,
                    StreamToken::Error(e) => {
                        return Err(anyhow::anyhow!("Stream error: {}", e));
                    }
                    _ => {}
                }
            }
            Err(e) => return Err(e),
        }
    }
    
    token_counter.fetch_add(token_count, Ordering::Relaxed);
    Ok((token_count, start.elapsed()))
}

/// Run concurrent load test
async fn run_load_test(
    provider: Arc<dyn AiProvider>,
    provider_name: String,
    num_requests: usize,
    max_concurrent: usize,
) -> LoadTestResult {
    println!("\n{}", format!("🚀 Testing {} with {} requests", provider_name, num_requests).bright_cyan().bold());
    println!("Max concurrent: {}", max_concurrent);
    
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let token_counter = Arc::new(AtomicUsize::new(0));
    let mut tasks = JoinSet::new();
    let start = Instant::now();
    
    // Launch all requests
    for i in 0..num_requests {
        let provider_clone = Arc::clone(&provider);
        let sem_clone = Arc::clone(&semaphore);
        let counter_clone = Arc::clone(&token_counter);
        
        tasks.spawn(async move {
            test_single_request(provider_clone, i, sem_clone, counter_clone).await
        });
    }
    
    // Collect results
    let mut successful = 0;
    let mut failed = 0;
    let mut errors = Vec::new();
    let mut latencies = Vec::new();
    
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok((_, latency))) => {
                successful += 1;
                latencies.push(latency.as_millis() as f64);
                
                // Print progress every 100 requests
                if (successful + failed) % 100 == 0 {
                    print!(".");
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }
            }
            Ok(Err(e)) => {
                failed += 1;
                if errors.len() < 10 { // Keep first 10 errors
                    errors.push(e.to_string());
                }
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("Task error: {}", e));
            }
        }
    }
    
    let duration = start.elapsed();
    let duration_secs = duration.as_secs_f64();
    
    // Calculate statistics
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_latency = if !latencies.is_empty() {
        latencies.iter().sum::<f64>() / latencies.len() as f64
    } else {
        0.0
    };
    
    let p99_latency = if !latencies.is_empty() {
        let p99_idx = (latencies.len() as f64 * 0.99) as usize;
        latencies[p99_idx.min(latencies.len() - 1)]
    } else {
        0.0
    };
    
    let total_tokens = token_counter.load(Ordering::Relaxed);
    
    LoadTestResult {
        provider: provider_name,
        total_requests: num_requests,
        successful,
        failed,
        total_tokens,
        duration_secs,
        requests_per_second: successful as f64 / duration_secs,
        tokens_per_second: total_tokens as f64 / duration_secs,
        avg_latency_ms: avg_latency,
        p99_latency_ms: p99_latency,
        errors,
    }
}

/// Test 1 million tokens from AWS Bedrock
async fn test_million_tokens(provider: Arc<dyn AiProvider>) -> Result<()> {
    println!("\n{}", "🎯 1 MILLION TOKEN TEST - AWS BEDROCK TITAN".bright_yellow().bold());
    println!("{}", "=".repeat(60).bright_yellow());
    
    let start = Instant::now();
    let mut total_tokens = 0;
    let mut request_count = 0;
    let target_tokens = 1_000_000;
    
    // Use larger requests to reach 1M tokens faster
    while total_tokens < target_tokens {
        request_count += 1;
        
        let request = ChatRequest {
            model: "amazon.titan-text-express-v1".to_string(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: Some("Generate a detailed technical explanation about distributed systems, covering topics like consensus algorithms, replication strategies, fault tolerance, CAP theorem, and practical implementations. Make this response as comprehensive as possible.".to_string()),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                }
            ],
            temperature: Some(0.7),
            max_tokens: Some(2048), // Max tokens per request
            stream: Some(true),
            top_p: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            seed: None,
            logprobs: None,
            top_logprobs: None,
        };
        
        match provider.chat_stream(request).await {
            Ok(mut stream) => {
                let mut request_tokens = 0;
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(token) => {
                            match token {
                                StreamToken::Text(text) => {
                                    request_tokens += text.len() / 4; // Approximate tokens
                                }
                                StreamToken::Delta { content } => {
                                    request_tokens += content.len() / 4; // Approximate tokens
                                }
                                StreamToken::Done => break,
                                StreamToken::Error(e) => {
                                    println!("  ⚠️ Stream error: {}", e.red());
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Request {} failed: {}", request_count, e.to_string().red());
                            break;
                        }
                    }
                }
                
                total_tokens += request_tokens;
                
                // Progress update
                if request_count % 10 == 0 {
                    let progress = (total_tokens as f64 / target_tokens as f64) * 100.0;
                    println!("  📊 Progress: {:.1}% ({} tokens)", progress, total_tokens);
                }
            }
            Err(e) => {
                println!("  ❌ Request {} failed: {}", request_count, e.to_string().red());
            }
        }
        
        // Add small delay to avoid rate limiting
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    let duration = start.elapsed();
    
    println!("\n{}", "📈 1M TOKEN TEST RESULTS".bright_green().bold());
    println!("  • Total tokens: {}", total_tokens.to_string().green());
    println!("  • Total requests: {}", request_count);
    println!("  • Duration: {:.2}s", duration.as_secs_f64());
    println!("  • Tokens/second: {:.0}", total_tokens as f64 / duration.as_secs_f64());
    println!("  • Avg tokens/request: {}", total_tokens / request_count);
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    println!("\n{}", "🔥 1000 CONCURRENT REQUEST TEST + 1M TOKEN VALIDATION".bright_magenta().bold());
    println!("{}", "=".repeat(60).bright_magenta());
    
    let mut providers: Vec<(String, Arc<dyn AiProvider>)> = Vec::new();
    
    // AWS Bedrock Provider (Primary for 1M token test)
    if let (Ok(access_key), Ok(secret_key)) = (
        std::env::var("AWS_ACCESS_KEY_ID"),
        std::env::var("AWS_SECRET_ACCESS_KEY"),
    ) {
        println!("✅ Configuring AWS Bedrock with Titan...");
        let config = BedrockConfig {
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            access_key_id: access_key,
            secret_access_key: secret_key,
            session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
            base_url: None,
            default_model: Some("amazon.titan-text-express-v1".to_string()),
            timeout_ms: Some(60000),
        };
        
        match BedrockProvider::new(config).await {
            Ok(provider) => {
                providers.push(("AWS Bedrock Titan".to_string(), Arc::new(provider) as Arc<dyn AiProvider>));
                println!("  ✅ AWS Bedrock ready");
            }
            Err(e) => println!("  ❌ AWS Bedrock failed: {}", e),
        }
    }
    
    // Gemini Provider (for concurrent test comparison)
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        println!("✅ Configuring Google Gemini...");
        let config = GeminiConfig {
            api_key,
            base_url: Some("https://generativelanguage.googleapis.com".to_string()),
            default_model: Some("gemini-pro".to_string()),
            api_version: Some("v1beta".to_string()),
            timeout_ms: Some(60000),
            project_id: None,
            location: None,
        };
        
        match GeminiProvider::new(config).await {
            Ok(provider) => {
                providers.push(("Google Gemini".to_string(), Arc::new(provider) as Arc<dyn AiProvider>));
                println!("  ✅ Gemini ready");
            }
            Err(e) => println!("  ❌ Gemini failed: {}", e),
        }
    }
    
    if providers.is_empty() {
        println!("{}", "❌ No providers configured!".red().bold());
        return Ok(());
    }
    
    // Phase 1: 1000 Concurrent Requests Test
    println!("\n{}", "PHASE 1: 1000 CONCURRENT REQUESTS".bright_cyan().bold());
    println!("{}", "=".repeat(60).bright_cyan());
    
    let mut results = Vec::new();
    
    for (name, provider) in &providers {
        let result = run_load_test(
            Arc::clone(provider),
            name.clone(),
            1000,  // Total requests
            100,   // Max concurrent
        ).await;
        
        results.push(result);
        
        // Cooldown between providers
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    
    // Display results
    println!("\n{}", "📊 CONCURRENT TEST RESULTS".bright_green().bold());
    println!("{}", "=".repeat(60).bright_green());
    
    for result in &results {
        println!("\n{}", result.provider.bright_cyan().bold());
        println!("  • Total Requests: {}", result.total_requests);
        println!("  • Successful: {} ({}%)", 
            result.successful.to_string().green(),
            (result.successful * 100 / result.total_requests));
        println!("  • Failed: {}", 
            if result.failed > 0 { result.failed.to_string().red() } else { "0".green() });
        println!("  • Duration: {:.2}s", result.duration_secs);
        println!("  • Requests/sec: {:.2}", result.requests_per_second);
        println!("  • Tokens/sec: {:.0}", result.tokens_per_second);
        println!("  • Avg Latency: {:.0}ms", result.avg_latency_ms);
        println!("  • P99 Latency: {:.0}ms", result.p99_latency_ms);
        
        if !result.errors.is_empty() {
            println!("  • First errors:");
            for (i, error) in result.errors.iter().take(3).enumerate() {
                println!("    {}. {}", i + 1, error.red());
            }
        }
    }
    
    // Phase 2: 1 Million Token Test (AWS Bedrock only)
    if let Some((_, bedrock_provider)) = providers.iter().find(|(name, _)| name.contains("Bedrock")) {
        println!("\n{}", "PHASE 2: 1 MILLION TOKEN TEST".bright_yellow().bold());
        println!("{}", "=".repeat(60).bright_yellow());
        
        test_million_tokens(Arc::clone(bedrock_provider)).await?;
    } else {
        println!("\n{}", "⚠️ AWS Bedrock not available for 1M token test".yellow());
    }
    
    println!("\n{}", "✅ ALL TESTS COMPLETE!".bright_green().bold());
    
    Ok(())
}
