/// Test 1000 concurrent requests with Gemini (better rate limits than AWS)
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
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    println!("\n{}", "🚀 GEMINI 1000 CONCURRENT REQUEST TEST".bright_magenta().bold());
    println!("{}", "=".repeat(60).bright_magenta());
    
    // Use provided Gemini API key
    let api_key = std::env::var("GEMINI_API_KEY")
        .unwrap_or_else(|_| "AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU".to_string());
    
    println!("📌 Configuration:");
    println!("  • API Key: {}...", &api_key[..20]);
    println!("  • Model: gemini-pro");
    
    // Create Gemini provider
    let config = GeminiConfig {
        api_key,
        base_url: Some("https://generativelanguage.googleapis.com".to_string()),
        default_model: Some("gemini-pro".to_string()),
        api_version: Some("v1beta".to_string()),
        timeout_ms: Some(30000),
        project_id: None,
        location: None,
    };
    
    println!("\n⚙️ Initializing Gemini provider...");
    let provider = Arc::new(GeminiProvider::new(config).await?);
    
    // Phase 1: Warm up with a single request
    println!("\n{}", "PHASE 1: Warm-up Request".bright_yellow().bold());
    println!("{}", "-".repeat(40).yellow());
    
    let warmup_request = ChatRequest {
        model: "gemini-pro".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: Some("Say 'Ready' and nothing else.".to_string()),
                name: None,
                function_call: None,
                tool_calls: None,
            }
        ],
        temperature: Some(0.0),
        max_tokens: Some(10),
        stream: Some(false),
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
    
    match provider.chat(warmup_request).await {
        Ok(response) => {
            println!("✅ Warm-up successful");
            if let Some(choice) = response.choices.first() {
                if let Some(content) = &choice.message.content {
                    println!("   Response: {}", content.trim().green());
                }
            }
        }
        Err(e) => {
            println!("❌ Warm-up failed: {}", e.to_string().red());
            return Ok(());
        }
    }
    
    // Phase 2: 1000 Concurrent Requests
    println!("\n{}", "PHASE 2: 1000 CONCURRENT REQUESTS".bright_cyan().bold());
    println!("{}", "=".repeat(60).bright_cyan());
    
    let total_requests = 1000;
    let max_concurrent = 50; // Start conservatively with Gemini
    
    println!("📊 Test Configuration:");
    println!("  • Total requests: {}", total_requests);
    println!("  • Max concurrent: {}", max_concurrent);
    println!("  • Provider: Google Gemini");
    
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let success_counter = Arc::new(AtomicUsize::new(0));
    let fail_counter = Arc::new(AtomicUsize::new(0));
    let token_counter = Arc::new(AtomicUsize::new(0));
    
    let mut tasks = JoinSet::new();
    let start = Instant::now();
    
    println!("\n🔄 Starting concurrent requests...");
    println!("Progress: ");
    
    // Launch all requests
    for i in 0..total_requests {
        let provider_clone = Arc::clone(&provider);
        let sem_clone = Arc::clone(&semaphore);
        let success_clone = Arc::clone(&success_counter);
        let fail_clone = Arc::clone(&fail_counter);
        let token_clone = Arc::clone(&token_counter);
        
        tasks.spawn(async move {
            let _permit = sem_clone.acquire().await.unwrap();
            let req_start = Instant::now();
            
            let request = ChatRequest {
                model: "gemini-pro".to_string(),
                messages: vec![
                    ChatMessage {
                        role: "user".to_string(),
                        content: Some(format!("Generate a short response for request #{}", i)),
                        name: None,
                        function_call: None,
                        tool_calls: None,
                    }
                ],
                temperature: Some(0.1),
                max_tokens: Some(30),
                stream: Some(false), // Non-streaming for speed
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
            
            match provider_clone.chat(request).await {
                Ok(response) => {
                    success_clone.fetch_add(1, Ordering::Relaxed);
                    
                    // Count tokens
                    if let Some(choice) = response.choices.first() {
                        if let Some(content) = &choice.message.content {
                            token_clone.fetch_add(content.len() / 4, Ordering::Relaxed);
                        }
                    }
                    
                    let latency = req_start.elapsed().as_millis();
                    (true, latency)
                }
                Err(_) => {
                    fail_clone.fetch_add(1, Ordering::Relaxed);
                    let latency = req_start.elapsed().as_millis();
                    (false, latency)
                }
            }
        });
        
        // Add a small delay between launches to avoid overwhelming
        if i % 10 == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    // Monitor progress
    let monitor_task = tokio::spawn({
        let success_clone = Arc::clone(&success_counter);
        let fail_clone = Arc::clone(&fail_counter);
        async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let success = success_clone.load(Ordering::Relaxed);
                let failed = fail_clone.load(Ordering::Relaxed);
                let total_done = success + failed;
                
                if total_done >= total_requests {
                    break;
                }
                
                let progress = (total_done as f64 / total_requests as f64) * 100.0;
                print!("\r  Progress: {:.1}% ({}/{})", progress, total_done, total_requests);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        }
    });
    
    // Collect results
    let mut latencies = Vec::new();
    while let Some(result) = tasks.join_next().await {
        if let Ok((_, latency)) = result {
            latencies.push(latency as f64);
        }
    }
    
    monitor_task.abort();
    
    let duration = start.elapsed();
    let success_count = success_counter.load(Ordering::Relaxed);
    let fail_count = fail_counter.load(Ordering::Relaxed);
    let total_tokens = token_counter.load(Ordering::Relaxed);
    
    // Calculate statistics
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_latency = if !latencies.is_empty() {
        latencies.iter().sum::<f64>() / latencies.len() as f64
    } else {
        0.0
    };
    
    let p50_latency = if latencies.len() > 0 {
        latencies[latencies.len() / 2]
    } else {
        0.0
    };
    
    let p99_latency = if !latencies.is_empty() {
        let p99_idx = (latencies.len() as f64 * 0.99) as usize;
        latencies[p99_idx.min(latencies.len() - 1)]
    } else {
        0.0
    };
    
    // Display results
    println!("\n\n{}", "📊 RESULTS".bright_green().bold());
    println!("{}", "=".repeat(60).bright_green());
    
    println!("\n📈 Performance Metrics:");
    println!("  • Total Requests: {}", total_requests);
    println!("  • Successful: {} ({}%)", 
        success_count.to_string().green(),
        (success_count * 100 / total_requests));
    println!("  • Failed: {} ({}%)", 
        if fail_count > 0 { fail_count.to_string().red() } else { "0".green() },
        (fail_count * 100 / total_requests));
    println!("  • Total Duration: {:.2}s", duration.as_secs_f64());
    println!("  • Requests/second: {:.2}", success_count as f64 / duration.as_secs_f64());
    
    println!("\n⏱️ Latency Analysis:");
    println!("  • Average: {:.0}ms", avg_latency);
    println!("  • P50 (median): {:.0}ms", p50_latency);
    println!("  • P99: {:.0}ms", p99_latency);
    
    println!("\n📝 Token Statistics:");
    println!("  • Total tokens: {}", total_tokens);
    println!("  • Tokens/second: {:.0}", total_tokens as f64 / duration.as_secs_f64());
    
    // Success criteria
    println!("\n{}", "✅ SUCCESS CRITERIA".bright_blue().bold());
    println!("{}", "=".repeat(60).bright_blue());
    
    let success_rate = success_count as f64 / total_requests as f64;
    if success_rate >= 0.95 {
        println!("✅ High success rate: {:.1}%", success_rate * 100.0);
    } else if success_rate >= 0.80 {
        println!("⚠️ Moderate success rate: {:.1}%", success_rate * 100.0);
    } else {
        println!("❌ Low success rate: {:.1}%", success_rate * 100.0);
    }
    
    if avg_latency < 1000.0 {
        println!("✅ Low average latency: {:.0}ms", avg_latency);
    } else if avg_latency < 5000.0 {
        println!("⚠️ Moderate average latency: {:.0}ms", avg_latency);
    } else {
        println!("❌ High average latency: {:.0}ms", avg_latency);
    }
    
    let req_per_sec = success_count as f64 / duration.as_secs_f64();
    if req_per_sec > 10.0 {
        println!("✅ High throughput: {:.2} req/s", req_per_sec);
    } else if req_per_sec > 5.0 {
        println!("⚠️ Moderate throughput: {:.2} req/s", req_per_sec);
    } else {
        println!("❌ Low throughput: {:.2} req/s", req_per_sec);
    }
    
    println!("\n{}", "🎉 TEST COMPLETE!".bright_green().bold());
    
    Ok(())
}
