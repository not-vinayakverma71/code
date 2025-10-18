/// Simple Gemini API Test with your API key

#[tokio::test]
async fn test_gemini_api() {
    use lapce_ai_rust::ai_providers::gemini::{GeminiProvider, GeminiConfig};
    use lapce_ai_rust::ai_providers::{AiProvider, CompletionRequest, Message};
    
    let api_key = "AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU";
    
    println!("\n=== GEMINI API VALIDATION TEST ===");
    println!("API Key: {}...", &api_key[..20]);
    
    let config = GeminiConfig {
        api_key: api_key.to_string(),
        default_model: Some("gemini-1.5-flash".to_string()),
        ..Default::default()
    };
    
    let provider = GeminiProvider::new(config).await.unwrap();
    println!("✓ Provider created");
    
    let request = CompletionRequest {
        model: "gemini-1.5-flash".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Reply with exactly: Hello World".to_string(),
                tool_calls: None,
            }
        ],
        temperature: Some(0.1),
        max_tokens: Some(10),
        ..Default::default()
    };
    
    println!("✓ Request prepared");
    println!("  Model: {}", request.model);
    println!("  Message: {}", request.messages[0].content);
    
    let start = std::time::Instant::now();
    
    match provider.complete(request).await {
        Ok(response) => {
            let latency = start.elapsed().as_millis();
            
            println!("\n✅ SUCCESS!");
            println!("  Response: {}", response.content);
            println!("  Latency: {}ms", latency);
            
            if let Some(usage) = response.usage {
                println!("  Input tokens: {}", usage.input_tokens);
                println!("  Output tokens: {}", usage.output_tokens);
                println!("  Total tokens: {}", usage.total_tokens);
            }
            
            // Performance validation
            assert!(!response.content.is_empty(), "Response should not be empty");
            assert!(latency < 5000, "Latency should be under 5 seconds");
            
            // Memory check
            let mem_kb = get_memory_usage();
            let mem_mb = mem_kb as f64 / 1024.0;
            println!("\n  Memory usage: {:.2} MB (Requirement: < 8MB)", mem_mb);
            assert!(mem_mb < 8.0, "Memory usage exceeds 8MB limit");
            
            println!("\n🎉 100% VALIDATED - ALL TESTS PASSED!");
        }
        Err(e) => {
            println!("\n❌ FAILED!");
            println!("  Error: {}", e);
            panic!("Gemini API test failed: {}", e);
        }
    }
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
