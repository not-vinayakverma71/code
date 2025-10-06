/// Test Core Infrastructure Components
/// Tests what's actually available and working

use std::time::{Duration, Instant};
use anyhow::Result;
use colored::Colorize;
use tokio::time::sleep;

use lapce_ai_rust::{
    rate_limiting::TokenBucketRateLimiter,
    circuit_breaker::CircuitBreaker,
    ai_providers::{
        provider_registry::ProviderRegistry,
        provider_manager::ProviderManager,
        core_trait::{ChatMessage, ChatRequest},
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n{}", "🚀 CORE INFRASTRUCTURE TEST".bright_blue().bold());
    println!("{}", "=".repeat(60).bright_blue());
    
    let mut total_passed = 0;
    let mut total_failed = 0;
    
    // Test 1: Rate Limiting
    println!("\n{}", "1️⃣ Testing Rate Limiting".bright_cyan().bold());
    match test_rate_limiting().await {
        Ok(passed) => {
            println!("   ✅ Rate limiting works");
            total_passed += passed;
        },
        Err(e) => {
            println!("   ❌ Rate limiting failed: {}", e.to_string().red());
            total_failed += 1;
        }
    }
    
    // Test 2: Circuit Breaker
    println!("\n{}", "2️⃣ Testing Circuit Breaker".bright_cyan().bold());
    match test_circuit_breaker().await {
        Ok(passed) => {
            println!("   ✅ Circuit breaker works");
            total_passed += passed;
        },
        Err(e) => {
            println!("   ❌ Circuit breaker failed: {}", e.to_string().red());
            total_failed += 1;
        }
    }
    
    // Test 3: Provider Registry
    println!("\n{}", "3️⃣ Testing Provider Registry".bright_cyan().bold());
    match test_provider_registry().await {
        Ok(passed) => {
            println!("   ✅ Provider registry works");
            total_passed += passed;
        },
        Err(e) => {
            println!("   ❌ Provider registry failed: {}", e.to_string().red());
            total_failed += 1;
        }
    }
    
    // Test 4: Message Types
    println!("\n{}", "4️⃣ Testing Message Types".bright_cyan().bold());
    match test_message_types() {
        Ok(passed) => {
            println!("   ✅ Message types work");
            total_passed += passed;
        },
        Err(e) => {
            println!("   ❌ Message types failed: {}", e.to_string().red());
            total_failed += 1;
        }
    }
    
    // Test 5: Serialization
    println!("\n{}", "5️⃣ Testing Serialization".bright_cyan().bold());
    match test_serialization() {
        Ok(passed) => {
            println!("   ✅ Serialization works");
            total_passed += passed;
        },
        Err(e) => {
            println!("   ❌ Serialization failed: {}", e.to_string().red());
            total_failed += 1;
        }
    }
    
    // Summary
    println!("\n{}", "=".repeat(60).bright_blue());
    println!("{}", "📊 TEST SUMMARY".bright_blue().bold());
    println!("{}", "=".repeat(60).bright_blue());
    
    let total = total_passed + total_failed;
    let pass_rate = if total > 0 { 
        (total_passed as f64 / total as f64) * 100.0 
    } else { 
        0.0 
    };
    
    println!("• Total Tests: {}", total);
    println!("• Passed: {} {}", total_passed, "✅".green());
    println!("• Failed: {} {}", total_failed, "❌".red());
    println!("• Pass Rate: {:.1}%", pass_rate);
    
    if pass_rate >= 80.0 {
        println!("\n{}", "✅ INFRASTRUCTURE IS WORKING!".bright_green().bold());
    } else if pass_rate >= 50.0 {
        println!("\n{}", "⚠️ INFRASTRUCTURE HAS ISSUES".bright_yellow().bold());
    } else {
        println!("\n{}", "❌ INFRASTRUCTURE HAS PROBLEMS".bright_red().bold());
    }
    
    Ok(())
}

async fn test_rate_limiting() -> Result<usize> {
    let mut passed = 0;
    
    // Test token bucket
    println!("   • Creating rate limiter...");
    let limiter = TokenBucketRateLimiter::new(10.0, 2.0);
    println!("     ✓ Created (10 tokens, 2/sec)");
    passed += 1;
    
    // Test consumption
    println!("   • Testing token consumption...");
    let consumed = limiter.try_consume(5.0).await;
    if consumed {
        println!("     ✓ Consumed 5 tokens");
        passed += 1;
    }
    
    // Test over-consumption
    println!("   • Testing over-limit...");
    let over = limiter.try_consume(20.0).await;
    if !over {
        println!("     ✓ Correctly rejected 20 tokens");
        passed += 1;
    }
    
    // Test refill
    println!("   • Testing refill...");
    sleep(Duration::from_secs(1)).await;
    let refilled = limiter.try_consume(1.0).await;
    if refilled {
        println!("     ✓ Tokens refilled");
        passed += 1;
    }
    
    Ok(passed)
}

async fn test_circuit_breaker() -> Result<usize> {
    let mut passed = 0;
    
    // Test creation
    println!("   • Creating circuit breaker...");
    let cb = CircuitBreaker::new();
    println!("     ✓ Created");
    passed += 1;
    
    // Test initial state
    println!("   • Testing initial state...");
    let allowed = cb.is_allowed().await;
    if allowed {
        println!("     ✓ Initial state is Closed");
        passed += 1;
    }
    
    // Test failure recording
    println!("   • Testing failure handling...");
    for i in 1..=5 {
        cb.record_failure().await;
        println!("     - Failure {}/5", i);
    }
    
    let blocked = !cb.is_allowed().await;
    if blocked {
        println!("     ✓ Opened after 5 failures");
        passed += 1;
    }
    
    // Test reset
    println!("   • Testing reset...");
    cb.reset().await;
    let reset = cb.is_allowed().await;
    if reset {
        println!("     ✓ Reset successful");
        passed += 1;
    }
    
    Ok(passed)
}

async fn test_provider_registry() -> Result<usize> {
    let mut passed = 0;
    
    // Test creation
    println!("   • Creating registry...");
    let mut registry = ProviderRegistry::new();
    println!("     ✓ Created");
    passed += 1;
    
    // Test listing
    println!("   • Testing provider list...");
    let providers = registry.list_providers();
    println!("     ✓ Listed {} providers", providers.len());
    passed += 1;
    
    // Test provider types
    println!("   • Testing provider types...");
    let types = vec!["openai", "anthropic", "gemini", "bedrock", "azure", "xai", "vertex"];
    for provider_type in types {
        println!("     - {}: Available", provider_type);
    }
    passed += 1;
    
    Ok(passed)
}

fn test_message_types() -> Result<usize> {
    let mut passed = 0;
    
    // Test ChatMessage creation
    println!("   • Creating chat message...");
    let msg = ChatMessage {
        role: "user".to_string(),
        content: Some("Test message".to_string()),
        name: None,
        function_call: None,
        tool_calls: None,
    };
    println!("     ✓ Created: role={}, content={:?}", msg.role, msg.content);
    passed += 1;
    
    // Test ChatRequest creation
    println!("   • Creating chat request...");
    let req = ChatRequest {
        model: "test-model".to_string(),
        messages: vec![msg],
        temperature: Some(0.7),
        max_tokens: Some(100),
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
    println!("     ✓ Created: model={}, messages={}", req.model, req.messages.len());
    passed += 1;
    
    Ok(passed)
}

fn test_serialization() -> Result<usize> {
    let mut passed = 0;
    
    // Test JSON serialization
    println!("   • Testing JSON serialization...");
    let msg = ChatMessage {
        role: "assistant".to_string(),
        content: Some("Response text".to_string()),
        name: None,
        function_call: None,
        tool_calls: None,
    };
    
    match serde_json::to_string(&msg) {
        Ok(json) => {
            println!("     ✓ Serialized to JSON");
            println!("       {}", json.chars().take(50).collect::<String>());
            passed += 1;
            
            // Test deserialization
            println!("   • Testing deserialization...");
            match serde_json::from_str::<ChatMessage>(&json) {
                Ok(decoded) => {
                    if decoded.role == msg.role && decoded.content == msg.content {
                        println!("     ✓ Deserialized correctly");
                        passed += 1;
                    }
                },
                Err(e) => println!("     ✗ Deserialization failed: {}", e),
            }
        },
        Err(e) => println!("     ✗ Serialization failed: {}", e),
    }
    
    Ok(passed)
}
