/// GEMINI PROVIDER ULTIMATE VALIDATION TEST
/// Testing with real API key to validate 100% functionality

use lapce_ai_rust::ai_providers::{gemini::GeminiProvider, gemini::GeminiConfig, AiProvider, CompletionRequest, Message};
use futures::StreamExt;
use std::time::Instant;
use anyhow::Result;

const API_KEY: &str = "AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU";

#[tokio::test]
async fn test_gemini_basic() -> Result<()> {
    println!("\n=== GEMINI BASIC TEST ===");
    
    let config = GeminiConfig {
        api_key: API_KEY.to_string(),
        default_model: Some("gemini-1.5-flash".to_string()),
        ..Default::default()
    };
    
    let provider = GeminiProvider::new(config).await?;
    
    let request = CompletionRequest {
        model: "gemini-1.5-flash".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Say 'Hello World' and nothing else.".to_string(),
                tool_calls: None,
            }
        ],
        temperature: Some(0.1),
        max_tokens: Some(10),
        ..Default::default()
    };
    
    let start = Instant::now();
    let response = provider.complete(request).await?;
    let latency = start.elapsed().as_millis();
    
    println!("Response: {}", response.content);
    println!("Latency: {}ms", latency);
    
    assert!(!response.content.is_empty());
    assert!(latency < 5000, "Latency should be under 5 seconds");
    
    println!("✅ PASSED");
    Ok(())
}

#[tokio::test]
async fn test_gemini_streaming() -> Result<()> {
    println!("\n=== GEMINI STREAMING TEST ===");
    
    let config = GeminiConfig {
        api_key: API_KEY.to_string(),
        ..Default::default()
    };
    
    let provider = GeminiProvider::new(config).await?;
    
    let request = CompletionRequest {
        model: "gemini-1.5-flash".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Count from 1 to 3".to_string(),
                tool_calls: None,
            }
        ],
        stream: Some(true),
        temperature: Some(0.1),
        max_tokens: Some(50),
        ..Default::default()
    };
    
    let start = Instant::now();
    let mut stream = provider.stream(request).await?;
    
    let mut full_response = String::new();
    let mut first_token_ms = 0u128;
    let mut token_count = 0;
    
    while let Some(token_result) = stream.next().await {
        match token_result? {
            lapce_ai_rust::ai_providers::StreamToken::Text(text) => {
                if token_count == 0 {
                    first_token_ms = start.elapsed().as_millis();
                }
                print!("{}", text);
                full_response.push_str(&text);
                token_count += 1;
            }
            lapce_ai_rust::ai_providers::StreamToken::Done => {
                println!("\n[Stream ended]");
                break;
            }
            _ => {}
        }
    }
    
    let total_ms = start.elapsed().as_millis();
    
    println!("First token: {}ms", first_token_ms);
    println!("Total time: {}ms", total_ms);
    println!("Tokens: {}", token_count);
    
    assert!(!full_response.is_empty());
    assert!(first_token_ms < 2000);
    
    println!("✅ PASSED");
    Ok(())
}

#[tokio::test]
async fn test_performance_requirements() -> Result<()> {
    println!("\n=== PERFORMANCE REQUIREMENTS TEST ===");
    
    // Get memory before
    let mem_before = get_memory_usage();
    
    let config = GeminiConfig {
        api_key: API_KEY.to_string(),
        ..Default::default()
    };
    
    let provider = GeminiProvider::new(config).await?;
    
    // Test dispatch overhead
    let start = Instant::now();
    let _ = provider.get_capabilities();
    let dispatch_ms = start.elapsed().as_millis();
    
    // Make a request
    let request = CompletionRequest {
        model: "gemini-1.5-flash".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hi".to_string(),
                tool_calls: None,
            }
        ],
        max_tokens: Some(5),
        ..Default::default()
    };
    
    let _ = provider.complete(request).await?;
    
    // Get memory after
    let mem_after = get_memory_usage();
    let mem_delta_mb = (mem_after as f64 - mem_before as f64) / 1024.0;
    
    println!("Memory usage: {:.2} MB (Requirement: < 8MB)", mem_delta_mb);
    println!("Dispatch overhead: {}ms (Requirement: < 5ms)", dispatch_ms);
    
    assert!(mem_delta_mb < 8.0, "Memory exceeds 8MB");
    assert!(dispatch_ms < 5, "Dispatch exceeds 5ms");
    
    println!("✅ ALL REQUIREMENTS MET");
    Ok(())
}

fn get_memory_usage() -> usize {
    use std::fs;
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            return line.split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }
    0
}
