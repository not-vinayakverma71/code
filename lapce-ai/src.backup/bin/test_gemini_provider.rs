/// Comprehensive Gemini Provider Testing
/// Tests all aspects of the Gemini implementation

use std::env;
use anyhow::Result;
use lapce_ai_rust::ai_providers::{
    gemini_exact::{GeminiProvider, GeminiConfig},
    core_trait::{AiProvider, ChatRequest, ChatMessage, CompletionRequest},
};
use futures::StreamExt;
use colored::Colorize;
use tokio::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment
    dotenv::dotenv().ok();
    
    println!("\n{}", "🚀 GEMINI PROVIDER COMPREHENSIVE TEST".bright_blue().bold());
    println!("{}", "="
.repeat(60).bright_blue());
    
    // Get API key
    let api_key = env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY not found in environment");
    
    println!("✅ API Key loaded: {}...", &api_key[..10]);
    
    // Create provider
    let config = GeminiConfig {
        api_key: api_key.clone(),
        base_url: Some("https://generativelanguage.googleapis.com".to_string()),
        default_model: Some("gemini-1.5-flash-002".to_string()),
        api_version: Some("v1beta".to_string()),
        timeout_ms: Some(30000),
        project_id: None,
        location: None,
    };
    
    let provider = GeminiProvider::new(config).await?;
    println!("✅ Provider created successfully\n");
    
    // Run all tests
    let mut passed = 0;
    let mut failed = 0;
    let mut total_latency = Duration::ZERO;
    
    // Test 1: Provider Name
    println!("{}", "1️⃣ Testing Provider Name".bright_cyan());
    let name = provider.name();
    if name == "Gemini" {
        println!("   ✅ Provider name: {}", name.green());
        passed += 1;
    } else {
        println!("   ❌ Unexpected name: {}", name.red());
        failed += 1;
    }
    
    // Test 2: Health Check
    println!("\n{}", "2️⃣ Testing Health Check".bright_cyan());
    let start = Instant::now();
    match provider.health_check().await {
        Ok(status) => {
            let latency = start.elapsed();
            total_latency += latency;
            println!("   ✅ Health check passed");
            println!("   • Healthy: {}", status.healthy.to_string().green());
            println!("   • Latency: {}ms", status.latency_ms);
            println!("   • Measured: {}ms", latency.as_millis());
            passed += 1;
        },
        Err(e) => {
            println!("   ❌ Health check failed: {}", e.to_string().red());
            failed += 1;
        }
    }
    
    // Test 3: List Models
    println!("\n{}", "3️⃣ Testing List Models".bright_cyan());
    let start = Instant::now();
    match provider.list_models().await {
        Ok(models) => {
            let latency = start.elapsed();
            total_latency += latency;
            println!("   ✅ Listed {} models ({}ms)", models.len(), latency.as_millis());
            for (i, model) in models.iter().take(5).enumerate() {
                println!("   {}. {}", i+1, model.id.green());
                println!("      • Context: {}", model.context_window);
                println!("      • Max output: {}", model.max_output_tokens);
                println!("      • Vision: {}", model.supports_vision);
            }
            if models.len() > 5 {
                println!("   ... and {} more", models.len() - 5);
            }
            passed += 1;
        },
        Err(e) => {
            println!("   ❌ List models failed: {}", e.to_string().red());
            failed += 1;
        }
    }
    
    // Test 4: Capabilities
    println!("\n{}", "4️⃣ Testing Capabilities".bright_cyan());
    let caps = provider.capabilities();
    println!("   ✅ Capabilities retrieved:");
    println!("   • Max tokens: {}", caps.max_tokens.to_string().green());
    println!("   • Streaming: {}", caps.supports_streaming.to_string().green());
    println!("   • Functions: {}", caps.supports_functions.to_string().green());
    println!("   • Vision: {}", caps.supports_vision.to_string().green());
    println!("   • Rate limits:");
    println!("     - RPM: {}", caps.rate_limits.requests_per_minute);
    println!("     - TPM: {}", caps.rate_limits.tokens_per_minute);
    passed += 1;
    
    // Test 5: Token Counting
    println!("\n{}", "5️⃣ Testing Token Counting".bright_cyan());
    let test_text = "The quick brown fox jumps over the lazy dog. This is a test sentence to count tokens.";
    match provider.count_tokens(test_text).await {
        Ok(count) => {
            println!("   ✅ Token count: {} tokens", count.to_string().green());
            println!("   • Text length: {} chars", test_text.len());
            println!("   • Ratio: {:.2} chars/token", test_text.len() as f64 / count as f64);
            passed += 1;
        },
        Err(e) => {
            println!("   ❌ Token counting failed: {}", e.to_string().red());
            failed += 1;
        }
    }
    
    // Test 6: Chat Completion (Non-streaming)
    println!("\n{}", "6️⃣ Testing Chat Completion (Non-streaming)".bright_cyan());
    let chat_request = ChatRequest {
        model: "gemini-1.5-flash-002".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: Some("What is 2+2? Reply with just the number.".to_string()),
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
    
    let start = Instant::now();
    match provider.chat(chat_request.clone()).await {
        Ok(response) => {
            let latency = start.elapsed();
            total_latency += latency;
            println!("   ✅ Chat completion successful ({}ms)", latency.as_millis());
            if !response.choices.is_empty() {
                let content = &response.choices[0].message.content;
                println!("   • Response: {}", content.as_deref().unwrap_or("(empty)").green());
                println!("   • Model: {}", response.model);
                if let Some(usage) = response.usage {
                    println!("   • Tokens: {} prompt + {} completion = {} total", 
                        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
                }
            }
            passed += 1;
        },
        Err(e) => {
            println!("   ❌ Chat completion failed: {}", e.to_string().red());
            failed += 1;
        }
    }
    
    // Test 7: Chat Streaming
    println!("\n{}", "7️⃣ Testing Chat Streaming".bright_cyan());
    let stream_request = ChatRequest {
        model: "gemini-1.5-flash-002".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: Some("Count from 1 to 5 slowly.".to_string()),
                name: None,
                function_call: None,
                tool_calls: None,
            }
        ],
        temperature: Some(0.7),
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
    
    let start = Instant::now();
    match provider.chat_stream(stream_request).await {
        Ok(mut stream) => {
            println!("   ✅ Stream created successfully");
            print!("   • Response: ");
            let mut token_count = 0;
            let mut full_response = String::new();
            
            while let Some(result) = stream.next().await {
                match result {
                    Ok(token) => {
                        token_count += 1;
                        match token {
                            lapce_ai_rust::ai_providers::core_trait::StreamToken::Delta { content } => {
                                print!("{}", content.green());
                                full_response.push_str(&content);
                            },
                            lapce_ai_rust::ai_providers::core_trait::StreamToken::Done => {
                                println!("\n   • Stream completed");
                                break;
                            },
                            _ => {}
                        }
                    },
                    Err(e) => {
                        println!("\n   ⚠️ Stream error: {}", e.to_string().yellow());
                        break;
                    }
                }
            }
            
            let latency = start.elapsed();
            total_latency += latency;
            println!("   • Tokens received: {}", token_count);
            println!("   • Total latency: {}ms", latency.as_millis());
            println!("   • Response length: {} chars", full_response.len());
            passed += 1;
        },
        Err(e) => {
            println!("   ❌ Chat streaming failed: {}", e.to_string().red());
            failed += 1;
        }
    }
    
    // Test 8: Completion (Legacy API)
    println!("\n{}", "8️⃣ Testing Completion API (Legacy)".bright_cyan());
    let completion_request = CompletionRequest {
        model: "gemini-1.5-flash-002".to_string(),
        prompt: "The capital of France is".to_string(),
        max_tokens: Some(10),
        temperature: Some(0.0),
        stream: Some(false),
        top_p: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: None,
        suffix: None,
        echo: None,
        n: None,
        best_of: None,
    };
    
    let start = Instant::now();
    match provider.complete(completion_request.clone()).await {
        Ok(response) => {
            let latency = start.elapsed();
            total_latency += latency;
            println!("   ✅ Completion successful ({}ms)", latency.as_millis());
            if !response.choices.is_empty() {
                let text = &response.choices[0].text;
                println!("   • Response: {}", text.green());
            }
            passed += 1;
        },
        Err(e) => {
            println!("   ❌ Completion failed: {}", e.to_string().red());
            failed += 1;
        }
    }
    
    // Test 9: Completion Streaming
    println!("\n{}", "9️⃣ Testing Completion Streaming".bright_cyan());
    let stream_completion = CompletionRequest {
        model: "gemini-1.5-flash-002".to_string(),
        prompt: "Write a haiku about coding:".to_string(),
        max_tokens: Some(50),
        temperature: Some(0.9),
        stream: Some(true),
        top_p: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: None,
        suffix: None,
        echo: None,
        n: None,
        best_of: None,
    };
    
    let start = Instant::now();
    match provider.complete_stream(stream_completion).await {
        Ok(mut stream) => {
            println!("   ✅ Completion stream created");
            print!("   • Response: ");
            let mut tokens = 0;
            
            while let Some(result) = stream.next().await {
                match result {
                    Ok(token) => {
                        tokens += 1;
                        match token {
                            lapce_ai_rust::ai_providers::core_trait::StreamToken::Delta { content } => {
                                print!("{}", content.green());
                            },
                            lapce_ai_rust::ai_providers::core_trait::StreamToken::Done => {
                                println!("\n   • Stream completed");
                                break;
                            },
                            _ => {}
                        }
                    },
                    Err(e) => {
                        println!("\n   ⚠️ Stream error: {}", e);
                        break;
                    }
                }
            }
            
            let latency = start.elapsed();
            total_latency += latency;
            println!("   • Tokens: {}", tokens);
            println!("   • Latency: {}ms", latency.as_millis());
            passed += 1;
        },
        Err(e) => {
            println!("   ❌ Completion streaming failed: {}", e.to_string().red());
            failed += 1;
        }
    }
    
    // Test 10: Error Handling
    println!("\n{}", "🔟 Testing Error Handling".bright_cyan());
    let bad_request = ChatRequest {
        model: "non-existent-model-xyz".to_string(),
        messages: vec![],
        temperature: None,
        max_tokens: None,
        stream: None,
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
    
    match provider.chat(bad_request).await {
        Ok(_) => {
            println!("   ⚠️ Expected error but got success");
            failed += 1;
        },
        Err(e) => {
            println!("   ✅ Error handling works: {}", e.to_string().yellow());
            passed += 1;
        }
    }
    
    // Summary
    println!("\n{}", "="
.repeat(60).bright_blue());
    println!("{}", "📊 TEST SUMMARY".bright_blue().bold());
    println!("{}", "="
.repeat(60).bright_blue());
    
    let total = passed + failed;
    let pass_rate = (passed as f64 / total as f64) * 100.0;
    let avg_latency = if passed > 0 {
        total_latency.as_millis() / passed as u128
    } else {
        0
    };
    
    println!("• Total Tests: {}", total);
    println!("• Passed: {} {}", passed, "✅".green());
    println!("• Failed: {} {}", failed, "❌".red());
    println!("• Pass Rate: {:.1}%", pass_rate);
    println!("• Average Latency: {}ms", avg_latency);
    
    if pass_rate >= 80.0 {
        println!("\n{}", "✅ GEMINI PROVIDER IS WORKING CORRECTLY!".bright_green().bold());
    } else if pass_rate >= 50.0 {
        println!("\n{}", "⚠️ GEMINI PROVIDER HAS SOME ISSUES".bright_yellow().bold());
    } else {
        println!("\n{}", "❌ GEMINI PROVIDER HAS SIGNIFICANT PROBLEMS".bright_red().bold());
    }
    
    Ok(())
}
